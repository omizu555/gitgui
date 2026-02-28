// ===== tabs.ts — タブ管理ロジック =====

import { invoke } from "@tauri-apps/api/core";
import type { Project } from "./types";

/** タブ表示用データ */
interface TabData {
  id: string;
  name: string;
  path: string;
  order: number;
  behind: number;
  changedFiles: number;
}

/**
 * タブ管理クラス
 * - タブの追加・削除・切り替え
 * - プロジェクト情報とのバインド
 * - D&D による並び替え
 */
export class TabManager {
  tabs: TabData[] = [];
  activeTabId: string | null = null;

  private tabBar: HTMLElement;
  private addBtn: HTMLElement;
  private _dragTabId: string | null = null;
  private _dragOverTabId: string | null = null;

  constructor() {
    this.tabBar = document.getElementById("tab-bar")!;
    this.addBtn = document.getElementById("btn-add-tab")!;
    this.addBtn.addEventListener("click", () => this.switchToNewTab());
  }

  // ========== データ操作 ==========

  /** バックエンドからプロジェクト一覧をロードしてタブを再構築 */
  async loadProjects(): Promise<void> {
    try {
      const projects = await invoke<Project[]>("list_projects");
      this.tabs = projects.map((p) => ({
        id: p.id,
        name: p.name,
        path: p.path,
        order: p.order,
        behind: 0,
        changedFiles: 0,
      }));
      this.render();

      if (this.tabs.length > 0) {
        const valid = this.tabs.find((t) => t.id === this.activeTabId);
        if (!valid) {
          this.switchTab(this.tabs[0].id);
        } else {
          this.switchTab(this.activeTabId!);
        }
      } else {
        this.switchToNewTab();
      }
    } catch (e) {
      console.error("プロジェクトの読み込みに失敗:", e);
      this.switchToNewTab();
    }
  }

  /** プロジェクトを追加してタブを生成 */
  async addProject(path: string): Promise<Project> {
    const project = await invoke<Project>("add_project", { path });
    this.tabs.push({
      id: project.id,
      name: project.name,
      path: project.path,
      order: project.order,
      behind: 0,
      changedFiles: 0,
    });
    this.render();
    this.switchTab(project.id);
    return project;
  }

  /** タブを閉じる（プロジェクト削除） */
  async removeTab(id: string): Promise<void> {
    try {
      await invoke("remove_project", { id });
      this.tabs = this.tabs.filter((t) => t.id !== id);
      this.render();

      if (this.activeTabId === id) {
        if (this.tabs.length > 0) {
          this.switchTab(this.tabs[this.tabs.length - 1].id);
        } else {
          this.switchToNewTab();
        }
      }
    } catch (e) {
      console.error("タブの削除に失敗:", e);
    }
  }

  // ========== 表示切り替え ==========

  /** プロジェクトタブに切り替え */
  switchTab(id: string): void {
    const tab = this.tabs.find((t) => t.id === id);
    if (!tab) return;

    this.activeTabId = id;
    this.render();

    document.getElementById("project-view")!.style.display = "";
    document.getElementById("newtab-view")!.style.display = "none";
    document.getElementById("toolbar")!.style.display = "";

    this._setToolbarEnabled(true);

    // コールバック: メイン側でステータスをリフレッシュ
    if (typeof (window as any).onTabSwitch === "function") {
      (window as any).onTabSwitch(tab);
    }
  }

  /** 新しいタブページに切り替え */
  switchToNewTab(): void {
    this.activeTabId = null;
    this.render();

    document.getElementById("project-view")!.style.display = "none";
    document.getElementById("newtab-view")!.style.display = "";
    document.getElementById("toolbar")!.style.display = "none";

    this._setToolbarEnabled(false);

    if (typeof (window as any).loadRecentProjects === "function") {
      (window as any).loadRecentProjects();
    }
  }

  /** ツールバーのボタン有効/無効 */
  private _setToolbarEnabled(enabled: boolean): void {
    const btns = document.querySelectorAll<HTMLButtonElement>(".toolbar-btn");
    btns.forEach((btn) => {
      // 設定ボタンは常に有効
      if (btn.id === "btn-settings") return;
      btn.disabled = !enabled;
    });
    const branchSel = document.getElementById("branch-selector");
    if (branchSel) branchSel.style.display = enabled ? "" : "none";
  }

  /** 現在のアクティブプロジェクトのパスを取得 */
  getActivePath(): string | null {
    if (!this.activeTabId) return null;
    const tab = this.tabs.find((t) => t.id === this.activeTabId);
    return tab ? tab.path : null;
  }

  /** 現在のアクティブプロジェクトを取得 */
  getActiveProject(): TabData | null {
    if (!this.activeTabId) return null;
    return this.tabs.find((t) => t.id === this.activeTabId) || null;
  }

  // ========== DOM レンダリング ==========

  render(): void {
    const existing = this.tabBar.querySelectorAll(".tab");
    existing.forEach((el) => el.remove());

    this.tabs.forEach((tab) => {
      const el = this._createTabElement(tab);
      this.tabBar.insertBefore(el, this.addBtn);
    });
  }

  private _createTabElement(tab: TabData): HTMLElement {
    const el = document.createElement("div");
    el.className = "tab" + (tab.id === this.activeTabId ? " active" : "");
    el.dataset.tabId = tab.id;
    el.setAttribute("draggable", "true");

    // タブ名
    const nameSpan = document.createElement("span");
    nameSpan.className = "tab-name";
    nameSpan.textContent = tab.name;
    nameSpan.title = tab.path;
    el.appendChild(nameSpan);

    // ブランチバッジ
    const branchSpan = document.createElement("span");
    branchSpan.className = "tab-branch";
    branchSpan.id = `tab-branch-${tab.id}`;
    branchSpan.textContent = "";
    el.appendChild(branchSpan);

    // 変更ファイルバッジ — メモリの値で初期化
    const changesBadge = document.createElement("span");
    changesBadge.className = "tab-changes-badge";
    changesBadge.id = `tab-changes-${tab.id}`;
    if (tab.changedFiles > 0) {
      changesBadge.textContent = `✎${tab.changedFiles}`;
      changesBadge.classList.add("has-changes");
    }
    el.appendChild(changesBadge);

    // 更新バッジ（behind 数）— メモリの値で初期化
    const updateBadge = document.createElement("span");
    updateBadge.className = "tab-update-badge";
    updateBadge.id = `tab-badge-${tab.id}`;
    if (tab.behind > 0) {
      updateBadge.textContent = `↓${tab.behind}`;
      updateBadge.classList.add("has-updates");
    }
    el.appendChild(updateBadge);

    // 閉じるボタン
    const closeBtn = document.createElement("span");
    closeBtn.className = "tab-close";
    closeBtn.textContent = "×";
    closeBtn.title = "閉じる";
    closeBtn.addEventListener("click", (e) => {
      e.stopPropagation();
      this.removeTab(tab.id);
    });
    el.appendChild(closeBtn);

    // クリックでタブ切り替え
    el.addEventListener("click", () => this.switchTab(tab.id));

    // === D&D: タブ並び替え ===
    el.addEventListener("dragstart", (e) => {
      this._dragTabId = tab.id;
      el.classList.add("dragging");
      e.dataTransfer!.effectAllowed = "move";
      e.dataTransfer!.setData("text/plain", tab.id);
    });

    el.addEventListener("dragend", () => {
      el.classList.remove("dragging");
      this._dragTabId = null;
      this._dragOverTabId = null;
      this.tabBar
        .querySelectorAll(".tab")
        .forEach((t) => t.classList.remove("drag-over"));
    });

    el.addEventListener("dragover", (e) => {
      e.preventDefault();
      e.dataTransfer!.dropEffect = "move";
      if (this._dragTabId && this._dragTabId !== tab.id) {
        this._dragOverTabId = tab.id;
        this.tabBar
          .querySelectorAll(".tab")
          .forEach((t) => t.classList.remove("drag-over"));
        el.classList.add("drag-over");
      }
    });

    el.addEventListener("dragleave", () => {
      el.classList.remove("drag-over");
    });

    el.addEventListener("drop", (e) => {
      e.preventDefault();
      el.classList.remove("drag-over");
      if (this._dragTabId && this._dragTabId !== tab.id) {
        this._reorderTabs(this._dragTabId, tab.id);
      }
    });

    return el;
  }

  /** D&D によるタブ並び替え */
  private async _reorderTabs(
    draggedId: string,
    targetId: string
  ): Promise<void> {
    const dragIdx = this.tabs.findIndex((t) => t.id === draggedId);
    const targetIdx = this.tabs.findIndex((t) => t.id === targetId);
    if (dragIdx === -1 || targetIdx === -1) return;

    const [moved] = this.tabs.splice(dragIdx, 1);
    this.tabs.splice(targetIdx, 0, moved);
    this.tabs.forEach((t, i) => (t.order = i));

    this.render();

    try {
      const ids = this.tabs.map((t) => t.id);
      await invoke("reorder_projects", { ids });
    } catch (e) {
      console.error("タブ順序の保存に失敗:", e);
    }
  }

  /** タブのブランチバッジを更新 */
  updateTabBranch(tabId: string, branchName: string): void {
    const badge = document.getElementById(`tab-branch-${tabId}`);
    if (badge) {
      badge.textContent = branchName;
    }
  }

  /** タブの変更ファイルバッジを更新 */
  updateTabChanges(tabId: string, count: number): void {
    const tab = this.tabs.find((t) => t.id === tabId);
    if (tab) tab.changedFiles = count;

    const badge = document.getElementById(`tab-changes-${tabId}`);
    if (badge) {
      if (count > 0) {
        badge.textContent = `✎${count}`;
        badge.classList.add("has-changes");
      } else {
        badge.textContent = "";
        badge.classList.remove("has-changes");
      }
    }
  }

  /** タブの更新バッジ（behind 数）を更新 */
  updateTabBehind(tabId: string, behind: number): void {
    // メモリにも保存（render() 時に復元するため）
    const tab = this.tabs.find((t) => t.id === tabId);
    if (tab) tab.behind = behind;

    const badge = document.getElementById(`tab-badge-${tabId}`);
    if (badge) {
      if (behind > 0) {
        badge.textContent = `↓${behind}`;
        badge.classList.add("has-updates");
      } else {
        badge.textContent = "";
        badge.classList.remove("has-updates");
      }
    }
  }
}
