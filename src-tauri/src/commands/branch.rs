use tauri::Manager;

use crate::commands::git_ops::{
    hide_console_window, run_blocking, run_git_command_with_progress, was_cancelled,
};
use crate::commands::locks::RepoLocks;
use crate::models::BranchInfo;

/// ブランチ一覧取得
#[tauri::command]
pub fn git_list_branches(path: String) -> Result<Vec<BranchInfo>, String> {
    let repo = git2::Repository::open(&path)
        .map_err(|e| format!("リポジトリを開けません: {}", e))?;

    let current_branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(|s| s.to_string()))
        .unwrap_or_default();

    let mut branches = Vec::new();

    // ローカルブランチ
    if let Ok(local_branches) = repo.branches(Some(git2::BranchType::Local)) {
        for branch_result in local_branches {
            if let Ok((branch, _)) = branch_result {
                let name = branch
                    .name()
                    .ok()
                    .flatten()
                    .unwrap_or("")
                    .to_string();
                branches.push(BranchInfo {
                    is_current: name == current_branch,
                    name,
                    is_remote: false,
                });
            }
        }
    }

    // リモートブランチ
    if let Ok(remote_branches) = repo.branches(Some(git2::BranchType::Remote)) {
        for branch_result in remote_branches {
            if let Ok((branch, _)) = branch_result {
                let name = branch
                    .name()
                    .ok()
                    .flatten()
                    .unwrap_or("")
                    .to_string();
                // HEAD 参照をスキップ
                if name.ends_with("/HEAD") {
                    continue;
                }
                branches.push(BranchInfo {
                    name,
                    is_remote: true,
                    is_current: false,
                });
            }
        }
    }

    Ok(branches)
}

/// ブランチ切り替え
///
/// git2 の checkout_tree (safe モード) は Windows の CRLF 正規化の扱いが git CLI と
/// 異なり、大きなリポジトリで「conflict prevents checkout」になりやすい。
/// また UI は「未コミットの変更は引き継がれる」と案内しているため、
/// その動作と一致する git CLI の checkout を使う。
#[tauri::command]
pub async fn git_checkout_branch(
    path: String,
    branch_name: String,
    op_id: Option<String>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let lock = app.state::<RepoLocks>().lock_for(&path);
    let _guard = lock.lock_owned().await;
    run_blocking(move || checkout_blocking(&path, &branch_name, op_id.as_deref(), &app)).await
}

/// ロックを取らない checkout 本体（git_create_branch から再利用してロックの二重取得を防ぐ）
fn checkout_blocking(
    path: &str,
    branch_name: &str,
    op_id: Option<&str>,
    app: &tauri::AppHandle,
) -> Result<String, String> {
    // checkout 対象と引数を決める:
    //   - ローカルブランチがあればそのまま切り替え
    //   - "origin/feature" 形式は、既にローカル "feature" があればそれへ切り替え、
    //     なければ --track でローカル追跡ブランチを作成して切り替え
    let (checkout_args, target) = {
        let repo = git2::Repository::open(path)
            .map_err(|e| format!("リポジトリを開けません: {}", e))?;
        let is_local =
            |name: &str| repo.find_reference(&format!("refs/heads/{}", name)).is_ok();

        if is_local(branch_name) {
            (vec![branch_name.to_string()], branch_name.to_string())
        } else if let Some((_, local)) = branch_name.split_once('/') {
            if is_local(local) {
                (vec![local.to_string()], local.to_string())
            } else {
                (
                    vec!["--track".to_string(), branch_name.to_string()],
                    local.to_string(),
                )
            }
        } else {
            // ローカルに存在しない名前は git の DWIM に任せる
            // （リモートに同名ブランチがあれば追跡ブランチを自動作成）
            (vec![branch_name.to_string()], branch_name.to_string())
        }
    };

    let mut cmd = std::process::Command::new("git");
    // core.longpaths: Windows の長いパス対策（他 OS では無視され無害）
    cmd.args(["-c", "core.longpaths=true", "checkout", "--progress"]);
    for arg in &checkout_args {
        cmd.arg(arg);
    }
    cmd.current_dir(path);
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    hide_console_window(&mut cmd);

    let (success, _stdout, stderr) =
        run_git_command_with_progress(&mut cmd, Some(app), "checkout", op_id)?;

    if was_cancelled(app, op_id) {
        return Err("ブランチ切り替えをキャンセルしました".to_string());
    }
    if success {
        Ok(format!("ブランチ '{}' に切り替えました", target))
    } else {
        Err(format!("チェックアウト失敗: {}", stderr))
    }
}

/// 新規ブランチ作成
#[tauri::command]
pub async fn git_create_branch(
    path: String,
    branch_name: String,
    checkout: bool,
    app: tauri::AppHandle,
) -> Result<String, String> {
    // ロックはここで1回だけ取得する。内部の checkout_blocking はロックを取らないため
    // 二重取得によるデッドロックは起きない
    let lock = app.state::<RepoLocks>().lock_for(&path);
    let _guard = lock.lock_owned().await;

    run_blocking(move || {
        {
            let repo = git2::Repository::open(&path)
                .map_err(|e| format!("リポジトリを開けません: {}", e))?;

            let head = repo.head().map_err(|e| e.to_string())?;
            let head_commit = head.peel_to_commit().map_err(|e| e.to_string())?;

            repo.branch(&branch_name, &head_commit, false)
                .map_err(|e| format!("ブランチ作成失敗: {}", e))?;
        }

        if checkout {
            checkout_blocking(&path, &branch_name, None, &app)?;
            return Ok(format!("ブランチ '{}' を作成して切り替えました", branch_name));
        }

        Ok(format!("ブランチ '{}' を作成しました", branch_name))
    })
    .await
}

/// ブランチ削除
#[tauri::command]
pub async fn git_delete_branch(
    path: String,
    branch_name: String,
    force: bool,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let lock = app.state::<RepoLocks>().lock_for(&path);
    let _guard = lock.lock_owned().await;
    run_blocking(move || delete_branch_blocking(&path, &branch_name, force)).await
}

fn delete_branch_blocking(path: &str, branch_name: &str, force: bool) -> Result<String, String> {
    let repo = git2::Repository::open(path)
        .map_err(|e| format!("リポジトリを開けません: {}", e))?;

    // 現在のブランチは削除不可
    let current_branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(|s| s.to_string()))
        .unwrap_or_default();

    if branch_name == current_branch {
        return Err("現在チェックアウト中のブランチは削除できません".to_string());
    }

    let mut branch = repo
        .find_branch(&branch_name, git2::BranchType::Local)
        .map_err(|e| format!("ブランチ '{}' が見つかりません: {}", branch_name, e))?;

    if force {
        branch.delete()
            .map_err(|e| format!("ブランチ削除失敗: {}", e))?;
    } else {
        // マージ済みかチェック
        if !branch.is_head() {
            let branch_commit = branch.get().peel_to_commit()
                .map_err(|e| format!("コミット取得失敗: {}", e))?;
            let head_commit = repo.head()
                .map_err(|e| e.to_string())?
                .peel_to_commit()
                .map_err(|e| e.to_string())?;

            let merge_base = repo.merge_base(branch_commit.id(), head_commit.id());
            let is_merged = match merge_base {
                Ok(base) => base == branch_commit.id(),
                Err(_) => false,
            };

            if !is_merged {
                return Err(format!(
                    "ブランチ '{}' はマージされていません。強制削除する場合は確認してください。",
                    branch_name
                ));
            }
        }
        branch.delete()
            .map_err(|e| format!("ブランチ削除失敗: {}", e))?;
    }

    Ok(format!("ブランチ '{}' を削除しました", branch_name))
}
