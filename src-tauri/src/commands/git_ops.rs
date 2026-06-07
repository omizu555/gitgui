use std::process::Command;
use std::process::Stdio;
use std::io::Read;
use tauri::{Emitter, Manager};
use crate::commands::locks::{RepoLocks, RunningOps};
use crate::models::{RepoStatus, FileStatus, CommitInfo, CommitDetail, GraphCommit, GraphLine, StashEntry, DiffResult, DiffHunk, DiffLine};

/// Windows でコンソールウィンドウを表示せずにプロセスを起動するためのフラグを設定する。
/// macOS / Linux ではなにもしない。
pub(crate) fn hide_console_window(cmd: &mut Command) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
}

/// git status 取得
#[tauri::command]
pub async fn git_status(path: String) -> Result<RepoStatus, String> {
    // 数万ファイルのリポジトリでは statuses() が秒単位かかるためワーカースレッドで実行
    run_blocking(move || git_status_blocking(&path)).await
}

fn git_status_blocking(path: &str) -> Result<RepoStatus, String> {
    let repo = git2::Repository::open(path)
        .map_err(|e| format!("リポジトリを開けません: {}", e))?;

    let branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(|s| s.to_string()))
        .unwrap_or_else(|| "HEAD".to_string());

    // ahead / behind
    let (ahead, behind) = get_ahead_behind(&repo).unwrap_or((0, 0));

    // ステータス取得
    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true);

    let statuses = repo
        .statuses(Some(&mut opts))
        .map_err(|e| format!("ステータス取得失敗: {}", e))?;

    let mut staged = Vec::new();
    let mut unstaged = Vec::new();

    for entry in statuses.iter() {
        let file_path = entry.path().unwrap_or("").to_string();
        let s = entry.status();

        // Staged
        if s.intersects(
            git2::Status::INDEX_NEW
                | git2::Status::INDEX_MODIFIED
                | git2::Status::INDEX_DELETED
                | git2::Status::INDEX_RENAMED,
        ) {
            let status_char = if s.contains(git2::Status::INDEX_NEW) {
                "A"
            } else if s.contains(git2::Status::INDEX_DELETED) {
                "D"
            } else if s.contains(git2::Status::INDEX_RENAMED) {
                "R"
            } else {
                "M"
            };
            staged.push(FileStatus {
                path: file_path.clone(),
                status: status_char.to_string(),
            });
        }

        // Unstaged (working tree)
        if s.intersects(
            git2::Status::WT_MODIFIED
                | git2::Status::WT_DELETED
                | git2::Status::WT_NEW
                | git2::Status::WT_RENAMED,
        ) {
            let status_char = if s.contains(git2::Status::WT_NEW) {
                "A"
            } else if s.contains(git2::Status::WT_DELETED) {
                "D"
            } else if s.contains(git2::Status::WT_RENAMED) {
                "R"
            } else {
                "M"
            };
            unstaged.push(FileStatus {
                path: file_path,
                status: status_char.to_string(),
            });
        }
    }

    Ok(RepoStatus {
        branch,
        ahead,
        behind,
        staged,
        unstaged,
    })
}

/// ahead / behind 計算
fn get_ahead_behind(repo: &git2::Repository) -> Result<(usize, usize), git2::Error> {
    let head = repo.head()?;
    let local_oid = head.target().ok_or_else(|| {
        git2::Error::from_str("HEAD target not found")
    })?;

    let branch_name = head.shorthand().unwrap_or("");
    let upstream_name = format!("refs/remotes/origin/{}", branch_name);

    let upstream_ref = match repo.find_reference(&upstream_name) {
        Ok(r) => r,
        Err(_) => return Ok((0, 0)),
    };

    let upstream_oid = upstream_ref.target().ok_or_else(|| {
        git2::Error::from_str("upstream target not found")
    })?;

    repo.graph_ahead_behind(local_oid, upstream_oid)
}

/// git clone (CLI)
/// dest は保存先の「親フォルダ」。git が URL から導出したサブフォルダにクローンする。
#[tauri::command]
pub async fn git_clone(
    url: String,
    dest: String,
    shallow: Option<bool>,
    op_id: Option<String>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    run_blocking(move || {
        clone_blocking(&url, &dest, shallow.unwrap_or(false), op_id.as_deref(), &app)
    })
    .await
}

fn clone_blocking(
    url: &str,
    dest: &str,
    shallow: bool,
    op_id: Option<&str>,
    app: &tauri::AppHandle,
) -> Result<String, String> {
    // dest フォルダの存在確認
    let dest_path = std::path::Path::new(dest);
    if !dest_path.exists() {
        std::fs::create_dir_all(dest_path)
            .map_err(|e| format!("保存先フォルダの作成に失敗: {}", e))?;
    }

    // クローン先のフルパス（キャンセル時のクリーンアップ判定にも使う）
    let repo_name = url
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("repo")
        .trim_end_matches(".git");
    let cloned_path = dest_path.join(repo_name);
    let existed_before = cloned_path.exists();

    let mut cmd = Command::new("git");
    cmd.args(["clone", "--progress"]);
    // core.longpaths: Windows の 260 文字パス制限対策。
    // clone の -c は新規リポジトリの設定に書き込まれるため、以後の checkout でも有効
    // （Windows 以外では無視され無害）
    cmd.args(["-c", "core.longpaths=true"]);
    if shallow {
        // 履歴を浅く取得。--no-single-branch で全ブランチの先端は取得し、
        // クローン後のブランチ切り替えを可能に保つ
        cmd.args(["--depth", "1", "--no-single-branch"]);
    }
    cmd.arg(url).current_dir(dest);
    // 認証プロンプトを出せない環境で無限ハングせず即エラーにする
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    hide_console_window(&mut cmd);
    let (success, _stdout, stderr) = run_git_command_with_progress(&mut cmd, Some(app), "clone", op_id)?;

    if was_cancelled(app, op_id) {
        // 部分的にクローンされたフォルダをベストエフォートで削除
        if !existed_before {
            remove_dir_all_force(&cloned_path);
        }
        return Err("クローンをキャンセルしました".to_string());
    }

    if success {
        Ok(cloned_path.to_string_lossy().to_string())
    } else {
        Err(format!("Clone 失敗: {}", stderr))
    }
}

/// git fetch (CLI)
/// silent=true は自動フェッチ用: 他の操作が実行中ならロック待ちせず "BUSY" で即返す
#[tauri::command]
pub async fn git_fetch(
    path: String,
    silent: Option<bool>,
    op_id: Option<String>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let is_silent = silent.unwrap_or(false);
    let lock = app.state::<RepoLocks>().lock_for(&path);
    let _guard = if is_silent {
        match lock.try_lock_owned() {
            Ok(g) => g,
            Err(_) => return Err("BUSY".to_string()),
        }
    } else {
        lock.lock_owned().await
    };

    run_blocking(move || {
        let mut cmd = Command::new("git");
        cmd.args(["fetch", "--all", "--progress"]).current_dir(&path);
        cmd.env("GIT_TERMINAL_PROMPT", "0");
        hide_console_window(&mut cmd);
        // 自動フェッチ (silent) は進捗イベントを出さない
        // （ユーザー操作中のローディング表示に別リポジトリの進捗が混ざるのを防ぐ）
        let progress_app = if is_silent { None } else { Some(&app) };
        let (success, _stdout, stderr) =
            run_git_command_with_progress(&mut cmd, progress_app, "fetch", op_id.as_deref())?;

        if was_cancelled(&app, op_id.as_deref()) {
            return Err("フェッチをキャンセルしました".to_string());
        }
        if success {
            Ok("Fetch 完了".to_string())
        } else {
            Err(format!("Fetch 失敗: {}", stderr))
        }
    })
    .await
}

/// git pull (CLI)
#[tauri::command]
pub async fn git_pull(
    path: String,
    op_id: Option<String>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let lock = app.state::<RepoLocks>().lock_for(&path);
    let _guard = lock.lock_owned().await;

    run_blocking(move || {
        let mut cmd = Command::new("git");
        cmd.args(["pull", "--progress"]).current_dir(&path);
        cmd.env("GIT_TERMINAL_PROMPT", "0");
        hide_console_window(&mut cmd);
        let (success, stdout, stderr) =
            run_git_command_with_progress(&mut cmd, Some(&app), "pull", op_id.as_deref())?;

        if was_cancelled(&app, op_id.as_deref()) {
            return Err("Pull をキャンセルしました".to_string());
        }
        if success {
            if stdout.contains("CONFLICT") || stderr.contains("CONFLICT") {
                Ok("Pull 完了（コンフリクトあり）".to_string())
            } else {
                Ok("Pull 完了".to_string())
            }
        } else {
            Err(format!("Pull 失敗: {}", stderr))
        }
    })
    .await
}

/// git push (CLI)
#[tauri::command]
pub async fn git_push(
    path: String,
    op_id: Option<String>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let lock = app.state::<RepoLocks>().lock_for(&path);
    let _guard = lock.lock_owned().await;

    run_blocking(move || {
        let mut cmd = Command::new("git");
        cmd.args(["push", "--progress"]).current_dir(&path);
        cmd.env("GIT_TERMINAL_PROMPT", "0");
        hide_console_window(&mut cmd);
        let (success, _stdout, stderr) =
            run_git_command_with_progress(&mut cmd, Some(&app), "push", op_id.as_deref())?;

        if was_cancelled(&app, op_id.as_deref()) {
            return Err("Push をキャンセルしました".to_string());
        }
        if success {
            Ok("Push 完了".to_string())
        } else {
            Err(format!("Push 失敗: {}", stderr))
        }
    })
    .await
}

/// ファイルをステージング
#[tauri::command]
pub async fn git_stage_files(
    path: String,
    files: Vec<String>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let p = path.clone();
    run_locked_blocking(&app, &path, move || {
        let repo = git2::Repository::open(&p)
            .map_err(|e| format!("リポジトリを開けません: {}", e))?;
        let mut index = repo.index().map_err(|e| e.to_string())?;

        for file in &files {
            let file_path = std::path::Path::new(file);
            let full_path = std::path::Path::new(&p).join(file);
            if full_path.exists() {
                index.add_path(file_path).map_err(|e| e.to_string())?;
            } else {
                index.remove_path(file_path).map_err(|e| e.to_string())?;
            }
        }

        index.write().map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
}

/// ファイルをアンステージ
#[tauri::command]
pub async fn git_unstage_files(
    path: String,
    files: Vec<String>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let p = path.clone();
    run_locked_blocking(&app, &path, move || unstage_files_blocking(&p, &files)).await
}

fn unstage_files_blocking(path: &str, files: &[String]) -> Result<(), String> {
    let repo = git2::Repository::open(path)
        .map_err(|e| format!("リポジトリを開けません: {}", e))?;

    let head = repo.head().map_err(|e| e.to_string())?;
    let head_commit = head.peel_to_commit().map_err(|e| e.to_string())?;
    let head_tree = head_commit.tree().map_err(|e| e.to_string())?;

    let mut index = repo.index().map_err(|e| e.to_string())?;

    for file in files {
        let file_path = std::path::Path::new(file);
        // HEAD のツリーにエントリがあれば復元、なければ削除
        match head_tree.get_path(file_path) {
            Ok(entry) => {
                let idx_entry = git2::IndexEntry {
                    ctime: git2::IndexTime::new(0, 0),
                    mtime: git2::IndexTime::new(0, 0),
                    dev: 0,
                    ino: 0,
                    mode: entry.filemode() as u32,
                    uid: 0,
                    gid: 0,
                    file_size: 0,
                    id: entry.id(),
                    flags: 0,
                    flags_extended: 0,
                    path: file.as_bytes().to_vec(),
                };
                index.add(&idx_entry).map_err(|e| e.to_string())?;
            }
            Err(_) => {
                index.remove_path(file_path).map_err(|e| e.to_string())?;
            }
        }
    }

    index.write().map_err(|e| e.to_string())?;
    Ok(())
}

/// 全ファイルをアンステージ
#[tauri::command]
pub async fn git_unstage_all(path: String, app: tauri::AppHandle) -> Result<(), String> {
    let p = path.clone();
    run_locked_blocking(&app, &path, move || {
        let repo = git2::Repository::open(&p)
            .map_err(|e| format!("リポジトリを開けません: {}", e))?;

        let head = repo.head().map_err(|e| e.to_string())?;
        let head_commit = head.peel_to_commit().map_err(|e| e.to_string())?;
        let head_tree = head_commit.tree().map_err(|e| e.to_string())?;

        repo.reset(head_tree.as_object(), git2::ResetType::Mixed, None)
            .map_err(|e| format!("Unstage All 失敗: {}", e))?;

        Ok(())
    })
    .await
}

/// 全ファイルをステージング
#[tauri::command]
pub async fn git_stage_all(path: String, app: tauri::AppHandle) -> Result<(), String> {
    let p = path.clone();
    run_locked_blocking(&app, &path, move || {
        let repo = git2::Repository::open(&p)
            .map_err(|e| format!("リポジトリを開けません: {}", e))?;
        let mut index = repo.index().map_err(|e| e.to_string())?;

        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .map_err(|e| e.to_string())?;

        // 削除されたファイルも反映
        index
            .update_all(["*"].iter(), None)
            .map_err(|e| e.to_string())?;

        index.write().map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
}

/// コミット作成
#[tauri::command]
pub async fn git_commit(
    path: String,
    message: String,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let p = path.clone();
    run_locked_blocking(&app, &path, move || commit_blocking(&p, &message)).await
}

fn commit_blocking(path: &str, message: &str) -> Result<String, String> {
    if message.trim().is_empty() {
        return Err("コミットメッセージを入力してください".to_string());
    }

    let repo = git2::Repository::open(path)
        .map_err(|e| format!("リポジトリを開けません: {}", e))?;

    let mut index = repo.index().map_err(|e| e.to_string())?;
    let tree_oid = index.write_tree().map_err(|e| e.to_string())?;
    let tree = repo.find_tree(tree_oid).map_err(|e| e.to_string())?;

    let signature = repo.signature().map_err(|e| e.to_string())?;

    let parent = match repo.head() {
        Ok(head) => {
            let commit = head.peel_to_commit().map_err(|e| e.to_string())?;
            Some(commit)
        }
        Err(_) => None,
    };

    let parents: Vec<&git2::Commit> = parent.iter().collect();

    let oid = repo
        .commit(Some("HEAD"), &signature, &signature, &message, &tree, &parents)
        .map_err(|e| format!("コミット失敗: {}", e))?;

    Ok(format!("{}", &oid.to_string()[..7]))
}

/// コミット修正 (amend)
#[tauri::command]
pub async fn git_commit_amend(
    path: String,
    message: String,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let p = path.clone();
    run_locked_blocking(&app, &path, move || commit_amend_blocking(&p, &message)).await
}

fn commit_amend_blocking(path: &str, message: &str) -> Result<String, String> {
    if message.trim().is_empty() {
        return Err("コミットメッセージを入力してください".to_string());
    }

    let repo = git2::Repository::open(path)
        .map_err(|e| format!("リポジトリを開けません: {}", e))?;

    let head = repo.head().map_err(|_| "HEAD がありません（初回コミット未実施）".to_string())?;
    let head_commit = head.peel_to_commit().map_err(|e| e.to_string())?;

    let mut index = repo.index().map_err(|e| e.to_string())?;
    let tree_oid = index.write_tree().map_err(|e| e.to_string())?;
    let tree = repo.find_tree(tree_oid).map_err(|e| e.to_string())?;

    let signature = repo.signature().map_err(|e| e.to_string())?;

    let oid = head_commit
        .amend(Some("HEAD"), Some(&signature), Some(&signature), None, Some(&message), Some(&tree))
        .map_err(|e| format!("Amend 失敗: {}", e))?;

    Ok(format!("{}", &oid.to_string()[..7]))
}

/// 前回のコミットメッセージを取得 (amend 用)
#[tauri::command]
pub fn git_last_commit_message(path: String) -> Result<String, String> {
    let repo = git2::Repository::open(&path).map_err(|e| e.to_string())?;
    let head = repo.head().map_err(|_| "まだコミットがありません".to_string())?;
    let commit = head.peel_to_commit().map_err(|e| e.to_string())?;
    Ok(commit.message().unwrap_or("").to_string())
}

/// 変更を破棄 (checkout)
#[tauri::command]
pub async fn git_discard_changes(
    path: String,
    files: Vec<String>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let p = path.clone();
    run_locked_blocking(&app, &path, move || {
        let repo = git2::Repository::open(&p)
            .map_err(|e| format!("リポジトリを開けません: {}", e))?;

        let mut checkout_builder = git2::build::CheckoutBuilder::new();
        checkout_builder.force();
        for file in &files {
            checkout_builder.path(file);
        }

        repo.checkout_head(Some(&mut checkout_builder))
            .map_err(|e| format!("変更の破棄に失敗: {}", e))?;

        Ok(())
    })
    .await
}

/// Stash 作成
#[tauri::command]
pub async fn git_stash(
    path: String,
    message: Option<String>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let p = path.clone();
    run_locked_blocking(&app, &path, move || {
        let repo = git2::Repository::open(&p)
            .map_err(|e| format!("リポジトリを開けません: {}", e))?;
        let signature = repo.signature().map_err(|e| e.to_string())?;
        let msg = message.as_deref().unwrap_or("WIP");

        // git2 の stash_save はミュータブル参照が必要
        let mut repo = repo;
        repo.stash_save(&signature, msg, None)
            .map_err(|e| format!("Stash 失敗: {}", e))?;

        Ok("Stash に保存しました".to_string())
    })
    .await
}

/// Stash Pop
#[tauri::command]
pub async fn git_stash_pop(path: String, app: tauri::AppHandle) -> Result<String, String> {
    let p = path.clone();
    run_locked_blocking(&app, &path, move || {
        let mut repo = git2::Repository::open(&p)
            .map_err(|e| format!("リポジトリを開けません: {}", e))?;

        let mut opts = git2::StashApplyOptions::new();
        repo.stash_pop(0, Some(&mut opts))
            .map_err(|e| format!("Stash Pop 失敗: {}", e))?;

        Ok("Stash を復元しました".to_string())
    })
    .await
}

/// Stash 一覧
#[tauri::command]
pub fn git_stash_list(path: String) -> Result<Vec<StashEntry>, String> {
    let mut repo = git2::Repository::open(&path)
        .map_err(|e| format!("リポジトリを開けません: {}", e))?;

    let mut entries = Vec::new();
    repo.stash_foreach(|index, message, _oid| {
        entries.push(StashEntry {
            index,
            message: message.to_string(),
        });
        true
    })
    .map_err(|e| format!("Stash 一覧取得失敗: {}", e))?;

    Ok(entries)
}

/// コミットグラフ付きログ取得
///
/// 2パス構成でコミットグラフを構築する:
///   パス1: revwalk でコミットを時系列順に収集し、RawCommit のリストを作成
///   パス2: active_columns（各列が追跡中のコミットハッシュ）を使ってレイアウトを計算
///
/// グラフレイアウトのアルゴリズム:
///   - 各コミットは active_columns 内の空き列か既存列に配置される
///   - 第1親は同じ列を引き継ぎ（直線）、第2親以降は新しい列を割り当て（マージ線）
///   - パススルー線: 自分以外のアクティブ列はそのまま次の行へ直線を引く
///   - 末尾の空き列は毎行削除してコンパクトに保つ
///   - 色は12色パレットからラウンドロビンで割り当て
#[tauri::command]
pub async fn git_log_graph(path: String, count: Option<usize>) -> Result<Vec<GraphCommit>, String> {
    // 大規模リポジトリでは revwalk + レイアウト計算が秒単位かかるためワーカースレッドで実行
    run_blocking(move || git_log_graph_blocking(&path, count)).await
}

fn git_log_graph_blocking(path: &str, count: Option<usize>) -> Result<Vec<GraphCommit>, String> {
    let repo = git2::Repository::open(path)
        .map_err(|e| format!("リポジトリを開けません: {}", e))?;

    let mut revwalk = repo.revwalk().map_err(|e| e.to_string())?;

    // 全ローカルブランチの先頭を起点に追加
    if let Ok(branches) = repo.branches(Some(git2::BranchType::Local)) {
        for branch_result in branches.flatten() {
            if let Some(oid) = branch_result.0.get().target() {
                let _ = revwalk.push(oid);
            }
        }
    }
    // リモートブランチも追加（未プルコミットを表示するため）
    if let Ok(branches) = repo.branches(Some(git2::BranchType::Remote)) {
        for branch_result in branches.flatten() {
            if let Some(oid) = branch_result.0.get().target() {
                let _ = revwalk.push(oid);
            }
        }
    }
    // HEAD も追加
    let _ = revwalk.push_head();
    revwalk.set_sorting(git2::Sort::TIME | git2::Sort::TOPOLOGICAL)
        .map_err(|e| e.to_string())?;

    let max_count = count.unwrap_or(200);
    let refs_map = build_refs_map(&repo);

    // 未プルコミットのハッシュセットを構築
    let unpulled_set = build_unpulled_set(&repo);

    // パス1: コミットデータを収集
    struct RawCommit {
        hash: String,
        short_hash: String,
        message: String,
        author: String,
        date: String,
        refs: Vec<String>,
        parents: Vec<String>,
        is_remote_only: bool,
    }

    let mut raw_commits: Vec<RawCommit> = Vec::new();

    for oid_result in revwalk.take(max_count) {
        let oid = oid_result.map_err(|e| e.to_string())?;
        let commit = repo.find_commit(oid).map_err(|e| e.to_string())?;

        let hash = oid.to_string();
        let short_hash = hash[..7].to_string();
        let refs = refs_map.get(&hash).cloned().unwrap_or_default();
        let author_sig = commit.author();
        let date = format_commit_time(author_sig.when());
        let parents: Vec<String> = commit.parent_ids().map(|id| id.to_string()).collect();

        raw_commits.push(RawCommit {
            hash,
            short_hash,
            message: commit.message().unwrap_or("").to_string(),
            author: author_sig.name().unwrap_or("").to_string(),
            date,
            refs,
            parents,
            is_remote_only: false, // 後で設定
        });
    }

    // 未プルコミットのフラグを設定
    for raw in &mut raw_commits {
        if unpulled_set.contains(&raw.hash) {
            raw.is_remote_only = true;
        }
    }

    // パス2: グラフレイアウト計算
    // active_columns[i] = Some(commit_hash) は、列 i がそのコミットの子孫ラインで使用中であることを意味する
    let mut active_columns: Vec<Option<String>> = Vec::new();
    let mut graph_commits: Vec<GraphCommit> = Vec::new();

    // ブランチごとの色パレット
    let colors = [
        "#4fc1ff", "#4ec9b0", "#ce9178", "#c586c0",
        "#dcdcaa", "#9cdcfe", "#f44747", "#608b4e",
        "#d7ba7d", "#b5cea8", "#569cd6", "#d16969",
    ];
    let mut color_index: usize = 0;
    // commit_hash → color の割り当てマップ
    let mut color_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    for raw in &raw_commits {
        let mut lines: Vec<GraphLine> = Vec::new();

        // このコミットがどの列にあるかを探す
        let col = active_columns.iter().position(|c| c.as_deref() == Some(&raw.hash));

        let my_col = if let Some(c) = col {
            c
        } else {
            // 新しい列を割り当て（空き列を探すか、末尾に追加）
            let empty = active_columns.iter().position(|c| c.is_none());
            if let Some(e) = empty {
                active_columns[e] = Some(raw.hash.clone());
                e
            } else {
                active_columns.push(Some(raw.hash.clone()));
                active_columns.len() - 1
            }
        };

        // 色の割り当て
        let my_color = color_map.entry(raw.hash.clone()).or_insert_with(|| {
            let c = colors[color_index % colors.len()].to_string();
            color_index += 1;
            c
        }).clone();

        // パススルー線: 自分以外のアクティブ列はそのまま下に続く
        for (i, slot) in active_columns.iter().enumerate() {
            if i != my_col && slot.is_some() {
                let slot_hash = slot.as_ref().unwrap();
                let line_color = color_map.get(slot_hash).cloned().unwrap_or_else(|| "#666".to_string());
                lines.push(GraphLine {
                    from_col: i,
                    to_col: i,
                    color: line_color,
                });
            }
        }

        // 自分の列をクリア (親に引き渡す)
        active_columns[my_col] = None;

        // 親の処理
        for (pi, parent_hash) in raw.parents.iter().enumerate() {
            // 親が既にアクティブ列にある場合
            let parent_col = active_columns.iter().position(|c| c.as_deref() == Some(parent_hash));

            if let Some(pc) = parent_col {
                // 既存列へのマージ線
                lines.push(GraphLine {
                    from_col: my_col,
                    to_col: pc,
                    color: my_color.clone(),
                });
            } else {
                // 親にまだ列がない → 割り当て
                if pi == 0 {
                    // 第1親: 同じ列を引き継ぐ
                    active_columns[my_col] = Some(parent_hash.clone());
                    color_map.entry(parent_hash.clone()).or_insert_with(|| my_color.clone());
                    lines.push(GraphLine {
                        from_col: my_col,
                        to_col: my_col,
                        color: my_color.clone(),
                    });
                } else {
                    // 第2親以降(マージ): 新しい列 or 空き列
                    let new_col = active_columns.iter().position(|c| c.is_none());
                    let nc = if let Some(e) = new_col {
                        active_columns[e] = Some(parent_hash.clone());
                        e
                    } else {
                        active_columns.push(Some(parent_hash.clone()));
                        active_columns.len() - 1
                    };
                    let branch_color = {
                        let c = colors[color_index % colors.len()].to_string();
                        color_index += 1;
                        c
                    };
                    color_map.entry(parent_hash.clone()).or_insert_with(|| branch_color.clone());
                    lines.push(GraphLine {
                        from_col: my_col,
                        to_col: nc,
                        color: branch_color,
                    });
                }
            }
        }

        // 末尾の空列を削除してコンパクトにする
        while active_columns.last().map_or(false, |c| c.is_none()) {
            active_columns.pop();
        }

        graph_commits.push(GraphCommit {
            commit: CommitInfo {
                hash: raw.hash.clone(),
                short_hash: raw.short_hash.clone(),
                message: raw.message.clone(),
                author: raw.author.clone(),
                date: raw.date.clone(),
                refs: raw.refs.clone(),
            },
            column: my_col,
            color: my_color,
            lines,
            is_remote_only: raw.is_remote_only,
        });
    }

    Ok(graph_commits)
}

/// コミット詳細取得（変更ファイル一覧付き）
#[tauri::command]
pub async fn git_commit_detail(path: String, hash: String) -> Result<CommitDetail, String> {
    run_blocking(move || commit_detail_blocking(&path, &hash)).await
}

fn commit_detail_blocking(path: &str, hash: &str) -> Result<CommitDetail, String> {
    let repo = git2::Repository::open(path)
        .map_err(|e| format!("リポジトリを開けません: {}", e))?;

    let oid = git2::Oid::from_str(hash)
        .map_err(|e| format!("無効なハッシュ: {}", e))?;
    let commit = repo.find_commit(oid)
        .map_err(|e| format!("コミットが見つかりません: {}", e))?;

    let author_sig = commit.author();
    let date = format_commit_time(author_sig.when());
    let parents: Vec<String> = commit.parent_ids().map(|id| id.to_string()).collect();

    // コミットの変更ファイル一覧を取得
    let commit_tree = commit.tree().map_err(|e| e.to_string())?;
    let parent_tree = if commit.parent_count() > 0 {
        Some(commit.parent(0).map_err(|e| e.to_string())?.tree().map_err(|e| e.to_string())?)
    } else {
        None
    };

    let diff = repo.diff_tree_to_tree(
        parent_tree.as_ref(),
        Some(&commit_tree),
        None,
    ).map_err(|e| format!("Diff 取得失敗: {}", e))?;

    let mut changed_files: Vec<FileStatus> = Vec::new();
    for delta in diff.deltas() {
        let file_path = delta.new_file().path()
            .or_else(|| delta.old_file().path())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        let status = match delta.status() {
            git2::Delta::Added => "A",
            git2::Delta::Deleted => "D",
            git2::Delta::Renamed => "R",
            _ => "M",
        };

        changed_files.push(FileStatus {
            path: file_path,
            status: status.to_string(),
        });
    }

    Ok(CommitDetail {
        hash: oid.to_string(),
        message: commit.message().unwrap_or("").to_string(),
        author: author_sig.name().unwrap_or("").to_string(),
        date,
        parents,
        changed_files,
    })
}

/// ファイルの diff 取得
#[tauri::command]
pub async fn git_diff_file(
    path: String,
    file_path: String,
    staged: bool,
) -> Result<DiffResult, String> {
    run_blocking(move || diff_file_blocking(&path, file_path, staged)).await
}

fn diff_file_blocking(path: &str, file_path: String, staged: bool) -> Result<DiffResult, String> {
    let repo = git2::Repository::open(path)
        .map_err(|e| format!("リポジトリを開けません: {}", e))?;

    let mut diff_opts = git2::DiffOptions::new();
    diff_opts.pathspec(&file_path);

    let diff = if staged {
        let head_tree = repo
            .head()
            .ok()
            .and_then(|h| h.peel_to_tree().ok());
        repo.diff_tree_to_index(head_tree.as_ref(), None, Some(&mut diff_opts))
    } else {
        repo.diff_index_to_workdir(None, Some(&mut diff_opts))
    }
    .map_err(|e| format!("Diff 取得失敗: {}", e))?;

    let mut hunks = Vec::new();
    let mut current_lines: Vec<DiffLine> = Vec::new();
    let mut current_header = String::new();

    diff.print(git2::DiffFormat::Patch, |_delta, hunk, line| {
        if let Some(hunk) = hunk {
            if !current_header.is_empty() || !current_lines.is_empty() {
                hunks.push(DiffHunk {
                    header: current_header.clone(),
                    lines: std::mem::take(&mut current_lines),
                });
            }
            current_header = String::from_utf8_lossy(hunk.header()).trim().to_string();
        }

        let content = String::from_utf8_lossy(line.content()).to_string();
        let kind = match line.origin() {
            '+' => "add",
            '-' => "del",
            _ => "ctx",
        };

        current_lines.push(DiffLine {
            old_lineno: line.old_lineno(),
            new_lineno: line.new_lineno(),
            kind: kind.to_string(),
            content,
        });

        true
    })
    .map_err(|e| e.to_string())?;

    // 最後のハンクを追加
    if !current_header.is_empty() || !current_lines.is_empty() {
        hunks.push(DiffHunk {
            header: current_header,
            lines: current_lines,
        });
    }

    Ok(DiffResult {
        file_path,
        hunks,
    })
}

/// エクスプローラーで開く
#[tauri::command]
pub fn open_in_explorer(path: String) -> Result<(), String> {
    Command::new("explorer")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("エクスプローラーを開けません: {}", e))?;
    Ok(())
}

/// ターミナルで開く
#[tauri::command]
pub fn open_in_terminal(path: String) -> Result<(), String> {
    Command::new("cmd")
        .args(["/c", "start", "cmd", "/k", &format!("cd /d {}", path)])
        .spawn()
        .map_err(|e| format!("ターミナルを開けません: {}", e))?;
    Ok(())
}

/// ファイルをOS既定アプリで開く
#[tauri::command]
pub fn open_file_default(path: String, file_path: String) -> Result<(), String> {
    let full_path = std::path::Path::new(&path).join(&file_path);
    if !full_path.exists() {
        return Err(format!("ファイルが見つかりません: {}", full_path.display()));
    }
    let full_path_str = full_path.to_string_lossy().to_string();
    let mut cmd = Command::new("cmd");
    cmd.args(["/c", "start", "", &full_path_str]);
    hide_console_window(&mut cmd);
    cmd.spawn()
        .map_err(|e| format!("ファイルを開けません: {}", e))?;
    Ok(())
}

/// リモートURL取得（HTTPS変換済み）
#[tauri::command]
pub fn get_remote_url(path: String) -> Result<Option<String>, String> {
    let repo = git2::Repository::open(&path)
        .map_err(|e| format!("リポジトリを開けません: {}", e))?;

    let remote = match repo.find_remote("origin") {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };

    let url = match remote.url() {
        Some(u) => u.to_string(),
        None => return Ok(None),
    };

    // SSH → HTTPS 変換
    let https_url = if url.starts_with("git@") {
        // git@github.com:user/repo.git → https://github.com/user/repo.git
        let converted = url
            .replace("git@", "https://")
            .replacen(":", "/", 1);
        converted
    } else {
        url
    };

    // 末尾の .git を除去
    let clean_url = if https_url.ends_with(".git") {
        https_url[..https_url.len() - 4].to_string()
    } else {
        https_url
    };

    Ok(Some(clean_url))
}

/// URLをブラウザで開く
#[tauri::command]
pub fn open_url_in_browser(url: String) -> Result<(), String> {
    let mut cmd = Command::new("cmd");
    cmd.args(["/c", "start", "", &url]);
    hide_console_window(&mut cmd);
    cmd.spawn()
        .map_err(|e| format!("ブラウザを開けません: {}", e))?;
    Ok(())
}

/// ahead / behind のみ取得（軽量版）
#[tauri::command]
pub fn git_ahead_behind(path: String) -> Result<(usize, usize), String> {
    let repo = git2::Repository::open(&path)
        .map_err(|e| format!("リポジトリを開けません: {}", e))?;
    get_ahead_behind(&repo).map_err(|e| e.to_string())
}

/// ファイル名でコミット検索（git log -- pattern）
#[tauri::command]
pub async fn git_log_search_by_file(
    path: String,
    pattern: String,
    count: Option<usize>,
) -> Result<Vec<String>, String> {
    run_blocking(move || {
        let max_count = count.unwrap_or(500);
        let glob = format!("*{}*", pattern);
        let mut cmd = Command::new("git");
        cmd.args([
            "-C", &path,
            "log", "--all",
            &format!("--max-count={}", max_count),
            "--format=%H",
            "--",
            &glob,
        ]);
        hide_console_window(&mut cmd);

        // 共通ランナー使用（大量マッチ時のパイプバッファ詰まり対策）
        let (success, stdout, stderr) = run_git_command_with_progress(&mut cmd, None, "log", None)?;

        if !success {
            // パターンに一致するファイルがない場合は空を返す
            if stderr.is_empty() {
                return Ok(Vec::new());
            }
            return Err(format!("git log エラー: {}", stderr));
        }

        let hashes: Vec<String> = stdout
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect();

        Ok(hashes)
    })
    .await
}

// ===== ヘルパー関数 =====

/// 進捗イベント ("git-progress") のペイロード
#[derive(Clone, serde::Serialize)]
pub struct GitProgress {
    pub op: String,
    pub line: String,
}

/// ブロッキング処理をワーカースレッドで実行する。
/// Tauri 2 では同期コマンドはメインスレッドで実行されウィンドウごとフリーズするため、
/// 重い git 操作は必ずこれを介して UI スレッドから逃がす。
pub(crate) async fn run_blocking<T, F>(f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| format!("内部エラー: {}", e))?
}

/// リポジトリ単位のロックを取得してからブロッキング処理を実行する（書き込み系操作用）。
/// 自動フェッチとユーザー操作が同一リポジトリで並行して index.lock 競合するのを防ぐ。
pub(crate) async fn run_locked_blocking<T, F>(
    app: &tauri::AppHandle,
    path: &str,
    f: F,
) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    let lock = app.state::<RepoLocks>().lock_for(path);
    let _guard = lock.lock_owned().await;
    run_blocking(f).await
}

/// op_id がキャンセル済みかを確認する（フラグは消費される）
pub(crate) fn was_cancelled(app: &tauri::AppHandle, op_id: Option<&str>) -> bool {
    match op_id {
        Some(id) => app.state::<RunningOps>().take_cancelled(id),
        None => false,
    }
}

/// 末尾 max_len バイトだけ残す（UTF-8 文字境界を壊さない位置で切る）
fn trim_to_tail(output: &mut String, max_len: usize) {
    if output.len() > max_len {
        let mut trim_at = output.len() - max_len;
        while !output.is_char_boundary(trim_at) {
            trim_at += 1;
        }
        *output = output[trim_at..].to_string();
    }
}

/// CLI git コマンドを安全に実行する共通ヘルパー。
/// `Command::output()` は stdout/stderr をすべてメモリに蓄積するため、
/// 大きなリポジトリ（30GB+ など）で git fetch/pull/clone すると
/// stderr の進捗出力でパイプバッファが溢れてデッドロックしたり、
/// 長時間ブロックされてフロントエンドがタイムアウトする。
/// この関数は spawn + スレッドで stdout/stderr を並行読み取りし、
/// バッファ溢れを防ぎつつ最後の出力のみを保持する（最大 64KB）。
///
/// さらに:
///   - `app` が Some の場合、stderr の進捗行 (`\r` / `\n` 区切り) を
///     "git-progress" イベントとしてフロントエンドへ通知する
///   - `op_id` が Some の場合、実行中プロセスの PID を RunningOps に登録し
///     `git_cancel` コマンドから kill できるようにする
///   - stdin は常に閉じる（認証プロンプト等による無限ハング防止）
pub(crate) fn run_git_command_with_progress(
    cmd: &mut Command,
    app: Option<&tauri::AppHandle>,
    op: &str,
    op_id: Option<&str>,
) -> Result<(bool, String, String), String> {
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = cmd.spawn()
        .map_err(|e| format!("git コマンド起動失敗: {}", e))?;

    // キャンセル用に PID を登録
    if let (Some(app), Some(id)) = (app, op_id) {
        app.state::<RunningOps>().register(id, child.id());
    }

    // stderr をバックグラウンドスレッドで読む（進捗出力がパイプを詰まらせるのを防ぐ）
    let stderr_handle = child.stderr.take();
    let app_clone = app.cloned();
    let op_name = op.to_string();
    let stderr_thread = std::thread::spawn(move || {
        let mut output = String::new();
        let mut segment = String::new(); // \r / \n で区切られた進捗1行分
        if let Some(mut stderr) = stderr_handle {
            let mut buf = [0u8; 8192];
            loop {
                match stderr.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let chunk = String::from_utf8_lossy(&buf[..n]);
                        // 進捗行 ("Receiving objects:  45% ..." など) を行ごとに emit
                        if let Some(app) = &app_clone {
                            for ch in chunk.chars() {
                                if ch == '\r' || ch == '\n' {
                                    let line = segment.trim();
                                    if !line.is_empty() {
                                        let _ = app.emit("git-progress", GitProgress {
                                            op: op_name.clone(),
                                            line: line.to_string(),
                                        });
                                    }
                                    segment.clear();
                                } else {
                                    segment.push(ch);
                                }
                            }
                        }
                        // 最後の 64KB のみ保持（巨大な進捗出力でメモリを食わない）
                        output.push_str(&chunk);
                        trim_to_tail(&mut output, 65536);
                    }
                    Err(_) => break,
                }
            }
        }
        output
    });

    // stdout もスレッドで読む
    let stdout_handle = child.stdout.take();
    let stdout_thread = std::thread::spawn(move || {
        let mut output = String::new();
        if let Some(mut stdout) = stdout_handle {
            let mut buf = [0u8; 8192];
            loop {
                match stdout.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let chunk = String::from_utf8_lossy(&buf[..n]);
                        output.push_str(&chunk);
                        trim_to_tail(&mut output, 65536);
                    }
                    Err(_) => break,
                }
            }
        }
        output
    });

    let status = child.wait()
        .map_err(|e| format!("git コマンド待機失敗: {}", e));

    // PID の登録解除（wait の成否に関わらず行う）
    if let (Some(app), Some(id)) = (app, op_id) {
        app.state::<RunningOps>().unregister(id);
    }
    let status = status?;

    let stdout_str = stdout_thread.join().unwrap_or_default();
    let stderr_str = stderr_thread.join().unwrap_or_default();

    Ok((status.success(), stdout_str, stderr_str))
}

/// 実行中の git 操作をキャンセルする（プロセスツリーごと強制終了）
#[tauri::command]
pub fn git_cancel(op_id: String, ops: tauri::State<'_, RunningOps>) -> Result<(), String> {
    let pid = ops.pid_of(&op_id)
        .ok_or_else(|| "該当する実行中の操作がありません".to_string())?;

    // kill 後にコマンド側が「キャンセルされた」と判別できるよう先にマークする
    ops.mark_cancelled(&op_id);

    #[cfg(target_os = "windows")]
    {
        // git は子プロセス (git-remote-https 等) を生むため /T でツリーごと終了する
        let mut kill = Command::new("taskkill");
        kill.args(["/T", "/F", "/PID", &pid.to_string()]);
        hide_console_window(&mut kill);
        kill.output().map_err(|e| format!("プロセス停止失敗: {}", e))?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = Command::new("kill")
            .args(["-KILL", &pid.to_string()])
            .output();
    }

    Ok(())
}

/// Windows では .git 配下の読み取り専用ファイルで remove_dir_all が失敗するため、
/// 読み取り専用属性を外しながらベストエフォートで削除する（キャンセルされた clone の後始末用）
fn remove_dir_all_force(path: &std::path::Path) {
    fn clear_readonly(p: &std::path::Path) {
        if let Ok(meta) = p.symlink_metadata() {
            let mut perm = meta.permissions();
            if perm.readonly() {
                perm.set_readonly(false);
                let _ = std::fs::set_permissions(p, perm);
            }
            if meta.is_dir() {
                if let Ok(entries) = std::fs::read_dir(p) {
                    for e in entries.flatten() {
                        clear_readonly(&e.path());
                    }
                }
            }
        }
    }
    clear_readonly(path);
    let _ = std::fs::remove_dir_all(path);
}

/// 未プル（リモートにのみ存在する）コミットのハッシュセットを構築
fn build_unpulled_set(repo: &git2::Repository) -> std::collections::HashSet<String> {
    let mut unpulled = std::collections::HashSet::new();

    // HEAD のブランチについて upstream との差分を計算
    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => return unpulled,
    };
    let local_oid = match head.target() {
        Some(oid) => oid,
        None => return unpulled,
    };
    let branch_name = head.shorthand().unwrap_or("");
    let upstream_name = format!("refs/remotes/origin/{}", branch_name);
    let upstream_ref = match repo.find_reference(&upstream_name) {
        Ok(r) => r,
        Err(_) => return unpulled,
    };
    let upstream_oid = match upstream_ref.target() {
        Some(oid) => oid,
        None => return unpulled,
    };

    // merge-base を取得
    let merge_base = match repo.merge_base(local_oid, upstream_oid) {
        Ok(oid) => oid,
        Err(_) => return unpulled,
    };

    // upstream から merge-base までの間のコミットを列挙
    let mut revwalk = match repo.revwalk() {
        Ok(rw) => rw,
        Err(_) => return unpulled,
    };
    let _ = revwalk.push(upstream_oid);
    let _ = revwalk.hide(merge_base);
    let _ = revwalk.set_sorting(git2::Sort::TIME);

    for oid_result in revwalk {
        if let Ok(oid) = oid_result {
            unpulled.insert(oid.to_string());
        }
    }

    unpulled
}

/// ブランチ/タグ → コミットハッシュのマップ
fn build_refs_map(repo: &git2::Repository) -> std::collections::HashMap<String, Vec<String>> {
    let mut map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

    if let Ok(refs) = repo.references() {
        for reference in refs.flatten() {
            if let Some(target) = reference.target() {
                let name = reference.shorthand().unwrap_or("").to_string();
                map.entry(target.to_string())
                    .or_default()
                    .push(name);
            }
        }
    }

    map
}

/// コミット時刻をフォーマット (YYYY/MM/DD HH:MM)
fn format_commit_time(time: git2::Time) -> String {
    let secs = time.seconds();
    let offset_min = time.offset_minutes();
    let local_secs = secs + (offset_min as i64) * 60;

    // UNIX epoch からの日時計算
    let days = local_secs.div_euclid(86400);
    let day_secs = local_secs.rem_euclid(86400);
    let hour = day_secs / 3600;
    let minute = (day_secs % 3600) / 60;

    // 日数から年月日を算出
    let (year, month, day) = days_to_ymd(days);

    format!("{:04}/{:02}/{:02} {:02}:{:02}", year, month, day, hour, minute)
}

/// UNIX epoch からの通算日数を年月日に変換
///
/// Howard Hinnant の civil_from_days アルゴリズムを使用。
/// 参照: https://howardhinnant.github.io/date_algorithms.html#civil_from_days
fn days_to_ymd(days: i64) -> (i64, i64, i64) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

// =========================================
//  Revert / Reset
// =========================================

/// ハッシュ文字列のバリデーション（hex文字のみ許可）
fn validate_hash(hash: &str) -> Result<(), String> {
    if hash.is_empty() || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("無効なコミットハッシュです".to_string());
    }
    Ok(())
}

/// git reset (CLI)
/// mode: "hard" = 完全に戻す（変更破棄）, "soft" = HEAD移動のみ（変更をステージに維持）
#[tauri::command]
pub async fn git_reset(
    path: String,
    hash: String,
    mode: String,
    app: tauri::AppHandle,
) -> Result<String, String> {
    validate_hash(&hash)?;

    // mode バリデーション
    let reset_mode = match mode.as_str() {
        "hard" => "--hard",
        "soft" => "--soft",
        _ => return Err("無効なリセットモードです（hard または soft のみ）".to_string()),
    };

    let p = path.clone();
    run_locked_blocking(&app, &path, move || {
        let mut cmd = Command::new("git");
        // core.longpaths: reset --hard はワークツリーを書き換えるため長いパス対策を入れる
        cmd.args(["-c", "core.longpaths=true", "reset", reset_mode, &hash])
            .current_dir(&p);
        hide_console_window(&mut cmd);
        let (success, _stdout, stderr) = run_git_command_with_progress(&mut cmd, None, "reset", None)?;

        if success {
            if mode == "hard" {
                Ok("Reset 完了（変更を破棄しました）".to_string())
            } else {
                Ok("Reset 完了（変更はステージに維持）".to_string())
            }
        } else {
            Err(format!("Reset 失敗: {}", stderr))
        }
    })
    .await
}

/// git gc --auto（不要オブジェクトの最適化）
#[tauri::command]
pub async fn git_gc(path: String, app: tauri::AppHandle) -> Result<String, String> {
    let p = path.clone();
    run_locked_blocking(&app, &path, move || {
        let mut cmd = Command::new("git");
        cmd.args(["gc", "--auto"]).current_dir(&p);
        hide_console_window(&mut cmd);
        let (success, _stdout, stderr) = run_git_command_with_progress(&mut cmd, None, "gc", None)?;

        if success {
            Ok("最適化完了".to_string())
        } else {
            Err(format!("最適化失敗: {}", stderr))
        }
    })
    .await
}
