// ===== main.ts — アプリ初期化 & イベントハンドラー =====

import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open, ask } from "@tauri-apps/plugin-dialog";
import { TabManager } from "./tabs";
import type {
  RepoStatus,
  CommitInfo,
  BranchInfo,
  StashEntry,
  DiffResult,
  DisplayFile,
  GraphCommit,
  CommitDetail,
  AppSettings,
} from "./types";

// =========================================
//  グローバル変数
// =========================================

let tabManager: TabManager;
let selectedFiles = new Set<string>();
let lastClickedIndex = -1;
let currentFiles: DisplayFile[] = [];
let isOperationRunning = false;
let loadedLogCount = 0;
const LOG_PAGE_SIZE = 200;
let autoFetchTimerId: ReturnType<typeof setInterval> | null = null;

// =========================================
//  DOM ヘルパー
// =========================================

/** 型安全な getElementById */
function $<T extends HTMLElement>(id: string): T {
  return document.getElementById(id) as T;
}

// =========================================
//  アプリ初期化
// =========================================

// =========================================
//  グローバルエラーハンドラ
// =========================================

window.addEventListener("unhandledrejection", (e) => {
  console.error("Unhandled rejection:", e.reason);
  const msg = e.reason instanceof Error ? e.reason.message : String(e.reason);
  showToast("エラー: " + msg, "error");
});

window.addEventListener("error", (e) => {
  console.error("Uncaught error:", e.error);
  showToast("エラー: " + e.message, "error");
});

document.addEventListener("DOMContentLoaded", async () => {
  tabManager = new TabManager();
  await tabManager.loadProjects();

  setupToolbarEvents();
  setupNewTabEvents();
  setupCommitArea();
  setupContextMenu();
  setupCommitContextMenu();
  setupBranchSelector();
  setupStashDropdown();
  setupDiffModal();
  setupCommitDetailModal();
  setupDragAndDrop();
  setupTabResize();
  setupKeyboardShortcuts();
  setupLogSearch();
  setupLogScrollLoad();
  setupWindowFocusRefresh();
  setupSettingsDialog();
  await initAutoFetch();

  // 起動時に全タブのバッジ（behind + 変更ファイル数）を初期化
  await updateAllTabBehindBadges();
});

// =========================================
//  タブ切り替え時コールバック
// =========================================

(window as any).onTabSwitch = async (tab: { path: string }) => {
  selectedFiles.clear();
  lastClickedIndex = -1;
  await refreshAll(tab.path);
};

// =========================================
//  ツールバーイベント
// =========================================

function setupToolbarEvents(): void {
  // ツールバーの右クリックメニュー抑制
  $("toolbar").addEventListener("contextmenu", (e) => e.preventDefault());

  $("btn-fetch").addEventListener("click", async () => {
    const path = tabManager.getActivePath();
    if (!path) return;
    setLoading(true, "Fetch 中...");
    try {
      await invoke("git_fetch", { path });
      showToast("Fetch 完了", "success");
      await refreshAll(path);
      // 全タブの behind を確認してバッジ更新
      await updateAllTabBehindBadges();
    } catch (e) {
      showToast("Fetch 失敗: " + e, "error");
    } finally {
      setLoading(false);
    }
  });

  $("btn-pull").addEventListener("click", async () => {
    const path = tabManager.getActivePath();
    if (!path) return;
    setLoading(true, "Pull 中...");
    try {
      const result = await invoke<string>("git_pull", { path });
      if (result.includes("コンフリクト")) {
        showToast(result, "warning");
      } else {
        showToast("Pull 完了", "success");
      }
      await refreshAll(path);
    } catch (e) {
      showToast("Pull 失敗: " + e, "error");
    } finally {
      setLoading(false);
    }
  });

  $("btn-push").addEventListener("click", async () => {
    const path = tabManager.getActivePath();
    if (!path) return;
    setLoading(true, "Push 中...");
    try {
      await invoke("git_push", { path });
      showToast("Push 完了", "success");
      await refreshAll(path);
    } catch (e) {
      showToast("Push 失敗: " + e, "error");
    } finally {
      setLoading(false);
    }
  });

  $("btn-explorer").addEventListener("click", async () => {
    const path = tabManager.getActivePath();
    if (!path) return;
    try {
      await invoke("open_in_explorer", { path });
    } catch (e) {
      showToast("エクスプローラーの起動に失敗: " + e, "error");
    }
  });

  $("btn-terminal").addEventListener("click", async () => {
    const path = tabManager.getActivePath();
    if (!path) return;
    try {
      await invoke("open_in_terminal", { path });
    } catch (e) {
      showToast("ターミナルの起動に失敗: " + e, "error");
    }
  });

  $("btn-remote-url").addEventListener("click", async () => {
    const path = tabManager.getActivePath();
    if (!path) return;
    try {
      const url = await invoke<string | null>("get_remote_url", { path });
      if (url) {
        await invoke("open_url_in_browser", { url });
      } else {
        showToast("リモートURLが設定されていません", "warning");
      }
    } catch (e) {
      showToast("リモートURLの取得に失敗: " + e, "error");
    }
  });

  // 設定ボタン（常に有効 — disabled 属性なし）
  $("btn-settings").addEventListener("click", () => {
    openSettingsModal();
  });
}

// =========================================
//  新しいタブページ
// =========================================

function setupNewTabEvents(): void {
  $("btn-clone-card").addEventListener("click", () => openCloneModal());
  $("btn-folder-card").addEventListener("click", () => openFolderAndAdd());

  $("btn-clone-cancel").addEventListener("click", closeCloneModal);
  $("btn-clone-ok").addEventListener("click", executeClone);
  $("btn-clone-browse").addEventListener("click", browseCloneDestination);

  $("clone-modal").addEventListener("click", (e) => {
    if (e.target === e.currentTarget) closeCloneModal();
  });
}

function openCloneModal(): void {
  $("clone-modal").classList.add("visible");
  $<HTMLInputElement>("clone-url").value = "";
  $<HTMLInputElement>("clone-dest").value = "";
  $<HTMLInputElement>("clone-url").focus();
}

function closeCloneModal(): void {
  $("clone-modal").classList.remove("visible");
}

async function browseCloneDestination(): Promise<void> {
  try {
    const selected = await open({
      directory: true,
      title: "保存先フォルダを選択",
    });
    if (selected) {
      $<HTMLInputElement>("clone-dest").value = selected;
    }
  } catch (e) {
    showToast("フォルダ選択に失敗: " + e, "error");
  }
}

async function executeClone(): Promise<void> {
  const url = $<HTMLInputElement>("clone-url").value.trim();
  const dest = $<HTMLInputElement>("clone-dest").value.trim();

  if (!url) {
    showToast("リポジトリ URL を入力してください", "error");
    return;
  }
  if (!dest) {
    showToast("保存先フォルダを選択してください", "error");
    return;
  }

  closeCloneModal();
  setLoading(true, "クローン中...");
  try {
    const clonedPath = await invoke<string>("git_clone", { url, dest });
    showToast("クローン完了", "success");
    await tabManager.addProject(clonedPath);
  } catch (e) {
    showToast("クローン失敗: " + e, "error");
  } finally {
    setLoading(false);
  }
}

/** フォルダダイアログを開いてプロジェクト追加 */
async function openFolderAndAdd(): Promise<void> {
  try {
    const selected = await open({
      directory: true,
      title: "Git リポジトリフォルダを選択",
    });
    if (selected) {
      await tabManager.addProject(selected);
      showToast("プロジェクトを追加しました", "success");
    }
  } catch (e) {
    showToast("" + e, "error");
  }
}

/** 最近閉じたプロジェクトを読み込み */
(window as any).loadRecentProjects = async () => {
  try {
    const recents = await invoke<
      { name: string; path: string; branch: string }[]
    >("list_recent_projects");
    const section = $("recent-section");
    const list = $("recent-list");
    list.innerHTML = "";

    if (recents.length === 0) {
      section.style.display = "none";
      return;
    }

    section.style.display = "";
    recents.forEach((r) => {
      const item = document.createElement("div");
      item.className = "recent-item";
      item.innerHTML = `
        <span class="recent-name">${escapeHtml(r.name)}</span>
        <span class="recent-branch">${escapeHtml(r.branch)}</span>
        <span class="recent-path">${escapeHtml(r.path)}</span>
      `;
      item.addEventListener("click", async () => {
        try {
          await tabManager.addProject(r.path);
          showToast("プロジェクトを再追加しました", "success");
        } catch (e) {
          showToast("" + e, "error");
        }
      });
      list.appendChild(item);
    });
  } catch (e) {
    console.error("最近のプロジェクト読み込みに失敗:", e);
  }
};

// =========================================
//  コミットエリア
// =========================================

function setupCommitArea(): void {
  $("btn-stage-all").addEventListener("click", async () => {
    const path = tabManager.getActivePath();
    if (!path) return;
    try {
      await invoke("git_stage_all", { path });
      showToast("全ファイルをステージしました", "success");
      await refreshStatus(path);
    } catch (e) {
      showToast("Stage All 失敗: " + e, "error");
    }
  });

  $("btn-commit").addEventListener("click", () => doCommit());

  $<HTMLInputElement>("amend-check").addEventListener("change", async (e) => {
    const target = e.target as HTMLInputElement;
    if (target.checked) {
      const path = tabManager.getActivePath();
      if (!path) return;
      try {
        const msg = await invoke<string>("git_last_commit_message", { path });
        $<HTMLTextAreaElement>("commit-message").value = msg;
      } catch (err) {
        console.error("直前のコミットメッセージ取得に失敗:", err);
      }
    }
  });
}

// =========================================
//  スクロール追加読み込み
// =========================================

function setupLogScrollLoad(): void {
  const logTable = $("log-table");
  logTable.addEventListener("scroll", async () => {
    if (isOperationRunning) return;
    const { scrollTop, scrollHeight, clientHeight } = logTable;
    // 末端から100px以内でトリガー
    if (scrollHeight - scrollTop - clientHeight < 100) {
      const path = tabManager.getActivePath();
      if (!path) return;
      await appendMoreLogs(path);
    }
  });
}

async function appendMoreLogs(path: string): Promise<void> {
  if (isOperationRunning) return;
  const nextCount = loadedLogCount + LOG_PAGE_SIZE;
  try {
    const graphCommits = await invoke<GraphCommit[]>("git_log_graph", {
      path,
      count: nextCount,
    });
    // 新しい分だけ追加
    if (graphCommits.length > loadedLogCount) {
      const container = $("log-table");
      const newCommits = graphCommits.slice(loadedLogCount);
      loadedLogCount = graphCommits.length;
      $("log-count").textContent = String(loadedLogCount);

      let maxCol = 0;
      graphCommits.forEach((gc) => {
        if (gc.column > maxCol) maxCol = gc.column;
        gc.lines.forEach((l) => {
          if (l.from_col > maxCol) maxCol = l.from_col;
          if (l.to_col > maxCol) maxCol = l.to_col;
        });
      });
      const colSpacing = 16;
      const rowHeight = 32;
      const svgWidth = Math.max(100, (maxCol + 2) * colSpacing);

      newCommits.forEach((gc) => {
        container.appendChild(createLogRow(gc, svgWidth, colSpacing, rowHeight));
      });
    }
  } catch (e) {
    console.error("追加ログ読み込み失敗:", e);
  }
}

// =========================================
//  ウィンドウフォーカス自動更新
// =========================================

function setupWindowFocusRefresh(): void {
  let lastFocusRefresh = 0;
  window.addEventListener("focus", async () => {
    // 5秒以内の再フォーカスは無視
    const now = Date.now();
    if (now - lastFocusRefresh < 5000) return;
    lastFocusRefresh = now;

    const path = tabManager.getActivePath();
    if (path && !isOperationRunning) {
      await refreshAll(path);
    }
  });
}

async function doCommit(): Promise<void> {
  const path = tabManager.getActivePath();
  if (!path) return;

  const message = $<HTMLTextAreaElement>("commit-message").value.trim();
  if (!message) {
    showToast("コミットメッセージを入力してください", "error");
    return;
  }

  const isAmend = $<HTMLInputElement>("amend-check").checked;

  try {
    if (isAmend) {
      await invoke("git_commit_amend", { path, message });
      showToast("Amend コミット完了", "success");
    } else {
      await invoke("git_commit", { path, message });
      showToast("コミット完了", "success");
    }
    $<HTMLTextAreaElement>("commit-message").value = "";
    $<HTMLInputElement>("amend-check").checked = false;
    await refreshAll(path);
  } catch (e) {
    showToast("コミット失敗: " + e, "error");
  }
}

// =========================================
//  ファイル一覧
// =========================================

async function refreshStatus(path: string): Promise<void> {
  try {
    const status = await invoke<RepoStatus>("git_status", { path });

    $("current-branch-name").textContent = status.branch;
    $("status-branch").textContent =
      `${status.branch}  ${status.ahead > 0 ? "↑" + status.ahead : ""} ${status.behind > 0 ? "↓" + status.behind : ""}`.trim();

    if (tabManager.activeTabId) {
      tabManager.updateTabBranch(tabManager.activeTabId, status.branch);
    }

    currentFiles = [];
    status.staged.forEach((f) =>
      currentFiles.push({ ...f, staged: true })
    );
    status.unstaged.forEach((f) =>
      currentFiles.push({ ...f, staged: false })
    );

    renderFileList();
    $("file-count").textContent = String(currentFiles.length);

    // Remote ボタンの有効/無効切り替え
    try {
      const remoteUrl = await invoke<string | null>("get_remote_url", { path });
      const remoteBtn = $<HTMLButtonElement>("btn-remote-url");
      remoteBtn.disabled = !remoteUrl;
    } catch {
      $<HTMLButtonElement>("btn-remote-url").disabled = true;
    }

    // アクティブタブのバッジを更新
    if (tabManager.activeTabId) {
      tabManager.updateTabBehind(tabManager.activeTabId, status.behind);
      tabManager.updateTabChanges(tabManager.activeTabId, currentFiles.length);
    }
  } catch (e) {
    console.error("ステータス取得に失敗:", e);
    showToast("ステータス取得に失敗: " + e, "error");
  }
}

function renderFileList(): void {
  const container = $("file-list");
  container.innerHTML = "";

  if (currentFiles.length === 0) {
    container.innerHTML =
      '<div style="text-align:center;color:#666;padding:32px">変更されたファイルはありません</div>';
    return;
  }

  const staged = currentFiles.filter((f) => f.staged);
  const unstaged = currentFiles.filter((f) => !f.staged);

  if (staged.length > 0) {
    const header = document.createElement("div");
    header.className = "file-section-header staged";
    header.textContent = `Staged (${staged.length})`;
    container.appendChild(header);
    staged.forEach((f) =>
      container.appendChild(createFileItem(f, currentFiles.indexOf(f)))
    );
  }

  if (unstaged.length > 0) {
    const header = document.createElement("div");
    header.className = "file-section-header unstaged";
    header.textContent = `Unstaged (${unstaged.length})`;
    container.appendChild(header);
    unstaged.forEach((f) =>
      container.appendChild(createFileItem(f, currentFiles.indexOf(f)))
    );
  }
}

function createFileItem(file: DisplayFile, index: number): HTMLElement {
  const el = document.createElement("div");
  el.className =
    "file-item" + (selectedFiles.has(file.path) ? " selected" : "");
  el.dataset.index = String(index);
  el.dataset.path = file.path;

  const statusClassMap: Record<string, string> = {
    M: "modified",
    A: "added",
    D: "deleted",
    R: "renamed",
  };
  const statusClass = statusClassMap[file.status] || "";

  el.innerHTML = `
    <span class="file-status-badge ${statusClass}">${escapeHtml(file.status)}</span>
    <span class="file-path">${escapeHtml(file.path)}</span>
    ${file.staged ? '<span class="file-staged-badge">STAGED</span>' : ""}
  `;

  el.addEventListener("click", (e) => handleFileClick(index, e));
  el.addEventListener("dblclick", (e) => {
    e.preventDefault();
    showDiff(file.path);
  });
  el.addEventListener("contextmenu", (e) => {
    e.preventDefault();
    if (!selectedFiles.has(file.path)) {
      selectedFiles.clear();
      selectedFiles.add(file.path);
      lastClickedIndex = index;
      renderFileList();
    }
    showContextMenu(e.clientX, e.clientY);
  });

  return el;
}

function handleFileClick(index: number, event: MouseEvent): void {
  const file = currentFiles[index];
  if (!file) return;

  if (event.ctrlKey || event.metaKey) {
    if (selectedFiles.has(file.path)) {
      selectedFiles.delete(file.path);
    } else {
      selectedFiles.add(file.path);
    }
    lastClickedIndex = index;
  } else if (event.shiftKey && lastClickedIndex >= 0) {
    const start = Math.min(lastClickedIndex, index);
    const end = Math.max(lastClickedIndex, index);
    for (let i = start; i <= end; i++) {
      if (currentFiles[i]) selectedFiles.add(currentFiles[i].path);
    }
  } else {
    selectedFiles.clear();
    selectedFiles.add(file.path);
    lastClickedIndex = index;
  }

  renderFileList();
}

// =========================================
//  コンテキストメニュー
// =========================================

function setupContextMenu(): void {
  const menu = $("context-menu");

  menu.querySelectorAll<HTMLElement>(".cm-item").forEach((item) => {
    item.addEventListener("click", () => {
      const action = item.dataset.action;
      if (action) handleContextAction(action);
      hideContextMenu();
    });
  });

  document.addEventListener("click", () => hideContextMenu());
  document.addEventListener("contextmenu", (e) => {
    if (!(e.target as HTMLElement).closest(".file-item")) {
      hideContextMenu();
    }
  });
}

function showContextMenu(x: number, y: number): void {
  const menu = $("context-menu");
  menu.style.left = x + "px";
  menu.style.top = y + "px";
  menu.classList.add("visible");
  $("cm-count").textContent = String(selectedFiles.size);

  const hasStaged = [...selectedFiles].some((p) =>
    currentFiles.find((f) => f.path === p && f.staged)
  );
  const hasUnstaged = [...selectedFiles].some((p) =>
    currentFiles.find((f) => f.path === p && !f.staged)
  );

  const stageItem = menu.querySelector<HTMLElement>('[data-action="stage"]')!;
  const unstageItem = menu.querySelector<HTMLElement>(
    '[data-action="unstage"]'
  )!;
  stageItem.style.display = hasUnstaged ? "" : "none";
  unstageItem.style.display = hasStaged ? "" : "none";
}

function hideContextMenu(): void {
  $("context-menu").classList.remove("visible");
}

async function handleContextAction(action: string): Promise<void> {
  const path = tabManager.getActivePath();
  if (!path || selectedFiles.size === 0) return;

  const files = [...selectedFiles];

  try {
    switch (action) {
      case "stage":
        await invoke("git_stage_files", { path, files });
        showToast(`${files.length} ファイルをステージしました`, "success");
        await refreshStatus(path);
        break;

      case "unstage":
        await invoke("git_unstage_files", { path, files });
        showToast(
          `${files.length} ファイルをアンステージしました`,
          "success"
        );
        await refreshStatus(path);
        break;

      case "discard": {
        const confirmed = await confirmDialog(
          "変更を破棄しますか？",
          `${files.length} ファイルの変更を元に戻します。この操作は取り消せません。`
        );
        if (confirmed) {
          await invoke("git_discard_changes", { path, files });
          showToast(
            `${files.length} ファイルの変更を破棄しました`,
            "success"
          );
          selectedFiles.clear();
          await refreshStatus(path);
        }
        break;
      }

      case "diff":
        if (files.length > 0) showDiff(files[0]);
        break;

      case "open-file":
        for (const f of files) {
          try {
            await invoke("open_file_default", { path, filePath: f });
          } catch (err) {
            showToast("ファイルを開けません: " + err, "error");
          }
        }
        break;

      case "copy-path": {
        const text = files.join("\n");
        await navigator.clipboard.writeText(text);
        showToast("パスをコピーしました", "success");
        break;
      }
    }
  } catch (e) {
    showToast("操作に失敗: " + e, "error");
  }
}

// =========================================
//  コミットログ コンテキストメニュー
// =========================================

let commitContextHash = "";

function setupCommitContextMenu(): void {
  const menu = $("commit-context-menu");

  menu.querySelectorAll<HTMLElement>(".cm-item").forEach((item) => {
    item.addEventListener("click", () => {
      const action = item.dataset.action;
      if (action) handleCommitContextAction(action);
      hideCommitContextMenu();
    });
  });

  document.addEventListener("click", () => hideCommitContextMenu());
  document.addEventListener("contextmenu", (e) => {
    if (!(e.target as HTMLElement).closest(".log-row")) {
      hideCommitContextMenu();
    }
  });
}

function showCommitContextMenu(x: number, y: number, hash: string, shortHash: string): void {
  hideContextMenu();
  const menu = $("commit-context-menu");
  commitContextHash = hash;
  $("ccm-hash").textContent = shortHash;

  // 画面外にはみ出さないよう補正
  const menuWidth = 280;
  const menuHeight = 300;
  const adjustedX = x + menuWidth > window.innerWidth ? window.innerWidth - menuWidth - 8 : x;
  const adjustedY = y + menuHeight > window.innerHeight ? window.innerHeight - menuHeight - 8 : y;

  menu.style.left = adjustedX + "px";
  menu.style.top = adjustedY + "px";
  menu.classList.add("visible");
}

function hideCommitContextMenu(): void {
  $("commit-context-menu").classList.remove("visible");
}

async function handleCommitContextAction(action: string): Promise<void> {
  const path = tabManager.getActivePath();
  if (!path || !commitContextHash) return;

  const hash = commitContextHash;

  try {
    switch (action) {
      case "show-detail":
        showCommitDetail(hash);
        break;

      case "reset-hard": {
        const confirmed = await confirmDialog(
          "Reset (Hard) を実行しますか？",
          "このコミットまで完全に戻します。それ以降の変更はすべて破棄されます。この操作は取り消せません。"
        );
        if (confirmed) {
          setLoading(true, "Reset 実行中...");
          const result = await invoke<string>("git_reset", {
            path,
            hash,
            mode: "hard",
          });
          showToast(result, "success");
          await refreshAll(path);
          setLoading(false);
        }
        break;
      }

      case "reset-soft": {
        const confirmed = await confirmDialog(
          "Reset (Soft) を実行しますか？",
          "このコミットまでHEADを戻します。それ以降の変更はステージングエリアに維持されます。"
        );
        if (confirmed) {
          setLoading(true, "Reset 実行中...");
          const result = await invoke<string>("git_reset", {
            path,
            hash,
            mode: "soft",
          });
          showToast(result, "success");
          await refreshAll(path);
          setLoading(false);
        }
        break;
      }

      case "copy-hash":
        await navigator.clipboard.writeText(hash);
        showToast("ハッシュをコピーしました", "success");
        break;
    }
  } catch (e) {
    setLoading(false);
    showToast("操作に失敗: " + e, "error");
  }
}

// =========================================
//  ブランチセレクター
// =========================================

function setupBranchSelector(): void {
  const selector = $("branch-selector");
  const dropdown = $("branch-dropdown");

  selector.addEventListener("click", async (e) => {
    e.stopPropagation();
    if (dropdown.classList.contains("visible")) {
      dropdown.classList.remove("visible");
    } else {
      await loadBranches();
      dropdown.classList.add("visible");
    }
  });

  document.addEventListener("click", () =>
    dropdown.classList.remove("visible")
  );

  $("btn-new-branch").addEventListener("click", (e) => {
    e.stopPropagation();
    dropdown.classList.remove("visible");
    openNewBranchModal();
  });

  $("btn-branch-cancel").addEventListener("click", closeNewBranchModal);
  $("btn-branch-ok").addEventListener("click", createBranch);
  $("new-branch-modal").addEventListener("click", (e) => {
    if (e.target === e.currentTarget) closeNewBranchModal();
  });
}

async function loadBranches(): Promise<void> {
  const path = tabManager.getActivePath();
  if (!path) return;

  try {
    const branches = await invoke<BranchInfo[]>("git_list_branches", { path });

    const localDiv = $("local-branches");
    const remoteDiv = $("remote-branches");
    localDiv.innerHTML = "";
    remoteDiv.innerHTML = "";

    branches.forEach((b) => {
      const item = document.createElement("div");
      item.className = "dd-item" + (b.is_current ? " current" : "");
      item.textContent = (b.is_current ? "✓ " : "  ") + b.name;
      item.addEventListener("click", async (e) => {
        e.stopPropagation();
        $("branch-dropdown").classList.remove("visible");
        if (!b.is_current) await checkoutBranch(b.name);
      });

      if (b.is_remote) {
        remoteDiv.appendChild(item);
      } else {
        localDiv.appendChild(item);
      }
    });
  } catch (e) {
    showToast("ブランチ一覧の取得に失敗: " + e, "error");
  }
}

async function checkoutBranch(name: string): Promise<void> {
  const path = tabManager.getActivePath();
  if (!path) return;

  try {
    const status = await invoke<RepoStatus>("git_status", { path });
    const hasChanges = status.staged.length > 0 || status.unstaged.length > 0;
    if (hasChanges) {
      const ok = await confirmDialog(
        "ブランチを切り替えますか？",
        "未コミットの変更があります。ブランチを切り替えると変更が引き継がれます。"
      );
      if (!ok) return;
    }
  } catch {
    /* ignore */
  }

  setLoading(true, "ブランチ切り替え中...");
  try {
    await invoke("git_checkout_branch", { path, branchName: name });
    showToast(`ブランチを ${name} に切り替えました`, "success");
    await refreshAll(path);
  } catch (e) {
    showToast("ブランチ切り替え失敗: " + e, "error");
  } finally {
    setLoading(false);
  }
}

function openNewBranchModal(): void {
  $("new-branch-modal").classList.add("visible");
  $<HTMLInputElement>("new-branch-name").value = "";
  $<HTMLInputElement>("new-branch-name").focus();
}

function closeNewBranchModal(): void {
  $("new-branch-modal").classList.remove("visible");
}

async function createBranch(): Promise<void> {
  const path = tabManager.getActivePath();
  if (!path) return;

  const name = $<HTMLInputElement>("new-branch-name").value.trim();
  if (!name) {
    showToast("ブランチ名を入力してください", "error");
    return;
  }

  closeNewBranchModal();
  try {
    await invoke("git_create_branch", { path, branchName: name, checkout: true });
    showToast(`ブランチ ${name} を作成して切り替えました`, "success");
    await refreshAll(path);
  } catch (e) {
    showToast("ブランチ作成失敗: " + e, "error");
  }
}

// =========================================
//  Stash ドロップダウン
// =========================================

function setupStashDropdown(): void {
  const btn = $("btn-stash");
  const dropdown = $("stash-dropdown");

  btn.addEventListener("click", async (e) => {
    e.stopPropagation();
    if (dropdown.classList.contains("visible")) {
      dropdown.classList.remove("visible");
    } else {
      await loadStashList();
      dropdown.classList.add("visible");
    }
  });

  document.addEventListener("click", () =>
    dropdown.classList.remove("visible")
  );

  $("btn-stash-new").addEventListener("click", async (e) => {
    e.stopPropagation();
    dropdown.classList.remove("visible");
    await doStash();
  });
}

async function loadStashList(): Promise<void> {
  const path = tabManager.getActivePath();
  if (!path) return;

  try {
    const entries = await invoke<StashEntry[]>("git_stash_list", { path });
    const container = $("stash-list");
    container.innerHTML = "";

    if (entries.length === 0) {
      container.innerHTML =
        '<div style="padding:12px;color:#666;text-align:center">Stash がありません</div>';
      return;
    }

    entries.forEach((entry) => {
      const item = document.createElement("div");
      item.className = "sd-item";
      item.innerHTML = `
        <span class="sd-index">stash@{${entry.index}}</span>
        <span class="sd-message">${escapeHtml(entry.message)}</span>
        <button class="sd-pop" title="復元 (Pop)">↩</button>
      `;
      item.querySelector(".sd-pop")!.addEventListener("click", async (e) => {
        e.stopPropagation();
        $("stash-dropdown").classList.remove("visible");
        await doStashPop();
      });
      container.appendChild(item);
    });
  } catch (e) {
    showToast("Stash 一覧の取得に失敗: " + e, "error");
  }
}

async function doStash(): Promise<void> {
  const path = tabManager.getActivePath();
  if (!path) return;

  try {
    await invoke("git_stash", { path });
    showToast("変更をStashに退避しました", "success");
    await refreshAll(path);
  } catch (e) {
    showToast("Stash 失敗: " + e, "error");
  }
}

async function doStashPop(): Promise<void> {
  const path = tabManager.getActivePath();
  if (!path) return;

  try {
    await invoke("git_stash_pop", { path });
    showToast("Stash を復元しました", "success");
    await refreshAll(path);
  } catch (e) {
    showToast("Stash Pop 失敗: " + e, "error");
  }
}

// =========================================
//  Diff モーダル
// =========================================

function setupDiffModal(): void {
  $("btn-diff-close").addEventListener("click", closeDiffModal);
  $("diff-modal").addEventListener("click", (e) => {
    if (e.target === e.currentTarget) closeDiffModal();
  });
}

async function showDiff(filePath: string): Promise<void> {
  const repoPath = tabManager.getActivePath();
  if (!repoPath) return;

  // ファイルが staged かどうかを判定
  const file = currentFiles.find((f) => f.path === filePath);
  const staged = file?.staged ?? false;

  try {
    const diff = await invoke<DiffResult>("git_diff_file", {
      path: repoPath,
      filePath,
      staged,
    });
    $("diff-filename").textContent = "📄 " + diff.file_path;

    const body = $("diff-body");
    body.innerHTML = "";

    if (diff.hunks.length === 0) {
      body.innerHTML =
        '<div style="padding:24px;color:#666;text-align:center">差分がありません（新規ファイルまたはバイナリ）</div>';
    } else {
      diff.hunks.forEach((hunk) => {
        const hunkEl = document.createElement("div");
        hunkEl.className = "diff-hunk";

        const headerEl = document.createElement("div");
        headerEl.className = "diff-hunk-header";
        headerEl.textContent = hunk.header;
        hunkEl.appendChild(headerEl);

        hunk.lines.forEach((line) => {
          const lineEl = document.createElement("div");
          lineEl.className = "diff-line " + line.kind;

          const oldNo = line.old_lineno ?? "";
          const newNo = line.new_lineno ?? "";
          const prefix =
            line.kind === "add" ? "+" : line.kind === "del" ? "-" : " ";

          lineEl.innerHTML = `
            <span class="diff-lineno">${oldNo}</span>
            <span class="diff-lineno">${newNo}</span>
            <span class="diff-prefix">${prefix}</span>
            <span class="diff-content">${escapeHtml(line.content)}</span>
          `;
          hunkEl.appendChild(lineEl);
        });

        body.appendChild(hunkEl);
      });
    }

    $("diff-modal").classList.add("visible");
  } catch (e) {
    showToast("Diff の取得に失敗: " + e, "error");
  }
}

function closeDiffModal(): void {
  $("diff-modal").classList.remove("visible");
}

// =========================================
//  コミット詳細モーダル
// =========================================

function setupCommitDetailModal(): void {
  $('btn-cd-close').addEventListener('click', closeCommitDetailModal);
  $("commit-detail-modal").addEventListener("click", (e) => {
    if (e.target === e.currentTarget) closeCommitDetailModal();
  });

  // ブロックコピーの共通ヘルパー
  function blockCopyFeedback(btn: HTMLButtonElement): void {
    const orig = btn.textContent;
    btn.textContent = "\u2714 コピー済";
    setTimeout(() => { btn.textContent = orig; }, 1000);
  }

  // メタ情報ブロックをコピー
  $("cd-copy-meta").addEventListener("click", () => {
    const text = [
      "Hash: " + $("cd-hash").textContent,
      "作者: " + $("cd-author").textContent,
      "日時: " + $("cd-date").textContent,
      "親コミット: " + $("cd-parents").textContent,
    ].join("\n");
    navigator.clipboard.writeText(text).then(() =>
      blockCopyFeedback($("cd-copy-meta") as HTMLButtonElement)
    );
  });

  // メッセージブロックをコピー
  $("cd-copy-message").addEventListener("click", () => {
    const text = $("cd-message").textContent || "";
    navigator.clipboard.writeText(text).then(() =>
      blockCopyFeedback($("cd-copy-message") as HTMLButtonElement)
    );
  });

  // 変更ファイル一覧ブロックをコピー
  $("cd-copy-files").addEventListener("click", () => {
    const items = document.querySelectorAll("#cd-files .cd-file-item");
    const lines = Array.from(items).map((el) => {
      const status = el.querySelector(".file-status-badge")?.textContent || "";
      const path = el.querySelector(".file-path")?.textContent || "";
      return status + " " + path;
    });
    navigator.clipboard.writeText(lines.join("\n")).then(() =>
      blockCopyFeedback($("cd-copy-files") as HTMLButtonElement)
    );
  });
}

async function showCommitDetail(hash: string): Promise<void> {
  const repoPath = tabManager.getActivePath();
  if (!repoPath) return;

  try {
    const detail = await invoke<CommitDetail>("git_commit_detail", {
      path: repoPath,
      hash,
    });

    $("cd-hash").textContent = detail.hash;
    $("cd-author").textContent = detail.author;
    $("cd-date").textContent = detail.date;
    $("cd-parents").textContent =
      detail.parents.length > 0
        ? detail.parents.map((p) => p.substring(0, 7)).join(", ")
        : "(初回コミット)";
    $("cd-message").textContent = detail.message;
    $("cd-file-count").textContent = String(detail.changed_files.length);

    const filesContainer = $("cd-files");
    filesContainer.innerHTML = "";

    if (detail.changed_files.length === 0) {
      filesContainer.innerHTML =
        '<div style="padding:12px;color:#666;text-align:center">変更ファイルなし</div>';
    } else {
      detail.changed_files.forEach((f) => {
        const statusClassMap: Record<string, string> = {
          M: "modified",
          A: "added",
          D: "deleted",
          R: "renamed",
        };
        const el = document.createElement("div");
        el.className = "cd-file-item";
        el.innerHTML = `
          <span class="file-status-badge ${statusClassMap[f.status] || ""}">${escapeHtml(f.status)}</span>
          <span class="file-path">${escapeHtml(f.path)}</span>
        `;
        filesContainer.appendChild(el);
      });
    }

    $("commit-detail-modal").classList.add("visible");
  } catch (e) {
    showToast("コミット詳細の取得に失敗: " + e, "error");
  }
}

function closeCommitDetailModal(): void {
  $("commit-detail-modal").classList.remove("visible");
}

// =========================================
//  D&D リポジトリ追加
// =========================================

// =========================================
//  タブバーリサイズ
// =========================================

function setupTabResize(): void {
  const handle = $("tab-resize-handle");
  const tabBar = $("tab-bar");
  let startX = 0;
  let startW = 0;

  handle.addEventListener("mousedown", (e: MouseEvent) => {
    e.preventDefault();
    startX = e.clientX;
    startW = tabBar.offsetWidth;
    handle.classList.add("dragging");
    document.body.style.cursor = "col-resize";

    const onMove = (ev: MouseEvent) => {
      const delta = startX - ev.clientX;
      const newW = Math.min(400, Math.max(100, startW + delta));
      tabBar.style.width = newW + "px";
    };
    const onUp = () => {
      handle.classList.remove("dragging");
      document.body.style.cursor = "";
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
    };
    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
  });
}

function setupDragAndDrop(): void {
  const overlay = $("drop-overlay");
  const appWindow = getCurrentWindow();

  // HTML5 の D&D はデフォルト動作を抑止 (Tauri ネイティブを使う)
  document.addEventListener("dragover", (e) => e.preventDefault());
  document.addEventListener("drop", (e) => e.preventDefault());

  appWindow.onDragDropEvent((event) => {
    if (event.payload.type === "enter") {
      overlay.classList.add("visible");
    } else if (event.payload.type === "leave") {
      overlay.classList.remove("visible");
    } else if (event.payload.type === "drop") {
      overlay.classList.remove("visible");
      for (const filePath of event.payload.paths) {
        tabManager.addProject(filePath)
          .then(() => showToast("プロジェクトを追加しました", "success"))
          .catch((err: unknown) => showToast("" + err, "error"));
      }
    }
  });
}

// =========================================
//  キーボードショートカット
// =========================================

function setupKeyboardShortcuts(): void {
  document.addEventListener("keydown", async (e) => {
    if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
      if ($("project-view").style.display !== "none") {
        e.preventDefault();
        await doCommit();
      }
    }

    if (e.key === "F5") {
      e.preventDefault();
      const path = tabManager.getActivePath();
      if (path) {
        await refreshAll(path);
        showToast("更新しました", "success");
      }
    }

    if (e.key === "Escape") {
      hideContextMenu();
      closeDiffModal();
      closeCommitDetailModal();
      closeCloneModal();
      closeNewBranchModal();
    }

    if (e.key === "d" && !e.ctrlKey && !e.altKey && !isInputFocused()) {
      if (selectedFiles.size > 0) showDiff([...selectedFiles][0]);
    }

    if (e.key === "Enter" && !e.ctrlKey && !isInputFocused()) {
      if (selectedFiles.size > 0) await handleContextAction("stage");
    }

    if (e.key === "Delete" && !isInputFocused()) {
      if (selectedFiles.size > 0) await handleContextAction("discard");
    }

    if ((e.ctrlKey || e.metaKey) && e.key === "a" && !isInputFocused()) {
      e.preventDefault();
      currentFiles.forEach((f) => selectedFiles.add(f.path));
      renderFileList();
    }
  });
}

function isInputFocused(): boolean {
  const el = document.activeElement;
  return !!el && (el.tagName === "INPUT" || el.tagName === "TEXTAREA");
}

// =========================================
//  コミットログ
// =========================================

async function refreshLog(path: string): Promise<void> {
  try {
    const graphCommits = await invoke<GraphCommit[]>("git_log_graph", {
      path,
      count: LOG_PAGE_SIZE,
    });
    const container = $("log-table");
    container.innerHTML = "";
    loadedLogCount = graphCommits.length;
    $("log-count").textContent = String(graphCommits.length);

    // グラフの最大列数を計算 (SVG 幅を動的に調整)
    let maxCol = 0;
    graphCommits.forEach((gc) => {
      if (gc.column > maxCol) maxCol = gc.column;
      gc.lines.forEach((l) => {
        if (l.from_col > maxCol) maxCol = l.from_col;
        if (l.to_col > maxCol) maxCol = l.to_col;
      });
    });

    const colSpacing = 16;
    const rowHeight = 32;
    const svgWidth = Math.max(100, (maxCol + 2) * colSpacing);

    // グラフカラムの幅を動的に変更
    const logHeader = document.querySelector<HTMLElement>(".log-header");
    if (logHeader) {
      logHeader.style.gridTemplateColumns = `${svgWidth}px 70px 1fr 140px 100px`;
    }

    graphCommits.forEach((gc) => {
      container.appendChild(createLogRow(gc, svgWidth, colSpacing, rowHeight));
    });
  } catch (e) {
    console.error("コミットログの取得に失敗:", e);
  }
}

/** コミットログの1行を作成 */
function createLogRow(
  gc: GraphCommit,
  svgWidth: number,
  colSpacing: number,
  rowHeight: number
): HTMLElement {
  const nodeRadius = 4;
  const c = gc.commit;
  const row = document.createElement("div");
  row.className = "log-row" + (gc.is_remote_only ? " log-row-remote-only" : "");
  row.style.gridTemplateColumns = `${svgWidth}px 70px 1fr 140px 100px`;
  row.dataset.message = c.message.toLowerCase();
  row.dataset.author = c.author.toLowerCase();
  row.dataset.hash = c.hash;

  // ダブルクリックでコミット詳細を表示
  row.addEventListener("dblclick", () => showCommitDetail(c.hash));

  // 右クリックでコミットコンテキストメニューを表示
  row.addEventListener("contextmenu", (e) => {
    e.preventDefault();
    showCommitContextMenu(e.clientX, e.clientY, c.hash, c.short_hash);
  });

  // --- SVG グラフ描画 ---
  const cx = gc.column * colSpacing + colSpacing / 2;
  const cy = rowHeight / 2;
  let svgContent = "";

  gc.lines.forEach((line) => {
    const x1 = line.from_col * colSpacing + colSpacing / 2;
    const y1 = cy;
    const x2 = line.to_col * colSpacing + colSpacing / 2;
    const y2 = rowHeight;

    if (x1 === x2) {
      svgContent += `<line x1="${x1}" y1="${y1}" x2="${x2}" y2="${y2}" stroke="${escapeHtml(line.color)}" stroke-width="2"/>`;
    } else {
      const midY = (y1 + y2) / 2;
      svgContent += `<path d="M${x1},${y1} C${x1},${midY} ${x2},${midY} ${x2},${y2}" fill="none" stroke="${escapeHtml(line.color)}" stroke-width="2"/>`;
    }
  });

  svgContent += `<circle cx="${cx}" cy="${cy}" r="${nodeRadius}" fill="${escapeHtml(gc.color)}" stroke="#1e1e1e" stroke-width="1.5"/>`;

  const refsBadges = c.refs
    .map((r) => {
      const cls = r.startsWith("origin/")
        ? "ref-remote"
        : r === "HEAD"
          ? "ref-head"
          : "ref-local";
      return `<span class="ref-badge ${cls}">${escapeHtml(r)}</span>`;
    })
    .join("");

  const remoteOnlyBadge = gc.is_remote_only
    ? '<span class=\"ref-badge ref-unpulled\">\u2193 \u672a\u30d7\u30eb</span>'
    : '';

  row.innerHTML = `
    <span class=\"graph-cell\"><svg width=\"${svgWidth}\" height=\"${rowHeight}\" xmlns=\"http://www.w3.org/2000/svg\">${svgContent}</svg></span>
    <span class=\"hash\">${escapeHtml(c.short_hash)}</span>
    <span class=\"message\">${remoteOnlyBadge}${refsBadges}${escapeHtml(c.message)}</span>
    <span class=\"author\">${escapeHtml(c.author)}</span>
    <span class=\"date\">${escapeHtml(c.date)}</span>
  `;
  return row;
}

// =========================================
//  ログ検索
// =========================================

let logSearchMode: "message" | "file" = "message";
let fileSearchDebounceTimer: ReturnType<typeof setTimeout> | null = null;

function setupLogSearch(): void {
  const input = $<HTMLInputElement>("log-search-input");
  const modeBtn = $<HTMLButtonElement>("log-search-mode");

  // モード切り替えボタン
  modeBtn.addEventListener("click", () => {
    if (logSearchMode === "message") {
      logSearchMode = "file";
      modeBtn.textContent = "📄";
      modeBtn.title = "検索モード切替: ファイル名";
      modeBtn.classList.add("file-mode");
      input.placeholder = "変更ファイル名で検索...";
    } else {
      logSearchMode = "message";
      modeBtn.textContent = "💬";
      modeBtn.title = "検索モード切替: メッセージ";
      modeBtn.classList.remove("file-mode");
      input.placeholder = "コミットメッセージまたは作者名で検索...";
    }
    // モード変更時に現在の入力で再検索
    input.value = "";
    const rows = document.querySelectorAll<HTMLElement>("#log-table .log-row");
    rows.forEach((row) => {
      row.classList.remove("log-row-hidden", "log-row-highlight");
    });
  });

  input.addEventListener("input", () => {
    if (logSearchMode === "message") {
      filterLogByMessage(input.value);
    } else {
      // ファイルモード: デバウンス 300ms
      if (fileSearchDebounceTimer) clearTimeout(fileSearchDebounceTimer);
      fileSearchDebounceTimer = setTimeout(() => {
        filterLogByFile(input.value);
      }, 300);
    }
  });
}

function filterLogByMessage(rawQuery: string): void {
  const query = rawQuery.toLowerCase().trim();
  const rows = document.querySelectorAll<HTMLElement>("#log-table .log-row");
  rows.forEach((row) => {
    if (!query) {
      row.classList.remove("log-row-hidden", "log-row-highlight");
      return;
    }
    const msg = row.dataset.message || "";
    const author = row.dataset.author || "";
    if (msg.includes(query) || author.includes(query)) {
      row.classList.remove("log-row-hidden");
      row.classList.add("log-row-highlight");
    } else {
      row.classList.add("log-row-hidden");
      row.classList.remove("log-row-highlight");
    }
  });
}

async function filterLogByFile(rawQuery: string): Promise<void> {
  const query = rawQuery.trim();
  const rows = document.querySelectorAll<HTMLElement>("#log-table .log-row");

  if (!query) {
    rows.forEach((row) => {
      row.classList.remove("log-row-hidden", "log-row-highlight");
    });
    return;
  }

  const path = tabManager.getActivePath();
  if (!path) return;

  try {
    const matchingHashes = await invoke<string[]>("git_log_search_by_file", {
      path,
      pattern: query,
    });
    const hashSet = new Set(matchingHashes);

    rows.forEach((row) => {
      const hash = row.dataset.hash || "";
      if (hashSet.has(hash)) {
        row.classList.remove("log-row-hidden");
        row.classList.add("log-row-highlight");
      } else {
        row.classList.add("log-row-hidden");
        row.classList.remove("log-row-highlight");
      }
    });
  } catch (e) {
    console.error("ファイル検索に失敗:", e);
  }
}

// =========================================
//  タブバッジ更新
// =========================================

async function updateAllTabBehindBadges(): Promise<void> {
  for (const tab of tabManager.tabs) {
    try {
      const [, behind] = await invoke<[number, number]>("git_ahead_behind", {
        path: tab.path,
      });
      tabManager.updateTabBehind(tab.id, behind);
    } catch {
      tabManager.updateTabBehind(tab.id, 0);
    }
    // 非アクティブタブの変更ファイル数も更新
    if (tab.id !== tabManager.activeTabId) {
      try {
        const st = await invoke<RepoStatus>("git_status", { path: tab.path });
        tabManager.updateTabChanges(tab.id, st.staged.length + st.unstaged.length);
      } catch {
        tabManager.updateTabChanges(tab.id, 0);
      }
    }
  }
}

// =========================================
//  全体リフレッシュ
// =========================================

async function refreshAll(path: string): Promise<void> {
  await Promise.all([refreshStatus(path), refreshLog(path)]);
}

// =========================================
//  ユーティリティ
// =========================================

type ToastType = "success" | "error" | "info" | "warning";

function showToast(message: string, type: ToastType = "info"): void {
  const container = $("toast-container");
  const toast = document.createElement("div");
  toast.className = `toast ${type}`;
  toast.textContent = message;
  container.appendChild(toast);

  requestAnimationFrame(() => toast.classList.add("show"));

  setTimeout(() => {
    toast.classList.remove("show");
    setTimeout(() => toast.remove(), 300);
  }, 3000);
}

function setLoading(show: boolean, message = ""): void {
  isOperationRunning = show;
  const statusBranch = $("status-branch") as HTMLElement;
  const overlay = document.getElementById("loading-overlay");

  if (show && message) {
    statusBranch.dataset.prevText = statusBranch.textContent || "";
    statusBranch.textContent = "⏳ " + message;
    // ローディングオーバーレイ表示
    if (overlay) {
      overlay.querySelector(".loading-text")!.textContent = message;
      overlay.classList.add("visible");
    }
    // ツールバーボタンを無効化
    setToolbarButtonsDisabled(true);
  } else if (!show) {
    if (statusBranch.dataset.prevText) {
      statusBranch.textContent = statusBranch.dataset.prevText;
      delete statusBranch.dataset.prevText;
    }
    if (overlay) overlay.classList.remove("visible");
    setToolbarButtonsDisabled(false);
  }
}

/** ツールバーボタンの有効/無効を切り替え */
function setToolbarButtonsDisabled(disabled: boolean): void {
  document.querySelectorAll<HTMLButtonElement>(".toolbar-btn").forEach((btn) => {
    if (btn.id === "btn-settings") return;
    btn.disabled = disabled;
  });
  const commitBtn = document.getElementById("btn-commit");
  const stageAllBtn = document.getElementById("btn-stage-all");
  if (commitBtn) (commitBtn as HTMLButtonElement).style.pointerEvents = disabled ? "none" : "";
  if (stageAllBtn) (stageAllBtn as HTMLButtonElement).style.pointerEvents = disabled ? "none" : "";
}

async function confirmDialog(
  title: string,
  message: string
): Promise<boolean> {
  try {
    return await ask(message, { title, kind: "warning" });
  } catch {
    return confirm(title + "\n" + message);
  }
}

function escapeHtml(str: string): string {
  if (!str) return "";
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

// =========================================
//  設定ダイアログ
// =========================================

function setupSettingsDialog(): void {
  $("btn-settings-cancel").addEventListener("click", closeSettingsModal);
  $("btn-settings-ok").addEventListener("click", saveSettings);

  // モーダル背景クリックで閉じる
  $("settings-modal").addEventListener("click", (e) => {
    if ((e.target as HTMLElement).id === "settings-modal") {
      closeSettingsModal();
    }
  });
}

async function openSettingsModal(): Promise<void> {
  try {
    const settings = await invoke<AppSettings>("get_settings");
    $<HTMLInputElement>("setting-auto-fetch").checked = settings.auto_fetch_enabled;
    $<HTMLSelectElement>("setting-fetch-interval").value = String(settings.auto_fetch_interval_minutes);
  } catch {
    // デフォルト値のまま
  }
  $("settings-modal").classList.add("visible");
}

function closeSettingsModal(): void {
  $("settings-modal").classList.remove("visible");
}

async function saveSettings(): Promise<void> {
  const autoFetchEnabled = $<HTMLInputElement>("setting-auto-fetch").checked;
  const intervalMinutes = parseInt($<HTMLSelectElement>("setting-fetch-interval").value, 10);

  const settings: AppSettings = {
    auto_fetch_enabled: autoFetchEnabled,
    auto_fetch_interval_minutes: intervalMinutes,
  };

  try {
    await invoke("save_settings", { settings });
    showToast("設定を保存しました", "success");
    closeSettingsModal();
    // 自動フェッチタイマーを再設定
    setupAutoFetchTimer(settings);
  } catch (e) {
    showToast("設定の保存に失敗: " + e, "error");
  }
}

// =========================================
//  自動フェッチ
// =========================================

async function initAutoFetch(): Promise<void> {
  try {
    const settings = await invoke<AppSettings>("get_settings");
    setupAutoFetchTimer(settings);
  } catch {
    // 設定読み込み失敗時はスキップ
  }
}

function setupAutoFetchTimer(settings: AppSettings): void {
  // 既存タイマーをクリア
  if (autoFetchTimerId !== null) {
    clearInterval(autoFetchTimerId);
    autoFetchTimerId = null;
  }

  if (!settings.auto_fetch_enabled) return;

  const intervalMs = settings.auto_fetch_interval_minutes * 60 * 1000;
  autoFetchTimerId = setInterval(async () => {
    await autoFetchAll();
  }, intervalMs);
}

async function autoFetchAll(): Promise<void> {
  // 操作中はスキップ
  if (isOperationRunning) return;
  if (tabManager.tabs.length === 0) return;

  for (const tab of tabManager.tabs) {
    try {
      await invoke("git_fetch", { path: tab.path });
    } catch {
      // サイレント失敗
    }
  }

  // 全タブの behind バッジを更新
  await updateAllTabBehindBadges();

  // アクティブタブをリフレッシュ
  const activePath = tabManager.getActivePath();
  if (activePath) {
    await refreshAll(activePath);
  }

  showToast("自動フェッチ完了", "info");
}
