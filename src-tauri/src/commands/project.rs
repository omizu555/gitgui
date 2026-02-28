use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

use crate::models::{AppData, AppSettings, Project, RecentProject};

/// アプリの状態管理
pub struct AppState {
    pub data: Mutex<AppData>,
    pub data_path: PathBuf,
}

impl AppState {
    pub fn new() -> Self {
        let data_path = Self::get_data_path();
        let data = Self::load_from_file(&data_path);
        Self {
            data: Mutex::new(data),
            data_path,
        }
    }

    fn get_data_path() -> PathBuf {
        let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        let app_dir = base.join("GitGUI");
        fs::create_dir_all(&app_dir).ok();
        app_dir.join("app_data.json")
    }

    fn load_from_file(path: &PathBuf) -> AppData {
        match fs::read_to_string(path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => AppData::default(),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let data = self.data.lock().map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(&*data)
            .map_err(|e| e.to_string())?;
        fs::write(&self.data_path, json)
            .map_err(|e| format!("データ保存に失敗: {}", e))?;
        Ok(())
    }
}

/// プロジェクト追加 (パス指定)
#[tauri::command]
pub fn add_project(path: String, state: State<'_, AppState>) -> Result<Project, String> {
    // Git リポジトリか検証
    let repo_path = std::path::Path::new(&path);
    if !repo_path.exists() {
        return Err("指定されたパスが存在しません".to_string());
    }
    git2::Repository::open(&path)
        .map_err(|_| "指定されたフォルダは Git リポジトリではありません".to_string())?;

    let name = repo_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let id = format!("{}_{}", name, chrono_simple_id());

    let project = Project {
        id: id.clone(),
        name,
        path: path.clone(),
        order: 0,
    };

    let mut data = state.data.lock().map_err(|e| e.to_string())?;

    // 重複チェック
    if data.projects.iter().any(|p| p.path == path) {
        return Err("このリポジトリは既にプロジェクトに追加されています".to_string());
    }

    // order を末尾に
    let max_order = data.projects.iter().map(|p| p.order).max().unwrap_or(0);
    let mut project = project;
    project.order = max_order + 1;

    data.projects.push(project.clone());

    // recent から除去
    data.recent_projects.retain(|r| r.path != path);

    drop(data);
    state.save()?;

    Ok(project)
}

/// プロジェクト削除
#[tauri::command]
pub fn remove_project(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut data = state.data.lock().map_err(|e| e.to_string())?;

    // recent に移動
    if let Some(proj) = data.projects.iter().find(|p| p.id == id) {
        let branch = get_current_branch(&proj.path).unwrap_or_else(|_| "unknown".to_string());
        let recent = RecentProject {
            name: proj.name.clone(),
            path: proj.path.clone(),
            branch,
            closed_at: current_timestamp(),
        };
        // recent の先頭に追加 (最大10件)
        data.recent_projects.insert(0, recent);
        data.recent_projects.truncate(10);
    }

    data.projects.retain(|p| p.id != id);

    drop(data);
    state.save()?;
    Ok(())
}

/// プロジェクト一覧
#[tauri::command]
pub fn list_projects(state: State<'_, AppState>) -> Result<Vec<Project>, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let mut projects = data.projects.clone();
    projects.sort_by_key(|p| p.order);
    Ok(projects)
}

/// 最近閉じたプロジェクト一覧
#[tauri::command]
pub fn list_recent_projects(state: State<'_, AppState>) -> Result<Vec<RecentProject>, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    Ok(data.recent_projects.clone())
}

/// プロジェクト順序更新
#[tauri::command]
pub fn reorder_projects(ids: Vec<String>, state: State<'_, AppState>) -> Result<(), String> {
    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    for (i, id) in ids.iter().enumerate() {
        if let Some(proj) = data.projects.iter_mut().find(|p| &p.id == id) {
            proj.order = i;
        }
    }
    drop(data);
    state.save()?;
    Ok(())
}

/// 現在のブランチ名を取得 (ユーティリティ)
fn get_current_branch(path: &str) -> Result<String, String> {
    let repo = git2::Repository::open(path).map_err(|e| e.to_string())?;
    let head = repo.head().map_err(|e| e.to_string())?;
    Ok(head
        .shorthand()
        .unwrap_or("HEAD")
        .to_string())
}

/// 簡易ID生成
fn chrono_simple_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", dur.as_millis() % 1_000_000_000)
}

/// 現在のタイムスタンプ
fn current_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", dur.as_secs())
}

/// 設定取得
#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    Ok(data.settings.clone())
}

/// 設定保存
#[tauri::command]
pub fn save_settings(settings: AppSettings, state: State<'_, AppState>) -> Result<(), String> {
    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    data.settings = settings;
    drop(data);
    state.save()?;
    Ok(())
}
