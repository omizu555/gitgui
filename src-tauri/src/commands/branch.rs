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
#[tauri::command]
pub fn git_checkout_branch(path: String, branch_name: String) -> Result<String, String> {
    let repo = git2::Repository::open(&path)
        .map_err(|e| format!("リポジトリを開けません: {}", e))?;

    // ブランチの参照を取得（ローカル優先、なければリモートから追跡ブランチを作成）
    let refname = format!("refs/heads/{}", branch_name);
    let reference = match repo.find_reference(&refname) {
        Ok(r) => r,
        Err(_) => {
            // ローカルに見つからない → リモートブランチを検索
            // "origin/feature" → "feature" としてローカル追跡ブランチを作成
            let local_name = if branch_name.contains('/') {
                branch_name.splitn(2, '/').nth(1).unwrap_or(&branch_name)
            } else {
                &branch_name
            };
            let remote_refname = format!("refs/remotes/{}", branch_name);
            let remote_ref = repo.find_reference(&remote_refname)
                .map_err(|e| format!("ブランチ '{}' が見つかりません: {}", branch_name, e))?;
            let remote_commit = remote_ref.peel_to_commit()
                .map_err(|e| format!("コミットの取得に失敗: {}", e))?;

            // ローカル追跡ブランチを作成
            repo.branch(local_name, &remote_commit, false)
                .map_err(|e| format!("追跡ブランチの作成に失敗: {}", e))?;

            let local_refname = format!("refs/heads/{}", local_name);
            let local_ref = repo.find_reference(&local_refname)
                .map_err(|e| format!("作成したブランチが見つかりません: {}", e))?;

            // ローカルブランチ名で checkout を続行
            let commit = local_ref.peel_to_commit().map_err(|e| e.to_string())?;
            let tree = commit.tree().map_err(|e| e.to_string())?;
            let mut checkout_builder = git2::build::CheckoutBuilder::new();
            checkout_builder.safe();
            repo.checkout_tree(tree.as_object(), Some(&mut checkout_builder))
                .map_err(|e| format!("チェックアウト失敗: {}", e))?;
            repo.set_head(&local_refname)
                .map_err(|e| format!("HEAD 更新失敗: {}", e))?;
            return Ok(format!("リモートブランチ '{}' をローカル '{}' として切り替えました", branch_name, local_name));
        }
    };

    let commit = reference
        .peel_to_commit()
        .map_err(|e| format!("コミットの取得に失敗: {}", e))?;

    let tree = commit.tree().map_err(|e| e.to_string())?;

    // checkout
    let mut checkout_builder = git2::build::CheckoutBuilder::new();
    checkout_builder.safe();
    repo.checkout_tree(tree.as_object(), Some(&mut checkout_builder))
        .map_err(|e| format!("チェックアウト失敗: {}", e))?;

    // HEAD を更新
    repo.set_head(&refname)
        .map_err(|e| format!("HEAD 更新失敗: {}", e))?;

    Ok(format!("ブランチ '{}' に切り替えました", branch_name))
}

/// 新規ブランチ作成
#[tauri::command]
pub fn git_create_branch(path: String, branch_name: String, checkout: bool) -> Result<String, String> {
    let repo = git2::Repository::open(&path)
        .map_err(|e| format!("リポジトリを開けません: {}", e))?;

    let head = repo.head().map_err(|e| e.to_string())?;
    let head_commit = head.peel_to_commit().map_err(|e| e.to_string())?;

    repo.branch(&branch_name, &head_commit, false)
        .map_err(|e| format!("ブランチ作成失敗: {}", e))?;

    if checkout {
        git_checkout_branch(path, branch_name.clone())?;
    }

    Ok(format!("ブランチ '{}' を作成しました", branch_name))
}
