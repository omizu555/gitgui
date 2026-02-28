mod models;
mod commands;

use commands::project::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            // プロジェクト管理
            commands::project::add_project,
            commands::project::remove_project,
            commands::project::list_projects,
            commands::project::list_recent_projects,
            commands::project::reorder_projects,
            commands::project::get_settings,
            commands::project::save_settings,
            // Git 操作
            commands::git_ops::git_status,
            commands::git_ops::git_clone,
            commands::git_ops::git_fetch,
            commands::git_ops::git_pull,
            commands::git_ops::git_push,
            commands::git_ops::git_stage_files,
            commands::git_ops::git_unstage_files,
            commands::git_ops::git_stage_all,
            commands::git_ops::git_commit,
            commands::git_ops::git_commit_amend,
            commands::git_ops::git_last_commit_message,
            commands::git_ops::git_discard_changes,
            commands::git_ops::git_stash,
            commands::git_ops::git_stash_pop,
            commands::git_ops::git_stash_list,
            commands::git_ops::git_log,
            commands::git_ops::git_log_graph,
            commands::git_ops::git_diff_file,
            commands::git_ops::git_commit_detail,
            commands::git_ops::open_in_explorer,
            commands::git_ops::open_in_terminal,
            commands::git_ops::open_file_default,
            commands::git_ops::get_remote_url,
            commands::git_ops::open_url_in_browser,
            commands::git_ops::git_ahead_behind,
            commands::git_ops::git_log_search_by_file,
            // ブランチ操作
            commands::branch::git_list_branches,
            commands::branch::git_checkout_branch,
            commands::branch::git_create_branch,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
