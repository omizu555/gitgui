# GitGUI アーキテクチャ設計書

## 概要

GitGUI は、日常的にファイル管理目的で Git を使うプログラマー向けの Windows デスクトップ Git クライアントです。

**技術スタック:**

| レイヤー | 技術 | 用途 |
|---|---|---|
| **デスクトップフレームワーク** | Tauri v2 (Rust) | ネイティブウィンドウ、ファイルシステム、プロセス管理 |
| **バックエンド** | Rust (git2 crate + git CLI) | Git 操作、データ永続化 |
| **フロントエンド** | TypeScript + Vite | UI ロジック（フレームワークなし、vanilla TS） |
| **UI** | HTML5 + CSS3 | VS Code ダークテーマ風 UI |

---

## ディレクトリ構成

```
GitGUI/
├── src/                          # フロントエンド
│   ├── main.ts                   # メインアプリケーションロジック (~1800行)
│   ├── tabs.ts                   # タブ管理 (TabManager クラス)
│   ├── types.ts                  # TypeScript 型定義 (Rust models と対応)
│   ├── index.html                # UI 構造 (モーダル、ツールバー、パネル)
│   └── styles.css                # VS Code ダークテーマ スタイル
├── src-tauri/                    # バックエンド (Rust)
│   ├── src/
│   │   ├── lib.rs                # Tauri アプリ設定、コマンド登録
│   │   ├── models.rs             # データモデル (serde 対応)
│   │   └── commands/
│   │       ├── git_ops.rs        # Git 操作コマンド (~1130行)
│   │       ├── branch.rs         # ブランチ操作
│   │       └── project.rs        # プロジェクト管理、設定
│   ├── Cargo.toml                # Rust 依存関係
│   └── tauri.conf.json           # Tauri 設定
├── docs/                         # ドキュメント
│   ├── ARCHITECTURE.md           # ← 本ファイル
│   ├── DEVELOPMENT_PLAN.md       # 開発計画 (Phase 1〜8)
│   ├── TASKS.md                  # タスク一覧
│   └── USER_MANUAL.md            # ユーザーマニュアル
├── package.json                  # Node.js 依存関係
├── tsconfig.json                 # TypeScript 設定
└── vite.config.ts                # Vite バンドラー設定
```

---

## アーキテクチャ概要図

```
┌─────────────────────────────────────────────────────────┐
│                   Tauri WebView (Edge/WebView2)          │
│                                                          │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐              │
│  │ index.html│  │ main.ts  │  │ tabs.ts  │              │
│  │ (UI構造)  │  │ (ロジック)│  │(タブ管理) │              │
│  └──────────┘  └────┬─────┘  └────┬─────┘              │
│                      │             │                     │
│                      └──────┬──────┘                     │
│                             │ invoke()                   │
├─────────────────────────────┼────────────────────────────┤
│                             │ Tauri IPC                  │
├─────────────────────────────┼────────────────────────────┤
│                   Rust Backend                           │
│                             │                            │
│  ┌──────────────────────────┼───────────────────────┐   │
│  │              Commands Layer                       │   │
│  │  ┌────────────┐  ┌──────────┐  ┌─────────────┐  │   │
│  │  │ git_ops.rs │  │branch.rs │  │ project.rs  │  │   │
│  │  │  (Git操作)  │  │(ブランチ) │  │(プロジェクト)│  │   │
│  │  └──────┬─────┘  └────┬─────┘  └──────┬──────┘  │   │
│  └─────────┼─────────────┼───────────────┼──────────┘   │
│            │             │               │               │
│  ┌─────────┴─────────────┴───────────────┴──────────┐   │
│  │                  models.rs                        │   │
│  │  (データモデル: serde Serialize/Deserialize)       │   │
│  └──────────────────────────────────────────────────┘   │
│                                                          │
│  ┌──────────────┐        ┌───────────────────┐          │
│  │  git2 crate  │        │   git CLI (spawn) │          │
│  │  (ローカル操作)│        │  (認証が必要な操作) │          │
│  └──────────────┘        └───────────────────┘          │
└─────────────────────────────────────────────────────────┘
```

---

## Git 操作の分離戦略

Git 操作は2つの方法で実行されます:

### git2 crate（ローカル操作）

認証不要のローカル操作は `git2` crate を直接使用。高速で安全。

- `git_status` — リポジトリ状態の取得
- `git_stage_files` / `git_unstage_all` — ステージング操作
- `git_commit` — コミット
- `git_diff_file` — ファイル差分
- `git_log_graph` — コミットログ + グラフ
- `git_branches` / `git_checkout_branch` / `git_create_branch` / `git_delete_branch` — ブランチ操作
- `git_stash_save` / `git_stash_list` / `git_stash_pop` — スタッシュ操作
- `git_commit_detail` — コミット詳細
- `git_reset` — リセット

### git CLI（認証が必要な操作）

ネットワークアクセスが必要な操作はシステムの `git` コマンドを使用。OS の credential manager を活用。

- `git_clone` — クローン
- `git_fetch` — フェッチ
- `git_pull` — プル
- `git_push` — プッシュ
- `git_gc` — ガベージコレクション
- `git_log_search_by_file` — ファイル名でのログ検索

### CLI 実行の共通ヘルパー

```rust
fn run_git_command(args: &[&str], repo_path: &str) -> Result<String, String>
```

- `CREATE_NO_WINDOW` フラグ（Windows）でコマンドプロンプト窓の表示を抑制
- stdout/stderr を別スレッドで読み取り、パイプバッファのデッドロックを防止
- 出力は最後の 64KB のみ保持（巨大リポジトリ対応）

---

## データモデル

### Rust 側 (`models.rs`)

```rust
// アプリケーションデータ（永続化）
struct AppData {
    projects: Vec<ProjectEntry>,     // 登録プロジェクト一覧
    recent_projects: Vec<String>,    // 最近閉じたプロジェクト
    tab_order: Vec<String>,          // タブ順序
    active_tab_id: Option<String>,   // アクティブタブ ID
    settings: AppSettings,           // アプリ設定
}

// アプリ設定
struct AppSettings {
    auto_fetch_enabled: bool,
    auto_fetch_interval_minutes: u32,
}

// リポジトリ状態
struct RepoStatus {
    branch: String,
    staged: Vec<FileStatus>,
    unstaged: Vec<FileStatus>,
    ahead: usize,
    behind: usize,
}

// コミットグラフ
struct GraphCommit {
    hash: String,
    short_hash: String,
    message: String,
    author: String,
    date: String,
    refs: Vec<String>,
    parents: Vec<String>,
    column: usize,
    color: usize,
    merge_lines: Vec<MergeLine>,
    is_remote_only: bool,
}
```

### TypeScript 側 (`types.ts`)

Rust の `models.rs` と 1:1 で対応する型定義。Tauri v2 の IPC では `camelCase ↔ snake_case` の自動変換が行われます。

---

## フロントエンド設計

### TabManager (`tabs.ts`)

タブの状態管理を一元化するクラス。

```typescript
class TabManager {
    tabs: TabData[]           // タブデータ配列
    activeTabId: string       // アクティブタブ ID

    // タブ操作
    addProject(path: string)  // プロジェクト追加
    removeTab(tabId: string)  // タブ削除（確認ダイアログ付き）
    switchTab(tabId: string)  // タブ切替

    // バッジ更新
    updateTabBehind(tabId: string, behind: number)  // ↓N
    updateTabAhead(tabId: string, ahead: number)     // ↑N
    updateTabChanges(tabId: string, count: number)   // ✎N
}
```

**TabData フィールド:**

| フィールド | 型 | 説明 |
|---|---|---|
| `id` | string | 一意な ID (UUID) |
| `path` | string | リポジトリパス |
| `name` | string | 表示名（フォルダ名） |
| `behind` | number | 未プルコミット数 |
| `ahead` | number | 未Push コミット数 |
| `changedFiles` | number | 変更ファイル数 |

### main.ts の主要関数

| 関数 | 役割 |
|---|---|
| `refreshStatus()` | アクティブタブの状態を再取得・UI 更新 |
| `refreshAll()` | ステータス + コミットログ + ブランチ一覧を更新 |
| `updateAllTabBehindBadges()` | 全タブの behind/ahead バッジを更新 |
| `loadBranches()` | ブランチドロップダウンの一覧を更新 |
| `loadCommitLog()` | コミットログ（グラフ付き）を取得・表示 |
| `showDiff()` | Diff モーダルを表示 |
| `showCommitDetail()` | コミット詳細モーダルを表示 |
| `deleteBranch()` | ブランチ削除（マージチェック + gc） |

---

## データ永続化

- **保存先:** `%APPDATA%/GitGUI/app_data.json`
- **タイミング:** プロジェクト追加/削除、タブ操作、設定変更のたびに自動保存
- **フォーマット:** JSON (serde_json)

---

## ビルドとテスト

```bash
# TypeScript 型チェック
npx tsc --noEmit

# Rust コンパイルチェック
cd src-tauri && cargo check

# 開発モード（ホットリロード付き）
cargo tauri dev

# リリースビルド
cargo tauri build
```

---

## Phase 一覧

| Phase | 内容 | 状態 |
|---|---|---|
| Phase 1 | プロジェクト基盤（Tauri セットアップ、UI 骨格） | ✅ 完了 |
| Phase 2 | Git 基本操作（Clone, Fetch, Pull, Push, Commit, Branch, Stash, Diff） | ✅ 完了 |
| Phase 3 | 情報表示（ステータス、コミットログ、グラフ、ブランチ一覧） | ✅ 完了 |
| Phase 4 | 仕上げ・品質向上 | ✅ 完了 |
| Phase 5 | バグ修正・改善（CSS クラス不一致、コマンドプロンプト抑制、D&D、コミット詳細） | ✅ 完了 |
| Phase 6 | 追加改善（ファイルを開く、ファイル名検索、リモートURL、タブバッジ） | ✅ 完了 |
| Phase 7 | 追加改善2（タブバッジ永続化、未プルログ表示、設定ダイアログ + 自動フェッチ） | ⬜ 未着手 |
| Phase 8 | UX改善（↑N Push待ち、タブ閉じ確認、Unstage All、ブランチ削除 + gc） | ✅ 完了 |

詳細は `DEVELOPMENT_PLAN.md` と `TASKS.md` を参照してください。
