# GitGUI

Windows 向けのデスクトップ Git クライアントです。  
VS Code ライクなダークテーマの GUI で、日常的な Git 操作をマウスとキーボードの両方で快適に行えます。
個人的につかう機能だけ盛り込んで、AIに作ってもらった

![Rust](https://img.shields.io/badge/Rust-1.91-orange)
![Tauri](https://img.shields.io/badge/Tauri-v2-blue)
![TypeScript](https://img.shields.io/badge/TypeScript-5.9-blue)
![Vite](https://img.shields.io/badge/Vite-7.3-purple)

## 主な機能

- **マルチタブ** — 複数リポジトリを同時に管理（D&D で並べ替え可能）
- **コミットグラフ** — ブランチの分岐・マージを視覚的に表示
- **ファイル操作** — Stage / Unstage / Commit / Amend / Discard をワンクリックで
- **ブランチ管理** — 作成・切替・リモートブランチの自動追跡チェックアウト
- **Stash** — 保存・復元をドロップダウンから操作
- **Diff ビューア** — Staged / Unstaged の差分をハンク単位で表示
- **コミット詳細** — ダブルクリックで変更ファイル一覧を確認
- **検索** — コミットメッセージ / ファイル名でログを絞り込み
- **未プル検出** — フェッチ済み・未マージのコミットをログ上で視覚表示
- **タブバッジ** — 各タブに未プル数（↓N）と変更ファイル数（✎N）を常時表示
- **設定** — 自動フェッチ（5〜60分間隔）の ON/OFF を GUI から切替
- **外部連携** — エクスプローラー / ターミナル / ブラウザ（リモート URL）をワンクリックで起動
- **D&D 対応** — フォルダをウィンドウにドロップしてリポジトリを追加

---

## 開発環境の構築

### 前提条件

| ツール | バージョン | 備考 |
|---|---|---|
| **Rust** | 1.91+ | [rustup.rs](https://rustup.rs/) からインストール |
| **Node.js** | 20+ | npm が同梱されていること |
| **Git** | 2.x | CLI として `git` コマンドが PATH に通っていること |
| **Visual Studio Build Tools** | 2022+ | Rust (MSVC) のリンク用。「C++ によるデスクトップ開発」ワークロードが必要 |

### セットアップ

```bash
# リポジトリをクローン
git clone <repository-url>
cd GitGUI

# フロントエンドの依存をインストール
npm install

# Rust 側は初回ビルド時に自動で依存が解決されます
```

---

## ビルド・実行

### 開発モード（ホットリロード対応）

```bash
cargo tauri dev
```

- Vite の開発サーバー（`localhost:1420`）が起動し、Tauri ウィンドウが開きます
- フロントエンド（TypeScript/CSS）の変更は即時反映されます
- Rust 側の変更は自動で再コンパイルされます

### リリースビルド

```bash
cargo tauri build
```

- 最適化された実行ファイルが以下に生成されます:
  ```
  src-tauri/target/release/git-gui.exe
  ```
- 別途インストーラー（`.msi` / `.exe`）も `src-tauri/target/release/bundle/` に生成されます

### フロントエンドのみビルド

```bash
npm run build
```

---

## プロジェクト構成

```
GitGUI/
├── src/                    # フロントエンド（TypeScript + HTML + CSS）
│   ├── main.ts             # アプリロジック・イベントハンドラー
│   ├── tabs.ts             # タブ管理クラス
│   ├── types.ts            # Rust models に対応する型定義
│   ├── index.html          # メイン HTML（モーダル含む）
│   └── styles.css          # VS Code ダークテーマ準拠のスタイル
├── src-tauri/              # バックエンド（Rust + Tauri v2）
│   ├── src/
│   │   ├── lib.rs          # Tauri アプリ設定・コマンド登録
│   │   ├── models.rs       # データ構造（Serialize/Deserialize）
│   │   └── commands/
│   │       ├── git_ops.rs  # Git 操作コマンド群
│   │       ├── branch.rs   # ブランチ操作コマンド
│   │       └── project.rs  # プロジェクト管理・設定コマンド
│   ├── Cargo.toml          # Rust 依存定義
│   └── tauri.conf.json     # Tauri 設定
├── docs/                   # ドキュメント
│   ├── DEVELOPMENT_PLAN.md # 開発計画
│   ├── TASKS.md            # タスク一覧
│   └── USER_MANUAL.md      # 使用マニュアル
├── package.json            # Node.js 依存定義
├── vite.config.ts          # Vite 設定
└── tsconfig.json           # TypeScript 設定
```

---

## 技術スタック

| レイヤー | 技術 | 用途 |
|---|---|---|
| バックエンド | Rust + Tauri v2 | ウィンドウ管理・IPC・OS連携 |
| Git 操作 | git2 クレート (libgit2) | ステータス取得・Diff・グラフ等 |
| Git CLI | `git` コマンド | Clone / Fetch / Pull / Push（認証対応） |
| フロントエンド | TypeScript + Vite | UI ロジック・ビルド |
| 永続化 | JSON ファイル | プロジェクト一覧・設定の保存 |
| プラグイン | tauri-plugin-dialog, tauri-plugin-shell | OS ダイアログ・シェル連携 |

---

## データ保存先

アプリの設定・プロジェクト情報は以下に JSON 形式で保存されます:

```
%APPDATA%/GitGUI/app_data.json
```

---

## ライセンス

MIT License
