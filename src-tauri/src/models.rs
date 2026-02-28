use serde::{Deserialize, Serialize};

/// プロジェクト（タブ1つに対応）
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub order: usize,
}

/// リポジトリの状態全体
#[derive(Serialize, Clone, Debug)]
pub struct RepoStatus {
    pub branch: String,
    pub ahead: usize,
    pub behind: usize,
    pub staged: Vec<FileStatus>,
    pub unstaged: Vec<FileStatus>,
}

/// ファイルの変更ステータス
#[derive(Serialize, Clone, Debug)]
pub struct FileStatus {
    pub path: String,
    /// "M" = Modified, "A" = New/Added, "D" = Deleted, "R" = Renamed
    pub status: String,
}

/// コミット情報
#[derive(Serialize, Clone, Debug)]
pub struct CommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
    pub author: String,
    pub date: String,
    pub refs: Vec<String>,
}

/// コミットグラフ用 — コミット＋描画情報
#[derive(Serialize, Clone, Debug)]
pub struct GraphCommit {
    pub commit: CommitInfo,
    pub column: usize,
    pub color: String,
    pub lines: Vec<GraphLine>,
    /// リモートにのみ存在する（未プル）コミット
    #[serde(default)]
    pub is_remote_only: bool,
}

/// コミットグラフの線分
#[derive(Serialize, Clone, Debug)]
pub struct GraphLine {
    pub from_col: usize,
    pub to_col: usize,
    pub color: String,
}

/// ブランチ情報
#[derive(Serialize, Clone, Debug)]
pub struct BranchInfo {
    pub name: String,
    pub is_remote: bool,
    pub is_current: bool,
}

/// Stash エントリ
#[derive(Serialize, Clone, Debug)]
pub struct StashEntry {
    pub index: usize,
    pub message: String,
}

/// Diff 結果
#[derive(Serialize, Clone, Debug)]
pub struct DiffResult {
    pub file_path: String,
    pub hunks: Vec<DiffHunk>,
}

/// Diff のハンク (区間)
#[derive(Serialize, Clone, Debug)]
pub struct DiffHunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

/// Diff の1行
#[derive(Serialize, Clone, Debug)]
pub struct DiffLine {
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
    /// "add", "del", "ctx"
    pub kind: String,
    pub content: String,
}

/// コミット詳細（ファイル変更一覧付き）
#[derive(Serialize, Clone, Debug)]
pub struct CommitDetail {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub date: String,
    pub parents: Vec<String>,
    pub changed_files: Vec<FileStatus>,
}

/// 最近閉じたプロジェクト
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecentProject {
    pub name: String,
    pub path: String,
    pub branch: String,
    pub closed_at: String,
}

/// アプリ設定
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppSettings {
    /// 自動フェッチを有効にする
    #[serde(default)]
    pub auto_fetch_enabled: bool,
    /// 自動フェッチ間隔（分）
    #[serde(default = "default_auto_fetch_interval")]
    pub auto_fetch_interval_minutes: u32,
}

fn default_auto_fetch_interval() -> u32 {
    10
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            auto_fetch_enabled: false,
            auto_fetch_interval_minutes: 10,
        }
    }
}

/// アプリ全体の永続化データ
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AppData {
    pub projects: Vec<Project>,
    #[serde(default)]
    pub recent_projects: Vec<RecentProject>,
    #[serde(default)]
    pub settings: AppSettings,
}
