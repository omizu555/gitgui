// ===== types.ts — Rust models.rs に対応する TypeScript 型定義 =====

/** プロジェクト（タブ1つに対応） */
export interface Project {
  id: string;
  name: string;
  path: string;
  order: number;
}

/** リポジトリの状態全体 */
export interface RepoStatus {
  branch: string;
  ahead: number;
  behind: number;
  staged: FileStatus[];
  unstaged: FileStatus[];
}

/** ファイルの変更ステータス */
export interface FileStatus {
  path: string;
  /** "M" = Modified, "A" = New/Added, "D" = Deleted, "R" = Renamed */
  status: string;
}

/** コミット情報 */
export interface CommitInfo {
  hash: string;
  short_hash: string;
  message: string;
  author: string;
  date: string;
  refs: string[];
}

/** コミットグラフ用 — コミット＋描画情報 */
export interface GraphCommit {
  commit: CommitInfo;
  column: number;
  color: string;
  lines: GraphLine[];
  /** リモートにのみ存在する（未プル）コミット */
  is_remote_only: boolean;
}

/** コミットグラフの線分 */
export interface GraphLine {
  from_col: number;
  to_col: number;
  color: string;
}

/** ブランチ情報 */
export interface BranchInfo {
  name: string;
  is_remote: boolean;
  is_current: boolean;
}

/** Stash エントリ */
export interface StashEntry {
  index: number;
  message: string;
}

/** Diff 結果 */
export interface DiffResult {
  file_path: string;
  hunks: DiffHunk[];
}

/** Diff のハンク (区間) */
export interface DiffHunk {
  header: string;
  lines: DiffLine[];
}

/** Diff の1行 */
export interface DiffLine {
  old_lineno: number | null;
  new_lineno: number | null;
  /** "add", "del", "ctx" */
  kind: string;
  content: string;
}

/** コミット詳細（変更ファイル一覧付き） */
export interface CommitDetail {
  hash: string;
  message: string;
  author: string;
  date: string;
  parents: string[];
  changed_files: FileStatus[];
}

/** 最近閉じたプロジェクト */
export interface RecentProject {
  name: string;
  path: string;
  branch: string;
  closed_at: string;
}

/** アプリ設定 */
export interface AppSettings {
  auto_fetch_enabled: boolean;
  auto_fetch_interval_minutes: number;
}

// ===== フロントエンド固有の型 =====

/** ファイルリスト表示用（staged フラグ付き） */
export interface DisplayFile extends FileStatus {
  staged: boolean;
}
