import { describe, expect, it } from "vitest";
import {
  formatCountdown,
  formatPercent,
  formatResetTime,
  groupByProvider,
  isModelSpecific,
  pickDualLimits,
  filterAntigravityModels,
  pickPrimaryLimit,
  shortGroupName,
  windowLabel,
  type ModelQuota,
  type QuotaSnapshot,
} from "./quota";

function snapshot(overrides: Partial<QuotaSnapshot> = {}): QuotaSnapshot {
  return {
    status: "ok",
    fetchedAt: "2026-10-23T19:18:00Z",
    fetchedAtMs: 1_792_792_680_000,
    account: { plan: "Pro", tier: "Google AI Pro" },
    groups: [
      {
        id: "gemini",
        name: "Gemini Models",
        description: null,
        buckets: [
          {
            id: "gemini-weekly",
            label: "Weekly Limit Remaining",
            window: "weekly",
            description: null,
            remainingPercent: 32.8,
            resetAt: "2026-10-24T02:34:50Z",
            resetAtMs: 1_792_898_090_000,
          },
          {
            id: "gemini-5h",
            label: "Five Hour Limit Remaining",
            window: "5h",
            description: null,
            remainingPercent: 100,
            resetAt: "2026-10-24T00:16:52Z",
            resetAtMs: 1_792_887_412_000,
          },
        ],
      },
      {
        id: "third-party",
        name: "Claude and GPT models",
        description: null,
        buckets: [
          {
            id: "3p-weekly",
            label: "Weekly Limit Remaining",
            window: "weekly",
            description: null,
            remainingPercent: 49.3,
            resetAt: null,
            resetAtMs: null,
          },
        ],
      },
    ],
    models: [],
    source: "Antigravity IDE · local language server",
    error: null,
    ...overrides,
  };
}

describe("formatPercent", () => {
  it("rounds to whole percentages", () => {
    expect(formatPercent(32.81504)).toBe("33%");
    expect(formatPercent(100)).toBe("100%");
    expect(formatPercent(0)).toBe("0%");
  });

  it("does not fake values", () => {
    expect(formatPercent(null)).toBe("—");
    expect(formatPercent(Number.NaN)).toBe("—");
    expect(formatPercent(0.4)).toBe("<1%");
    expect(formatPercent(99.6)).toBe(">99%");
  });

  it("clamps out-of-range values", () => {
    expect(formatPercent(140)).toBe("100%");
    expect(formatPercent(-10)).toBe("0%");
  });
});

describe("formatCountdown", () => {
  const now = 1_000_000_000_000;

  it("handles missing reset times", () => {
    expect(formatCountdown(null, now)).toBeNull();
  });

  it("formats hours, days and minutes", () => {
    expect(formatCountdown(now + 7 * 3600_000 + 16 * 60_000, now)).toBe("7h 16m");
    expect(formatCountdown(now + 5 * 86400_000 + 8 * 3600_000, now)).toBe("5d 8h");
    expect(formatCountdown(now + 42 * 60_000, now)).toBe("42m");
    expect(formatCountdown(now + 20_000, now)).toBe("<1m");
  });

  it("marks elapsed reset times", () => {
    expect(formatCountdown(now - 1, now)).toBe("resetting");
  });
});

describe("formatResetTime", () => {
  it("handles missing reset times", () => {
    expect(formatResetTime(null)).toBeNull();
    expect(formatResetTime(Number.NaN)).toBeNull();
  });

  it("formats valid reset times", () => {
    const fixedNow = new Date("2026-10-24T10:00:00Z").getTime();
    const sameDay = new Date("2026-10-24T14:30:00Z").getTime();
    const otherDay = new Date("2026-10-28T09:15:00Z").getTime();

    expect(formatResetTime(sameDay, fixedNow)).toBeTruthy();
    expect(formatResetTime(otherDay, fixedNow)).toBeTruthy();
  });
});

describe("pickPrimaryLimit", () => {
  it("picks the tightest reported limit", () => {
    expect(pickPrimaryLimit(snapshot())).toEqual({
      percent: 32.8,
      groupName: "Gemini",
      window: "weekly",
    });
  });

  it("prefers the weekly window on ties", () => {
    const data = snapshot();
    data.groups[0]!.buckets.push({
      id: "extra",
      label: "Extra",
      window: "5h",
      description: null,
      remainingPercent: 32.8,
      resetAt: null,
      resetAtMs: null,
    });
    expect(pickPrimaryLimit(data)?.window).toBe("weekly");
  });

  it("ignores unknown values and unavailable snapshots", () => {
    const data = snapshot();
    data.groups.forEach((group) =>
      group.buckets.forEach((bucket) => {
        bucket.remainingPercent = null;
      }),
    );
    expect(pickPrimaryLimit(data)).toBeNull();
    expect(pickPrimaryLimit(null)).toBeNull();
    expect(pickPrimaryLimit(snapshot({ status: "unavailable" }))).toBeNull();
  });
});

describe("labels", () => {
  it("shortens group names without changing meaning", () => {
    expect(shortGroupName("Gemini Models")).toBe("Gemini");
    expect(shortGroupName("Claude and GPT models")).toBe("Claude & GPT");
    expect(shortGroupName("Google")).toBe("Google");
  });

  it("humanizes windows", () => {
    expect(windowLabel("weekly")).toBe("Weekly");
    expect(windowLabel("5h")).toBe("5 hours");
    expect(windowLabel(null)).toBe("Limit");
  });
});

describe("model grouping", () => {
  const model = (overrides: Partial<ModelQuota>): ModelQuota => ({
    id: "m",
    name: "Model",
    provider: "Google",
    poolId: "gemini",
    poolName: "Gemini Models",
    limitId: "gemini-weekly",
    limitLabel: "weekly",
    shared: true,
    remainingPercent: 50,
    resetAt: null,
    resetAtMs: null,
    ...overrides,
  });

  it("keeps providers together", () => {
    const grouped = groupByProvider([
      model({ id: "a", provider: "Google" }),
      model({ id: "b", provider: "Google" }),
      model({ id: "c", provider: "Anthropic" }),
    ]);
    expect(grouped.map((group) => group.provider)).toEqual(["Google", "Anthropic"]);
    expect(grouped[0]!.models).toHaveLength(2);
  });

  it("distinguishes model-specific limits", () => {
    expect(isModelSpecific(model({ shared: false, limitId: "model" }))).toBe(true);
    expect(isModelSpecific(model({}))).toBe(false);
  });
});

describe("pickDualLimits", () => {
  it("extracts both weekly and 5-hour limits across pools", () => {
    const snap = snapshot();
    const dual = pickDualLimits(snap);

    expect(dual.weekly).not.toBeNull();
    expect(dual.weekly?.percent).toBe(32.8);
    expect(dual.weekly?.groupName).toBe("Gemini");

    expect(dual.hourly).not.toBeNull();
    expect(dual.hourly?.percent).toBe(100);
    expect(dual.hourly?.groupName).toBe("Gemini");
  });

  it("returns nulls for offline or unavailable snapshot", () => {
    expect(pickDualLimits(null)).toEqual({ weekly: null, hourly: null });
    expect(pickDualLimits(snapshot({ status: "unavailable" }))).toEqual({
      weekly: null,
      hourly: null,
    });
  });
});

describe("filterAntigravityModels", () => {
  it("filters out empty or unallocated models", () => {
    const models: ModelQuota[] = [
      {
        id: "valid-1",
        name: "Claude 3.5 Sonnet",
        provider: "Anthropic",
        poolId: "third-party",
        poolName: "Claude",
        limitId: "3p-weekly",
        limitLabel: "weekly",
        shared: true,
        remainingPercent: 50,
        resetAt: null,
        resetAtMs: null,
      },
      {
        id: "unbound",
        name: "Experimental Model",
        provider: "Google",
        poolId: null,
        poolName: null,
        limitId: null,
        limitLabel: null,
        shared: false,
        remainingPercent: null,
        resetAt: null,
        resetAtMs: null,
      },
    ];

    const filtered = filterAntigravityModels(models);
    expect(filtered).toHaveLength(1);
    expect(filtered[0]?.name).toBe("Claude 3.5 Sonnet");
  });
});

