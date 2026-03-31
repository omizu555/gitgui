// ===== tabs.ts — タブ管理ロジック =====

import { invoke } from "@tauri-apps/api/core";
import type { Project } from "./types";

/** タブ表示用データ */
interface TabData {
  id: string;
  name: string;
  path: string;
  order: number;
  ahead: number;
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

  // マウスベース D&D 状態
  private _dragState: {
    tabId: string;
    el: HTMLElement;
    startY: number;
    isDragging: boolean;
    placeholder: HTMLElement | null;
  } | null = null;

  constructor() {
    this.tabBar = document.getElementById("tab-bar")!;
    this.addBtn = document.getElementById("btn-add-tab")!;
    this.addBtn.addEventListener("click", () => this.switchToNewTab());

    // タブバー全体の右クリックメニュー抑制
    this.tabBar.addEventListener("contextmenu", (e) => e.preventDefault());
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
        ahead: 0,
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
      ahead: 0,
      behind: 0,
      changedFiles: 0,
    });
    this.render();
    this.switchTab(project.id);
    return project;
  }

  /** タブを閉じる（プロジェクト削除） — 未コミット変更がある場合は確認 */
  async removeTab(id: string): Promise<void> {
    try {
      const tab = this.tabs.find((t) => t.id === id);
      if (tab) {
        // 変更ファイルがあるか確認
        const hasChanges = tab.changedFiles > 0 || tab.ahead > 0;
        if (hasChanges) {
          const { ask } = await import("@tauri-apps/plugin-dialog");
          let msg = "";
          if (tab.changedFiles > 0) msg += `未コミットの変更が ${tab.changedFiles} ファイルあります。`;
          if (tab.ahead > 0) msg += `${msg ? "\n" : ""}未プッシュのコミットが ${tab.ahead} 件あります。`;
          msg += "\nこのタブを閉じますか？";
          const confirmed = await ask(msg, { title: "タブを閉じる", kind: "warning" });
          if (!confirmed) return;
        }
      }

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

    // Push待ちバッジ（ahead 数）— メモリの値で初期化
    const aheadBadge = document.createElement("span");
    aheadBadge.className = "tab-ahead-badge";
    aheadBadge.id = `tab-ahead-${tab.id}`;
    if (tab.ahead > 0) {
      aheadBadge.textContent = `↑${tab.ahead}`;
      aheadBadge.classList.add("has-ahead");
    }
    el.appendChild(aheadBadge);

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

    // クリックでタブ切り替え（ドラッグ中はスキップ）
    el.addEventListener("click", () => {
      if (!this._dragState || !this._dragState.isDragging) {
        this.switchTab(tab.id);
      }
    });

    // 右クリックメニュー抑制
    el.addEventListener("contextmenu", (e) => e.preventDefault());

    // === マウスベース D&D: タブ並び替え ===
    el.addEventListener("mousedown", (e) => {
      // 閉じるボタン上では開始しない
      if ((e.target as HTMLElement).classList.contains("tab-close")) return;
      // 左ボタンのみ
      if (e.button !== 0) return;

      this._dragState = {
        tabId: tab.id,
        el,
        startY: e.clientY,
        isDragging: false,
        placeholder: null,
      };
    });

    return el;
  }

  /** マウスベース D&D のグローバルイベントを初期化 */
  initDragListeners(): void {
    document.addEventListener("mousemove", (e) => this._onMouseMove(e));
    document.addEventListener("mouseup", () => this._onMouseUp());
  }

  private _onMouseMove(e: MouseEvent): void {
    if (!this._dragState) return;

    const dy = Math.abs(e.clientY - this._dragState.startY);

    // 5px 以上動いたらドラッグ開始
    if (!this._dragState.isDragging) {
      if (dy < 5) return;
      this._dragState.isDragging = true;
      this._dragState.el.classList.add("dragging");
      document.body.style.cursor = "grabbing";
      document.body.style.userSelect = "none";
    }

    // ドラッグ中: マウス位置からドロップ先タブを判定
    const tabEls = this.tabBar.querySelectorAll<HTMLElement>(".tab:not(.dragging)");
    let targetEl: HTMLElement | null = null;

    for (const t of tabEls) {
      const rect = t.getBoundingClientRect();
      const midY = rect.top + rect.height / 2;
      if (e.clientY < midY) {
        targetEl = t;
        break;
      }
    }

    // ドロップインジケーター更新
    this.tabBar.querySelectorAll(".tab").forEach((t) => t.classList.remove("drag-over"));
    if (targetEl) {
      targetEl.classList.add("drag-over");
    }
  }

  private _onMouseUp(): void {
    if (!this._dragState) return;

    if (this._dragState.isDragging) {
      // ドロップ先を確定
      const tabEls = this.tabBar.querySelectorAll<HTMLElement>(".tab:not(.dragging)");
      const draggedEl = this._dragState.el;
      const draggedRect = draggedEl.getBoundingClientRect();
      const draggedMidY = draggedRect.top + draggedRect.height / 2;

      let targetId: string | null = null;

      // マウスの位置ではなく、ドラッグ中の要素の位置とインジケーターで判定
      for (const t of tabEls) {
        if (t.classList.contains("drag-over")) {
          targetId = t.dataset.tabId || null;
          break;
        }
      }

      // クリーンアップ
      this._dragState.el.classList.remove("dragging");
      this.tabBar.querySelectorAll(".tab").forEach((t) => t.classList.remove("drag-over"));
      document.body.style.cursor = "";
      document.body.style.userSelect = "";

      if (targetId && targetId !== this._dragState.tabId) {
        this._reorderTabs(this._dragState.tabId, targetId);
      }
    }

    this._dragState = null;
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
    // dragIdx < targetIdx の場合、splice でインデックスが 1 ずれるので補正する
    // これにより常に「ターゲットの前」に挿入され、border-top の視覚表示と一致する
    const insertIdx = dragIdx < targetIdx ? targetIdx - 1 : targetIdx;
    this.tabs.splice(insertIdx, 0, moved);
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

  /** タブのPush待ちバッジ（ahead 数）を更新 */
  updateTabAhead(tabId: string, ahead: number): void {
    const tab = this.tabs.find((t) => t.id === tabId);
    if (tab) tab.ahead = ahead;

    const badge = document.getElementById(`tab-ahead-${tabId}`);
    if (badge) {
      if (ahead > 0) {
        badge.textContent = `↑${ahead}`;
        badge.classList.add("has-ahead");
      } else {
        badge.textContent = "";
        badge.classList.remove("has-ahead");
      }
    }
  }
}
