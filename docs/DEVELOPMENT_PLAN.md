# GitGUI 開発計画

## 概要

Rust + Tauri v2 で構築する Git GUI アプリケーション。  
複数の Git リポジトリをタブで管理し、基本的な Git 操作を GUI で実行できる。

---

## 技術スタック

| レイヤー | 技術 |
|---|---|
| バックエンド | Rust (Tauri v2 コマンド) |
| Git操作 | `git2` crate (libgit2バインディング) + 一部 `git` CLI 呼び出し |
| フロントエンド | HTML + CSS + **TypeScript** (フレームワーク不使用、軽量) |
| ビルドツール | **Vite** (TypeScript のトランスパイル + HMR) |
| ビルド | Cargo + Tauri CLI v2 |
| 対象OS | Windows (将来的にクロスプラットフォーム対応可能) |

> **変更履歴 (2026-02-28):** フロントエンドを JavaScript → TypeScript に切り替え。
> Rust の `models.rs` と対になる型定義を持つことでフロント・バック間の型安全を確保。
> ビルドツールとして Vite を導入し、HMR（ホットモジュールリプレースメント）にも対応。

---

## アーキテクチャ

```
┌─────────────────────────────────────────────┐
│           フロントエンド (WebView)             │
│  ┌─────────────────────────────────────────┐ │
│  │  タブバー (プロジェクト管理)               │ │
│  ├─────────────────────────────────────────┤ │
│  │  ツールバー                              │ │
│  │  [Clone][Fetch][Pull][Push][Stash][Open]  │ │
│  ├─────────────────────────────────────────┤ │
│  │  メインパネル                            │ │
│  │  ┌──────────────┬──────────────────────┐ │ │
│  │  │ 変更ファイル   │ コミットログ          │ │ │
│  │  │ 一覧          │                     │ │ │
│  │  └──────────────┴──────────────────────┘ │ │
│  └─────────────────────────────────────────┘ │
└──────────────────┬──────────────────────────┘
                   │ Tauri IPC (invoke)
┌──────────────────▼──────────────────────────┐
│           バックエンド (Rust)                  │
│  - git2 crate による Git 操作                 │
│  - プロジェクト管理 (JSON 永続化)              │
│  - ファイルシステム操作                        │
└─────────────────────────────────────────────┘
```

---

## 機能一覧

### 必須機能

| # | 機能 | 説明 |
|---|---|---|
| 1 | Clone | URL を指定してリポジトリをクローン |
| 2 | Fetch | リモートから最新情報を取得 |
| 3 | Pull | リモートの変更をマージ |
| 4 | Commit | ファイルのステージング＋コミット |
| 5 | ブランチ変更 | ブランチ一覧表示・切り替え・新規作成 |
| 6 | タブ管理 | 複数リポジトリをタブで管理 |
| 7 | ファイル右クリックメニュー | ファイルを右クリックで Stage / Unstage / Diff 等の操作 |
| 8 | ファイル複数選択 | Ctrl+Click / Shift+Click でファイルを複数選択、右クリックは選択済みファイルに一括適用 |
| 9 | Push | ローカルコミットをリモートにプッシュ |
| 10 | Stash（一時退避） | 未コミットの変更を一時的に退避・復元 (stash / stash pop) |
| 11 | Commit Amend | 直前のコミットを修正（メッセージ変更・ファイル追加） |
| 12 | 簡易 Diff ビューア | ファイルの変更差分を行単位で色付き表示（緑=追加 / 赤=削除） |
| 13 | コミットログ検索 | コミットメッセージ・作者名でフィルター検索 |
| 14 | D&D でリポジトリ追加 | フォルダをウィンドウにドラッグ&ドロップでタブ追加 |
| 15 | エクスプローラー/ターミナルで開く | リポジトリを外部アプリで開く |
| 16 | タブの並び替え | ドラッグでタブの順序を変更 |

### 情報表示機能

| # | 機能 | 説明 |
|---|---|---|
| 17 | リポジトリステータス | 変更ファイル一覧、現在のブランチ表示 |
| 18 | コミットログ | 直近のコミット履歴を表示 |
| 19 | コミットグラフ | ブランチの分岐・マージを視覚的にグラフ表示（git log --graph 風） |
| 20 | ブランチ一覧 | ローカル＆リモートブランチ一覧 |

---

## 実装フェーズ

### Phase 1: プロジェクト基盤
- [ ] Tauri v2 プロジェクトのセットアップ
- [ ] 基本 UI 骨格（タブバー、ツールバー、メインパネル）
- [ ] プロジェクト（リポジトリ）の追加・削除・タブ管理
- [ ] プロジェクト情報の永続化 (JSON)
- [ ] ドラッグ&ドロップでリポジトリ追加
- [ ] タブのドラッグ並び替え

### Phase 2: Git 基本操作（必須機能）
- [ ] Clone — URL 指定でクローン
- [ ] Fetch — リモートから取得
- [ ] Pull — リモートからプル
- [ ] Push — リモートへプッシュ
- [ ] Commit — ステージング＆コミット
- [ ] Commit Amend — 直前のコミットを修正
- [ ] Stash — 変更の一時退避・復元
- [ ] ブランチ操作 — 一覧・切り替え・新規作成
- [ ] ファイル右クリックコンテキストメニュー（Stage / Unstage / Diff / Discard Changes）
- [ ] ファイル複数選択 (Ctrl+Click, Shift+Click) 対応
- [ ] 簡易 Diff ビューア（変更差分の色付き表示）

### Phase 3: 情報表示
- [ ] リポジトリステータス表示（変更ファイル一覧）
- [ ] コミットログ表示
- [ ] コミットグラフ表示（ブランチの分岐・マージを可視化）
- [ ] コミットログ検索・フィルター
- [ ] ブランチ一覧表示（ローカル＋リモート）

---

## Rust バックエンド コマンド設計

```rust
// === プロジェクト管理 ===
#[tauri::command] fn add_project(path: String) -> Result<Project, String>
#[tauri::command] fn remove_project(path: String) -> Result<(), String>
#[tauri::command] fn list_projects() -> Result<Vec<Project>, String>
#[tauri::command] fn open_folder_dialog() -> Result<Option<String>, String>

// === Git 操作 ===
#[tauri::command] fn git_clone(url: String, path: String) -> Result<(), String>
#[tauri::command] fn git_fetch(path: String) -> Result<(), String>
#[tauri::command] fn git_pull(path: String) -> Result<(), String>
#[tauri::command] fn git_push(path: String) -> Result<(), String>
#[tauri::command] fn git_commit(path: String, message: String) -> Result<(), String>
#[tauri::command] fn git_commit_amend(path: String, message: String) -> Result<(), String>
#[tauri::command] fn git_stage_files(path: String, files: Vec<String>) -> Result<(), String>
#[tauri::command] fn git_unstage_files(path: String, files: Vec<String>) -> Result<(), String>
#[tauri::command] fn git_stage_all(path: String) -> Result<(), String>
#[tauri::command] fn git_discard_changes(path: String, files: Vec<String>) -> Result<(), String>
#[tauri::command] fn git_stash(path: String, message: Option<String>) -> Result<(), String>
#[tauri::command] fn git_stash_pop(path: String) -> Result<(), String>
#[tauri::command] fn git_stash_list(path: String) -> Result<Vec<StashEntry>, String>
#[tauri::command] fn git_diff_file(path: String, file: String) -> Result<DiffResult, String>

// === ブランチ操作 ===
#[tauri::command] fn git_list_branches(path: String) -> Result<BranchInfo, String>
#[tauri::command] fn git_checkout_branch(path: String, branch: String) -> Result<(), String>
#[tauri::command] fn git_create_branch(path: String, branch: String) -> Result<(), String>

// === 情報取得 ===
#[tauri::command] fn git_status(path: String) -> Result<RepoStatus, String>
#[tauri::command] fn git_log(path: String, count: u32) -> Result<Vec<CommitInfo>, String>
#[tauri::command] fn git_log_graph(path: String, count: u32) -> Result<Vec<GraphCommit>, String>

// === 外部アプリ連携 ===
#[tauri::command] fn open_in_explorer(path: String) -> Result<(), String>
#[tauri::command] fn open_in_terminal(path: String) -> Result<(), String>
```

---

## データ構造

```rust
#[derive(Serialize, Deserialize, Clone)]
struct Project {
    name: String,
    path: String,
}

#[derive(Serialize, Clone)]
struct RepoStatus {
    branch: String,
    changed_files: Vec<FileStatus>,
    ahead: u32,
    behind: u32,
}

#[derive(Serialize, Clone)]
struct FileStatus {
    path: String,
    status: String,  // "modified", "new", "deleted", "renamed"
    staged: bool,
}

#[derive(Serialize, Clone)]
struct CommitInfo {
    hash: String,
    short_hash: String,
    message: String,
    author: String,
    date: String,
}

#[derive(Serialize, Clone)]
struct GraphCommit {
    hash: String,
    short_hash: String,
    message: String,
    author: String,
    date: String,
    parents: Vec<String>,         // 親コミットのハッシュ
    graph_column: u32,            // グラフ上の列位置
    graph_lines: Vec<GraphLine>,  // 描画すべきグラフの線分
    refs: Vec<String>,            // ブランチ/タグ参照
}

#[derive(Serialize, Clone)]
struct GraphLine {
    from_col: u32,
    to_col: u32,
    color: String,                // 線の色 (ブランチごとに色分け)
    line_type: String,            // "commit", "merge", "branch", "pass-through"
}

#[derive(Serialize, Clone)]
struct BranchInfo {
    current: String,
    local: Vec<String>,
    remote: Vec<String>,
}

#[derive(Serialize, Clone)]
struct StashEntry {
    index: u32,
    message: String,
    date: String,
}

#[derive(Serialize, Clone)]
struct DiffResult {
    file_path: String,
    hunks: Vec<DiffHunk>,
}

#[derive(Serialize, Clone)]
struct DiffHunk {
    header: String,
    lines: Vec<DiffLine>,
}

#[derive(Serialize, Clone)]
struct DiffLine {
    line_type: String,  // "add", "delete", "context"
    content: String,
    old_lineno: Option<u32>,
    new_lineno: Option<u32>,
}
```

---

## ディレクトリ構成

```
f:\temp\GitGUI\
├── DEVELOPMENT_PLAN.md       # この開発計画
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/
│   │   └── default.json
│   └── src/
│       ├── main.rs           # エントリポイント
│       ├── lib.rs            # Tauri セットアップ
│       ├── commands/
│       │   ├── mod.rs
│       │   ├── git_ops.rs    # Git 操作コマンド (clone/fetch/pull/commit)
│       │   ├── branch.rs     # ブランチ操作
│       │   └── project.rs    # プロジェクト管理
│       └── models.rs         # データ構造体
├── src/
│   ├── index.html            # メイン HTML (Vite エントリ)
│   ├── styles.css            # スタイルシート (ダークテーマ)
│   ├── types.ts              # Rust models.rs に対応する型定義
│   ├── tabs.ts               # タブ管理 (TypeScript)
│   └── main.ts               # メインロジック (TypeScript)
├── tsconfig.json             # TypeScript 設定
├── tsconfig.node.json        # Vite 用 TypeScript 設定
├── vite.config.ts            # Vite 設定
└── package.json              # npm 依存 (vite, typescript, @tauri-apps/api)
```

---

## UI デザイン方針

- **ダークテーマ** (VS Code 風の配色)
- 上部に **タブバー** — プロジェクトごとのタブ＋「+」ボタン
- タブ下に **ツールバー** — アクションボタン群 (Fetch, Pull, Push, Stash) + ユーティリティ (エクスプローラー/ターミナルで開く)
- **「＋」タブページ** — 新しいタブを開いた時に表示されるランディングページ
  - 「リポジトリをクローン」カード — URL 指定で Clone
  - 「既存のフォルダを追加」カード — フォルダ選択ダイアログでローカルリポジトリを追加
  - 「最近閉じたプロジェクト」リスト — ワンクリックで再追加
  - D&D でもリポジトリ追加可能
- メインエリア左に **変更ファイル一覧**（チェックボックス付き）
  - ファイル **右クリック** でコンテキストメニュー（Stage / Unstage / Diff / Discard Changes）
  - **Ctrl+Click** で個別複数選択、**Shift+Click** で範囲選択
  - 複数選択時の右クリックは選択ファイル全体に一括適用
- メインエリア右に **コミットログ**
  - **コミットグラフ**（SVG描画）でブランチの分岐・マージを視覚化
  - ブランチごとに色分けされた線でフローを表示
  - コミットログ上部に**検索バー**（メッセージ・作者名でフィルター）
- ファイルダブルクリック or 右クリック「Diff」で **差分ビューア** をモーダル表示
- **ドラッグ&ドロップ** でフォルダをウィンドウに放り込んでリポジトリ追加
- **タブのドラッグ** で並び替え可能
- 下部に **ステータスバー**（現在のブランチ、ahead/behind 表示）

---

## 環境情報

| 項目 | バージョン |
|---|---|
| Rust (rustc) | 1.91.1 |
| Cargo | 1.91.1 |
| Node.js | 22.21.1 |
| npm | 10.9.4 |
| Git | 2.52.0 |
| Rust Target | x86_64-pc-windows-msvc |
| OS | Windows |

---

## 備考

- Tauri v2 を使用（v1 ではなく最新の v2 系）
- `git2` crate をメインに使用し、credentials 処理が必要な操作(clone/fetch/pull)は `git` CLI にフォールバック
- プロジェクト設定は `AppData` 配下に JSON で永続化

---

## Phase 5: バグ修正 & 改善（ユーザーテストフィードバック + コード監査 2026-02-28）

実際にGUIを操作してのテスト結果 + ソースコード全体の精査で発見された修正項目。

---

### 5.0 CSS クラス名不一致 — 全ポップアップ/モーダルが動作しない 【バグ・致命的・根本原因】

**発見:** コード監査で特定。Phase 5 の複数バグ (5.1, 5.3, 5.5, 5.6) の元凶。

| # | 問題 | 根本原因 | 影響範囲 | 対応 |
|---|---|---|---|---|
| 5.0.1 | JS 側は全ポップアップに `.classList.add("visible")` を使用しているが、CSS 側は `.show` クラスで表示制御している | **クラス名の不一致**: JS=`visible` / CSS=`show` | コンテキストメニュー、ブランチDD、StashDD、Cloneモーダル、新規ブランチモーダル、Diffモーダル、D&Dオーバーレイ (**7箇所**) | CSS の `.show` ルールを `.visible` に統一する（Loading overlay は既に `.visible` で正常動作中） |

**不一致の詳細一覧:**

| 要素 | CSS セレクタ (現状) | JS で使用するクラス | 動作 |
|---|---|---|---|
| コンテキストメニュー | `.context-menu.show` | `visible` | ❌ 表示されない |
| ブランチドロップダウン | `.branch-dropdown.show` | `visible` | ❌ 表示されない |
| Stash ドロップダウン | `.stash-dropdown.show` | `visible` | ❌ 表示されない |
| Clone モーダル | `.modal-overlay.show` | `visible` | ❌ 表示されない |
| 新規ブランチモーダル | `.modal-overlay.show` | `visible` | ❌ 表示されない |
| Diff モーダル (親) | `.modal-overlay.show` | `visible` | ❌ 表示されない |
| D&D オーバーレイ | `.drop-overlay.show` | `visible` | ❌ 表示されない |
| ローディングオーバーレイ | `.loading-overlay.visible` | `visible` | ✅ 正常 |
| トースト通知 | `.toast.show` | `show` | ✅ 正常 |

**修正方法:** `styles.css` の以下 CSS ルールを `.show` → `.visible` に変更:
```css
.context-menu.show → .context-menu.visible
.branch-dropdown.show → .branch-dropdown.visible
.stash-dropdown.show → .stash-dropdown.visible
.modal-overlay.show → .modal-overlay.visible
.drop-overlay.show → .drop-overlay.visible
```

---

### 5.1 ステージング＆コミット不可 【バグ・致命的】

| # | 問題 | 根本原因 (確定) | 対応 |
|---|---|---|---|
| 5.1.1 | 右クリック → Stage が機能しない | **5.0 の CSS クラス不一致**により、コンテキストメニューが `display: none` のまま表示されない | 5.0 を修正すれば解決 |
| 5.1.2 | 「Stage All」ボタンと「Commit」ボタンは正常に動作する | `setupCommitArea()` のイベントハンドラは正しく登録されている。invoke パラメータも正しい | 修正不要（ただし UX 上、右クリックが使えないと気付きにくい） |

> **補足:** `git_stage_files` の invoke 呼び出し `{ path, files }` とRust側の `path: String, files: Vec<String>` は一致。ステージング自体のロジックには問題なし。

---

### 5.2 CLI実行時のコマンドプロンプト表示 【バグ・UX】

| # | 問題 | 根本原因 (確定) | 対応 |
|---|---|---|---|
| 5.2.1 | Fetch / Pull / Push / Clone を押すとコマンドプロンプト窓が一瞬表示される | `git_ops.rs` の `Command::new("git")` に `.creation_flags(0x08000000)` (`CREATE_NO_WINDOW`) が付与されていない (Windows 固有) | 全 4 箇所の `Command::new("git")` に追加 |
| 5.2.2 | `open_in_terminal` の中間 `cmd /c` 呼び出しも一瞬窓が出る可能性 | `Command::new("cmd")` にも同フラグがない | ターミナル起動コマンドにもフラグ追加（ただし最終的に cmd 窓を開くのが目的なので、spawn の中間プロセスのみ非表示にする） |

**修正方法:**
```rust
// git_ops.rs 冒頭に追加
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

// 各 Command::new("git") に追加
let mut cmd = Command::new("git");
#[cfg(target_os = "windows")]
cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
```

**対象箇所 (`git_ops.rs`):**
- `git_clone` (L122付近)
- `git_fetch` (L148付近)
- `git_pull` (L163付近)
- `git_push` (L187付近)

---

### 5.3 ブランチセレクターで切り替えできない 【バグ】

| # | 問題 | 根本原因 (確定) | 対応 |
|---|---|---|---|
| 5.3.1 | ブランチドロップダウンが開かない | **5.0 の CSS クラス不一致**により `.branch-dropdown` が表示されない | 5.0 を修正すれば解決 |
| 5.3.2 | (参考) ドロップダウン項目のクリックハンドラは正常 | `loadBranches()` で各項目に `e.stopPropagation()` + `checkoutBranch()` 呼び出しが正しく設定されている | 修正不要 |

> **補足:** `git_checkout_branch` の invoke パラメータ `{ path, branchName: name }` は Tauri v2 の自動 camelCase→snake_case 変換により `branch_name` として正しく伝達される。

---

### 5.4 コミット詳細表示 【新機能】

| # | 要望 | 対応 |
|---|---|---|
| 5.4.1 | コミットログのダブルクリックで、そのコミットの詳細を見たい | コミット詳細モーダルを新規作成 |
| 5.4.2 | どのファイルを変更したかが見えるとうれしい | `git_commit_detail` コマンドを新規作成し、変更ファイル一覧（追加/変更/削除）を返す |
| 5.4.3 | コミット詳細モーダル UI | Hash, メッセージ全文, 作者, 日時, 変更ファイル一覧を表示するモーダル |

**実装概要:**
- **Rust 側:** `git_commit_detail(path: String, hash: String) -> Result<CommitDetail, String>` を新規作成。`CommitDetail` は `CommitInfo` + `changed_files: Vec<FileStatus>` を含む
- **HTML:** 新規モーダルを `index.html` に追加
- **CSS:** `.commit-detail-modal` スタイル追加
- **JS:** `createLogRow()` に `dblclick` イベントハンドラ追加 → `showCommitDetail(hash)` を呼び出し

---

### 5.5 新規タブページでのD&Dが動作しない 【バグ】

| # | 問題 | 根本原因 (確定) | 対応 |
|---|---|---|---|
| 5.5.1 | D&D オーバーレイが表示されない | **5.0 の CSS クラス不一致**により `.drop-overlay` が表示されない | 5.0 を修正すれば視覚フィードバックが復活 |
| 5.5.2 | D&D でファイルパスが取得できない | `(file as any).path` は Tauri v2 の WebView (WebView2/Chromium) では利用不可。`file.name` にフォールバックするが、これはフォルダ名のみで完全パスではないため `addProject()` が失敗する | Tauri v2 のネイティブ `drag-drop` イベント API に切り替え |

**修正方法:**
```typescript
// main.ts — setupDragAndDrop() を Tauri ネイティブイベントに置き換え
import { getCurrentWindow } from '@tauri-apps/api/window';

function setupDragAndDrop(): void {
  const overlay = $("drop-overlay");
  const appWindow = getCurrentWindow();

  appWindow.onDragDropEvent((event) => {
    if (event.payload.type === 'enter') {
      overlay.classList.add("visible");
    } else if (event.payload.type === 'leave') {
      overlay.classList.remove("visible");
    } else if (event.payload.type === 'drop') {
      overlay.classList.remove("visible");
      for (const path of event.payload.paths) {
        tabManager.addProject(path)
          .then(() => showToast("プロジェクトを追加しました", "success"))
          .catch((err) => showToast("" + err, "error"));
      }
    }
  });
}
```

---

### 5.6 Diff ビューアが開かない 【バグ】

| # | 問題 | 根本原因 (確定) | 対応 |
|---|---|---|---|
| 5.6.1 | Diff モーダルが表示されない | **5.0 の CSS クラス不一致**により `.modal-overlay` (`#diff-modal`) が表示されない | 5.0 を修正すれば解決 |
| 5.6.2 | (参考) invoke パラメータは正常 | `invoke("git_diff_file", { path, filePath, staged })` の `filePath` は Tauri v2 が自動的に `file_path` に変換。Rust 側 `fn git_diff_file(path: String, file_path: String, staged: bool)` と一致 | 修正不要 |
| 5.6.3 | (参考) 右クリック→Diff は 5.0 修正後に動作 | コンテキストメニューの CSS 修正が先に必要 | 5.0 を修正すれば解決 |
| 5.6.4 | (参考) ファイルダブルクリック→Diff も 5.0 修正後に動作 | `createFileItem()` の `dblclick` ハンドラで `showDiff()` が呼ばれる。モーダルの表示だけが問題 | 5.0 を修正すれば解決 |

---

### 5.7 追加発見事項（コード監査） 【改善】

| # | 問題 | 詳細 | 優先度 | 対応 |
|---|---|---|---|---|
| 5.7.1 | リモートブランチの checkout 未対応 | `git_checkout_branch` が `refs/heads/` のみ解決。リモートブランチ (例: `origin/feature`) を選択しても追跡ブランチが作成されない | P2 | リモートブランチ検出 → ローカル追跡ブランチ自動作成ロジックを追加 |
| 5.7.2 | `git_log` コマンドが未使用 (デッドコード) | フロントエンドは `git_log_graph` のみ使用。`git_log` は `lib.rs` に登録済みだが呼ばれない | P3 | 将来使う可能性があるため保留 or 削除 |
| 5.7.3 | StashEntry に `date` フィールドがない | 開発計画では `date: String` を含むが、models.rs / types.ts 両方で省略 | P3 | Stash 一覧の日時表示が欲しければ追加 |
| 5.7.4 | 開発計画のデータ構造設計と実装の乖離 | `DiffLine.line_type`→`kind`, `GraphCommit.graph_column`→`column`, `RepoStatus.changed_files`→`staged/unstaged` 分離, `git_diff_file` に `staged` パラメータ追加 など | P3 | 実装の方が良い設計。開発計画ドキュメントの更新のみ |

---

### Phase 5 対応優先度（更新版）

| 優先度 | 項目 | 理由 | 修正工数 |
|---|---|---|---|
| **P0 (最優先)** | 5.0 CSS クラス名不一致 | **5.1, 5.3, 5.5(一部), 5.6 の根本原因**。CSS 5行の変更で 4件のバグが解決 | 5分 |
| **P1 (高)** | 5.2 コマンドプロンプト表示 | UX上非常に目立つ問題 | 15分 |
| **P1 (高)** | 5.5.2 D&D ファイルパス取得 | Tauri ネイティブイベントへの切り替えが必要 | 20分 |
| **P2 (中)** | 5.4 コミット詳細表示 | 新機能追加 | 60分 |
| **P2 (中)** | 5.7.1 リモートブランチ checkout | 機能拡張 | 20分 |
| **P3 (低)** | 5.7.2〜5.7.4 | コード整理・ドキュメント更新 | 15分 |

---

## Phase 6: 追加改善（ユーザーフィードバック 2026-02-28）

### 6.1 変更ファイルをOS既定アプリで開く 【新機能・30分】

**要望:** 変更ファイル一覧のファイルを右クリック → 「ファイルを開く」で、OS の既定アプリケーションで開きたい（例: PDF、画像など。Diff ではなく現在のファイルそのものを見たい）。

**実装方針:**
- **Rust 側:** 新コマンド `open_file_default(path: String, file_path: String)` を追加
  - `path`（リポジトリルート）+ `file_path`（相対パス）を結合してフルパスを構成
  - Windows: `Command::new("cmd").args(["/c", "start", "", &full_path])` でOS既定アプリを起動
  - `CREATE_NO_WINDOW` フラグは中間プロセスに必要
- **HTML:** コンテキストメニュー `#context-menu` に新しい項目を追加
  ```html
  <div class="cm-item" data-action="open-file">
    <span class="cm-icon">🔗</span> ファイルを開く
  </div>
  ```
- **JS:** `handleContextAction()` に `case "open-file"` 分岐を追加。選択ファイルのうち最初の1つを `invoke("open_file_default", { path, filePath })` で開く

**対象ファイル:** `git_ops.rs`, `lib.rs`, `index.html`, `main.ts`

---

### 6.2 コミットログのファイル名検索 【新機能・45分】

**要望:** 現在のコミットログ検索はメッセージと作者名のみ対応。特定のファイルを変更したコミットを検索したい。

**実装方針:**
- **Rust 側:** 新コマンド `git_log_search_by_file(path: String, pattern: String, count: Option<usize>)` を追加
  - git CLI `git log --all --format="%H" -- "*{pattern}*"` で、指定パターンにマッチするファイルを変更したコミットハッシュ一覧を取得
  - `CREATE_NO_WINDOW` フラグ付き（Windows）
  - 結果は `Vec<String>`（ハッシュ一覧）で返す
- **HTML:** 検索バーに検索モードトグルを追加（メッセージ検索 / ファイル検索 の切り替え）
  ```html
  <div class="log-search">
    <span>🔍</span>
    <input type="text" id="log-search-input" placeholder="コミットメッセージまたは作者名で検索...">
    <button id="log-search-mode" class="search-mode-btn" title="検索モード切替">💬</button>
  </div>
  ```
  - 💬 = メッセージ検索モード（デフォルト）
  - 📄 = ファイル検索モード
- **JS:** 
  - `setupLogSearch()` にモード管理を追加
  - メッセージモード: 現在のフロントエンドフィルター（既存動作）
  - ファイルモード: `invoke("git_log_search_by_file")` → 返ってきたハッシュ一覧で `log-row` をフィルター
  - デバウンス（300ms）を入れてAPI呼び出し頻度を制御
- **CSS:** 検索モードボタンのスタイル追加

**対象ファイル:** `git_ops.rs`, `lib.rs`, `index.html`, `styles.css`, `main.ts`

---

### 6.3 リモートURLをブラウザで開く 【新機能・30分】

**要望:** リモートブランチがある場合、そのリポジトリのURLをブラウザで開くボタンが欲しい。リモートがない場合は動作しなくてよい。

**実装方針:**
- **Rust 側:** 新コマンド `get_remote_url(path: String) -> Result<Option<String>, String>` を追加
  - `repo.find_remote("origin")` → `remote.url()` でリモートURL取得
  - SSH形式のURLをHTTPSに変換: `git@github.com:user/repo.git` → `https://github.com/user/repo.git`
  - 末尾の `.git` は除去
  - リモートが存在しない場合は `Ok(None)` を返す
- **Rust 側:** 新コマンド `open_url_in_browser(url: String)` を追加
  - Windows: `Command::new("cmd").args(["/c", "start", "", &url])` でブラウザを起動
- **HTML:** ツールバーにボタン追加
  ```html
  <button class="toolbar-btn" id="btn-remote-url" disabled title="リモートをブラウザで開く">
    <span class="icon">🌐</span> Remote
  </button>
  ```
- **JS:**
  - `refreshStatus()` 時に `invoke("get_remote_url")` でリモートURLの有無を確認し、ボタンの有効/無効を切り替え
  - ボタンクリックで `invoke("open_url_in_browser", { url })` を実行
- **CSS:** 特になし（既存の `.toolbar-btn` スタイルで十分）

**対象ファイル:** `git_ops.rs`, `lib.rs`, `index.html`, `main.ts`

---

### 6.4 非アクティブタブにプル待ちバッジ 【新機能・40分】

**要望:** 非選択のタブに対して、fetch後にプルすべきコミットが存在するとき（`behind > 0`）、変更ありのインジケーターを表示したい。

**実装方針:**
- **Rust 側:** 新コマンド `git_ahead_behind(path: String) -> Result<(usize, usize), String>` を追加（軽量版、`git_status` の `ahead/behind` 部分のみ）
  - 既存の `get_ahead_behind()` ヘルパー関数をラップする形で公開
- **HTML/CSS:**
  - タブ要素にバッジ用 `<span>` を追加（`tabs.ts` の `_createTabElement()` で生成）
  - CSS: `.tab-update-badge` スタイル（小さなドット or 数字バッジ）
  ```css
  .tab-update-badge {
    display: none;
    background: #f48771;
    color: #fff;
    font-size: 10px;
    padding: 0 5px;
    border-radius: 8px;
    margin-left: 4px;
  }
  .tab-update-badge.has-updates {
    display: inline-block;
  }
  ```
- **JS (`tabs.ts`):**
  - `_createTabElement()` にバッジ `<span>` を追加
  - 新メソッド `updateTabBehind(tabId: string, behind: number)` を追加
- **JS (`main.ts`):**
  - `btn-fetch` のクリック後、全非アクティブタブに対して `invoke("git_ahead_behind", { path: tab.path })` を呼び出し
  - `behind > 0` なら `tabManager.updateTabBehind(tab.id, behind)` でバッジ表示
  - `refreshAll()` 時にアクティブタブのバッジをクリア
  - タブ切り替え時にもバッジ更新

**対象ファイル:** `git_ops.rs`, `lib.rs`, `tabs.ts`, `styles.css`, `main.ts`

---

### Phase 6 サマリー

| カテゴリ | タスク数 | 推定時間 | 依存関係 |
|---|---|---|---|
| 6.1 ファイルをOS既定で開く | 4 | 30分 | なし |
| 6.2 ファイル名でログ検索 | 6 | 45分 | なし |
| 6.3 リモートURLをブラウザで | 5 | 30分 | なし |
| 6.4 タブにプル待ちバッジ | 6 | 40分 | なし |
| **合計** | **21** | **約 2.5 時間** | |

### Phase 6 実装順序

```
6.1 (ファイルを開く) ← 最もシンプル、コンテキストメニュー1項目追加
  ├─→ 6.3 (リモートURL) ← ツールバーボタン追加 + URL変換
  ├─→ 6.4 (タブバッジ) ← tabs.ts + main.ts の連携修正
  └─→ 6.2 (ファイル名検索) ← 最も複雑、検索モードUI + バックエンド追加
```

---

## Phase 7: 追加改善2（ユーザーフィードバック 2026-02-28）

### ユーザーからのリクエスト

1. **タブバッジの永続化** — タブ切替時に behind バッジが消失する問題を修正
2. **コミットログに未プル情報を表示** — フェッチ済みだが未プルのリモートコミットをログに表示
3. **設定ダイアログ + 自動フェッチ** — 設定ボタン・ダイアログを追加し、10分毎の自動フェッチ on/off を設定可能にする

---

### 7.1A: タブバッジ永続化（タブ切替対応）

**現状の問題:**
- `TabManager.switchTab()` → `render()` が全タブ DOM を再生成
- `_createTabElement()` で updateBadge が空で作成されるため、非アクティブタブの behind 情報が消失

**実装方針:**

1. `TabData` インターフェースに `behind: number` フィールドを追加（デフォルト 0）
2. `updateTabBehind()` でメモリ内の `tabs[]` にも値を保存
3. `_createTabElement()` で保存済みの behind 値を使ってバッジを初期化
4. タブ切替後の `refreshStatus()` ではアクティブタブ分のみ更新

**変更ファイル:** `src/tabs.ts`

**推定時間:** 15分

---

### 7.1B: コミットログに未プル（フェッチ済み）情報を表示

**現状の問題:**
- `git_log_graph` は `BranchType::Local` + HEAD のみを revwalk 起点にしている
- フェッチ済みだがマージされていないリモートコミットがログに表示されない

**実装方針:**

1. **Rust 側 (`git_ops.rs`):**
   - `git_log_graph` の revwalk にリモートブランチ (`BranchType::Remote`) も追加
   - 現在のブランチの upstream との merge-base を計算
   - upstream から merge-base までのコミットハッシュを HashSet に収集
   - `GraphCommit` に `is_remote_only: bool` フィールドを追加
   - unpulled セットに含まれるコミットに `is_remote_only: true` を設定

2. **モデル (`models.rs`):**
   - `GraphCommit` に `pub is_remote_only: bool` を追加

3. **TypeScript 側 (`types.ts`, `main.ts`):**
   - `GraphCommit` 型に `is_remote_only: boolean` を追加
   - `createLogRow()` で `is_remote_only` が true のとき `.log-row-remote-only` クラスを付与
   - ログ行に「↓ 未プル」インジケーターを表示

4. **CSS (`styles.css`):**
   - `.log-row-remote-only` スタイル（半透明 + 右端に ↓ アイコン等）

**変更ファイル:** `src-tauri/src/commands/git_ops.rs`, `src-tauri/src/models.rs`, `src/types.ts`, `src/main.ts`, `src/styles.css`

**推定時間:** 45分

---

### 7.2: 設定ダイアログ + 自動フェッチ

**実装方針:**

1. **データモデル (`models.rs`):**
   - `AppSettings` 構造体を追加:
     ```rust
     pub struct AppSettings {
         pub auto_fetch_enabled: bool,
         pub auto_fetch_interval_minutes: u32,
     }
     ```
   - `AppData` に `settings: AppSettings` フィールドを追加

2. **Rust コマンド (`project.rs` or 新規 `settings.rs`):**
   - `get_settings` — 現在の設定を返す
   - `save_settings` — 設定を保存

3. **フロントエンド HTML (`index.html`):**
   - ツールバーに ⚙️ Settings ボタンを追加
   - 設定ダイアログモーダルを追加（自動フェッチ on/off トグル、間隔設定）

4. **フロントエンド TS (`main.ts`):**
   - 設定ダイアログの開閉ロジック
   - 自動フェッチ: `setInterval` で定期的に `git_fetch` を全タブに対して実行
   - 設定変更時にタイマーを再設定

5. **CSS (`styles.css`):**
   - 設定ダイアログのスタイル（VS Code テーマに合わせたフォーム）

**変更ファイル:** `src-tauri/src/models.rs`, `src-tauri/src/commands/project.rs`, `src-tauri/src/lib.rs`, `src/index.html`, `src/main.ts`, `src/styles.css`, `src/types.ts`

**推定時間:** 45分

---

### Phase 7 サマリー

| カテゴリ | タスク数 | 推定時間 | 依存関係 |
|---|---|---|---|
| 7.1A タブバッジ永続化 | 3 | 15分 | なし |
| 7.1B コミットログ未プル表示 | 7 | 45分 | なし |
| 7.2 設定ダイアログ + 自動フェッチ | 8 | 45分 | なし |
| **合計** | **18** | **約 1.75 時間** | |

### Phase 7 実装順序

```
7.1A (タブバッジ永続化) ← 最もシンプル、tabs.ts のみ
  ├─→ 7.1B (コミットログ未プル表示) ← バックエンド + フロントエンド連携
  └─→ 7.2 (設定ダイアログ) ← 新規モーダル + 永続化 + タイマー
```

---

## Phase 8: UX 改善（操作安全性・機能対称性 2026-03-31）

> ユーザー操作観点でのレビューに基づく改善。「サラリーマンプログラマーがファイル管理として使う」シナリオで不足していた機能を追加。

### ユーザーからのフィードバック

1. **Push 忘れ防止** — タブバッジに `↓N`（未プル）はあるが、`↑N`（未Push）がない。コミットだけして Push を忘れる事故が最多
2. **タブ誤閉じ防止** — タブの × で即座にプロジェクト削除される。未コミット変更や未Push コミットがあっても確認なし
3. **Unstage All がない** — Stage All はあるのに対になる操作がない。間違えて全ステージした場合に不便
4. **ブランチ削除ができない** — 作成・切替はあるが削除がない。feature ブランチが増え続ける。削除後に `git gc --auto` で最適化も実施

---

### 8.1: タブに Push 待ちコミット数（↑N）を表示 【UX改善・高優先】

**現状の問題:**
- `RepoStatus.ahead` は既に取得・ステータスバーに表示されているが、タブバッジとしては表示されていない
- コミット後にタブを切り替えると Push 忘れに気付けない

**実装方針:**

1. **`tabs.ts`:**
   - `TabData` に `ahead: number` フィールドを追加
   - `_createTabElement()` に `tab-ahead-badge` の `<span>` を追加（紫色 `#c586c0`）
   - 新メソッド `updateTabAhead(tabId, ahead)` を追加

2. **`main.ts`:**
   - `refreshStatus()` で `tabManager.updateTabAhead()` を呼び出し
   - `updateAllTabBehindBadges()` で ahead も同時に取得・更新

3. **`styles.css`:**
   - `.tab-ahead-badge` / `.tab-ahead-badge.has-ahead` スタイル追加

**変更ファイル:** `src/tabs.ts`, `src/main.ts`, `src/styles.css`

---

### 8.2: タブ閉じる時の確認ダイアログ 【安全性・高優先】

**現状の問題:**
- タブの × クリックで `removeTab()` → `invoke("remove_project")` が即座に実行される
- 未コミット変更がある状態でも確認なしにプロジェクトが消える（recent には行くが気付きにくい）

**実装方針:**

1. **`tabs.ts`:**
   - `removeTab()` で `tab.changedFiles > 0 || tab.ahead > 0` を検査
   - 条件に該当する場合、`@tauri-apps/plugin-dialog` の `ask()` で確認ダイアログを表示
   - メッセージ例:「未コミットの変更が 3 ファイルあります。\n未プッシュのコミットが 2 件あります。\nこのタブを閉じますか？」
   - キャンセル時は `return` で何もしない

**変更ファイル:** `src/tabs.ts`

---

### 8.3: Unstage All ボタン 【機能対称性・中優先】

**現状の問題:**
- Stage All ボタンはコミットエリアにあるが、対になる Unstage All がない
- 間違えて全ステージした場合、ファイルを一つずつ右クリック → Unstage するしかない

**実装方針:**

1. **Rust 側 (`git_ops.rs`):**
   - 新コマンド `git_unstage_all(path: String)` を追加
   - `repo.reset(head_tree.as_object(), ResetType::Mixed, None)` で HEAD のツリーにインデックスをリセット

2. **HTML (`index.html`):**
   - Stage All と Commit の間に `Unstage All` ボタンを追加

3. **JS (`main.ts`):**
   - `setupCommitArea()` に Unstage All のクリックハンドラを追加

4. **`lib.rs`:**
   - `git_unstage_all` コマンドを登録

**変更ファイル:** `src-tauri/src/commands/git_ops.rs`, `src-tauri/src/lib.rs`, `src/index.html`, `src/main.ts`

---

### 8.4: ブランチ削除 + git gc 最適化 【機能補完・中優先】

**現状の問題:**
- ブランチの作成・切替はできるが削除ができない
- 長期間使うと不要な feature ブランチが蓄積する

**実装方針:**

1. **Rust 側 (`branch.rs`):**
   - 新コマンド `git_delete_branch(path, branch_name, force)` を追加
   - 現在チェックアウト中のブランチの削除は拒否
   - `force: false` の場合、マージ済みかチェック。未マージならエラーを返す
   - `force: true` の場合、強制削除

2. **Rust 側 (`git_ops.rs`):**
   - 新コマンド `git_gc(path)` を追加
   - `git gc --auto` を CLI で実行（必要な場合のみ自動最適化）

3. **JS (`main.ts`):**
   - `loadBranches()` のブランチ一覧各行に 🗑️ 削除ボタンを追加（ホバーで表示）
   - 現在のブランチには削除ボタンを表示しない
   - 削除時: まず `force: false` で試行 → 未マージエラー時は確認ダイアログ → `force: true` で再試行
   - 削除成功後に `git_gc` を呼び出し

4. **CSS (`styles.css`):**
   - `.dd-delete-btn` スタイル追加（ホバーで opacity 表示）

5. **`lib.rs`:**
   - `git_delete_branch`, `git_gc` コマンドを登録

**変更ファイル:** `src-tauri/src/commands/branch.rs`, `src-tauri/src/commands/git_ops.rs`, `src-tauri/src/lib.rs`, `src/main.ts`, `src/styles.css`

---

### Phase 8 サマリー

| カテゴリ | 推定変更ファイル数 | 依存関係 |
|---|---|---|
| 8.1 タブ ↑N Push待ち表示 | 3 | なし |
| 8.2 タブ閉じる確認ダイアログ | 1 | 8.1（ahead データが必要） |
| 8.3 Unstage All | 4 | なし |
| 8.4 ブランチ削除 + gc | 5 | なし |

### Phase 8 実装順序

```
8.1 (↑N Push待ち表示) ← tabs.ts にデータ追加が先
  └─→ 8.2 (タブ閉じ確認) ← 8.1 の ahead データを利用
8.3 (Unstage All) ← 独立して実施可能
8.4 (ブランチ削除 + gc) ← 独立して実施可能
```

---

### Rust バックエンド コマンド設計（Phase 8 追加分）

```rust
// === Git 操作（追加） ===
#[tauri::command] fn git_unstage_all(path: String) -> Result<(), String>
#[tauri::command] fn git_gc(path: String) -> Result<String, String>

// === ブランチ操作（追加） ===
#[tauri::command] fn git_delete_branch(path: String, branch_name: String, force: bool) -> Result<String, String>
```
