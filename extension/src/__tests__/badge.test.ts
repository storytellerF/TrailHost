import { describe, it, expect } from "vitest";
import {
  getSyncBadgeColor,
  getSyncBadgeText,
  getSyncBadgeTitle,
  isBadgeVisible,
} from "../background/badge";

describe("sync badge helpers", () => {
  it("hides the badge when idle", () => {
    expect(isBadgeVisible({ pendingCount: 0, syncing: false })).toBe(false);
    expect(getSyncBadgeText({ pendingCount: 0, syncing: false })).toBe("");
    expect(getSyncBadgeTitle({ pendingCount: 0, syncing: false })).toBe("TrailHost");
  });

  it("shows pending count", () => {
    expect(isBadgeVisible({ pendingCount: 3, syncing: false })).toBe(true);
    expect(getSyncBadgeText({ pendingCount: 3, syncing: false })).toBe("3");
    expect(getSyncBadgeColor({ pendingCount: 3, syncing: false })).toBe("#d97706");
    expect(getSyncBadgeTitle({ pendingCount: 3, syncing: false })).toBe(
      "TrailHost: 3 条待同步"
    );
  });

  it("shows syncing state alongside the count", () => {
    expect(isBadgeVisible({ pendingCount: 3, syncing: true })).toBe(true);
    expect(getSyncBadgeText({ pendingCount: 3, syncing: true })).toBe("3");
    expect(getSyncBadgeColor({ pendingCount: 3, syncing: true })).toBe("#2563eb");
    expect(getSyncBadgeTitle({ pendingCount: 3, syncing: true })).toBe(
      "TrailHost: 同步中 3 条待同步"
    );
  });

  it("shows an activity mark when syncing without pending items", () => {
    expect(isBadgeVisible({ pendingCount: 0, syncing: true })).toBe(true);
    expect(getSyncBadgeText({ pendingCount: 0, syncing: true })).toBe("…");
    expect(getSyncBadgeTitle({ pendingCount: 0, syncing: true })).toBe("TrailHost: 同步中");
  });

  it("caps large counts", () => {
    expect(getSyncBadgeText({ pendingCount: 120, syncing: false })).toBe("99+");
  });
});
