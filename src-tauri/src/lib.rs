mod models;
mod commands;

use commands::project::AppState;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager, WindowEvent,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // 2つ目のインスタンスが起動されたら、既存ウィンドウを前面に出す
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::new())
        .setup(|app| {
            // --- システムトレイ ---
            let show = MenuItem::with_id(app, "show", "表示", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "終了", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;

            let icon = Image::from_path("icons/icon.png").unwrap_or_else(|_| {
                Image::from_bytes(include_bytes!("../icons/icon.png")).expect("tray icon")
            });

            TrayIconBuilder::new()
                .icon(icon)
                .tooltip("GitGUI")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.unminimize();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::DoubleClick { .. } = event {
                        if let Some(w) = tray.app_handle().get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.unminimize();
                            let _ = w.set_focus();
                        }
                    }
                })
                .build(app)?;

            // --- ×ボタンでウィンドウを隠す（トレイに格納） ---
            let window = app.get_webview_window("main").unwrap();
            let win_clone = window.clone();
            window.on_window_event(move |event| {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = win_clone.hide();
                }
            });

            Ok(())
        })
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
            commands::git_ops::git_reset,
            // ブランチ操作
            commands::branch::git_list_branches,
            commands::branch::git_checkout_branch,
            commands::branch::git_create_branch,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
