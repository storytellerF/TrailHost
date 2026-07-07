export interface SyncBadgeState {
  pendingCount: number;
  syncing: boolean;
}

const IDLE_TITLE = "TrailHost";
const PENDING_COLOR = "#d97706";
const SYNCING_COLOR = "#2563eb";

export function getSyncBadgeText(state: SyncBadgeState): string {
  if (state.pendingCount <= 0) {
    return state.syncing ? "…" : "";
  }

  return state.pendingCount > 99 ? "99+" : String(state.pendingCount);
}

export function getSyncBadgeTitle(state: SyncBadgeState): string {
  if (state.pendingCount <= 0 && !state.syncing) {
    return IDLE_TITLE;
  }

  const parts = ["TrailHost:"];
  if (state.syncing) {
    parts.push("同步中");
  }
  if (state.pendingCount > 0) {
    parts.push(`${state.pendingCount} 条待同步`);
  }
  return parts.join(" ");
}

export function getSyncBadgeColor(state: SyncBadgeState): string {
  if (state.syncing) return SYNCING_COLOR;
  return PENDING_COLOR;
}

export function isBadgeVisible(state: SyncBadgeState): boolean {
  return state.pendingCount > 0 || state.syncing;
}

