export type QuotaStatus = "ok" | "unavailable";

export interface Account {
  plan: string | null;
  tier: string | null;
}

export interface Bucket {
  id: string;
  label: string;
  window: string | null;
  description: string | null;
  remainingPercent: number | null;
  resetAt: string | null;
  resetAtMs: number | null;
}

export interface QuotaGroup {
  id: string;
  name: string;
  description: string | null;
  buckets: Bucket[];
}

export interface ModelQuota {
  id: string;
  name: string;
  provider: string;
  poolId: string | null;
  poolName: string | null;
  limitId: string | null;
  limitLabel: string | null;
  shared: boolean;
  remainingPercent: number | null;
  resetAt: string | null;
  resetAtMs: number | null;
}

export interface QuotaProblem {
  code: string;
  title: string;
  detail: string;
}

export interface QuotaSnapshot {
  status: QuotaStatus;
  fetchedAt: string;
  fetchedAtMs: number;
  account: Account | null;
  groups: QuotaGroup[];
  models: ModelQuota[];
  source: string | null;
  error: QuotaProblem | null;
}

export interface Preferences {
  theme: "system" | "light" | "dark";
  alwaysOnTop: boolean;
  startWithWindows: boolean;
  expanded: boolean;
  locked?: boolean;
  width?: number;
  position: { x: number; y: number } | null;
}

export type PreferencesPatch = Partial<Pick<Preferences, "theme" | "alwaysOnTop" | "startWithWindows" | "expanded" | "locked" | "width">>;

const clockFormat = new Intl.DateTimeFormat(undefined, {
  hour: "2-digit",
  minute: "2-digit",
});

/** Formats a 0–100 remaining percentage. Never rounds up to a misleading 100%. */
export function formatPercent(value: number | null): string {
  if (value === null || !Number.isFinite(value)) return "—";
  const clamped = Math.min(100, Math.max(0, value));
  if (clamped === 0) return "0%";
  if (clamped < 1) return "<1%";
  if (clamped > 99 && clamped < 100) return ">99%";
  return `${Math.round(clamped)}%`;
}

/**
 * Compact countdown used under a limit bar. Returns null when no reset time was
 * reported by Antigravity.
 */
export function formatCountdown(resetAtMs: number | null, nowMs: number): string | null {
  if (resetAtMs === null) return null;
  const diff = resetAtMs - nowMs;
  if (diff <= 0) return "resetting";

  const minutes = Math.floor(diff / 60_000);
  if (diff < 60_000) return "<1m";
  if (minutes < 60) return `${minutes}m`;

  const hours = Math.floor(minutes / 60);
  if (hours < 24) {
    const rest = minutes % 60;
    return rest === 0 ? `${hours}h` : `${hours}h ${rest}m`;
  }

  const days = Math.floor(hours / 24);
  const rest = hours % 24;
  return rest === 0 ? `${days}d` : `${days}d ${rest}h`;
}

export function formatClock(ms: number): string {
  return clockFormat.format(ms);
}

const resetTimeFormatSameDay = new Intl.DateTimeFormat(undefined, {
  hour: "numeric",
  minute: "2-digit",
});

const resetTimeFormatOtherDay = new Intl.DateTimeFormat(undefined, {
  weekday: "short",
  hour: "numeric",
  minute: "2-digit",
});

/**
 * Formats a reset time timestamp for display.
 * Displays time (e.g. 2:34 AM) if resetting today, or weekday and time (e.g. Sat, 2:34 AM).
 */
export function formatResetTime(resetAtMs: number | null, nowMs: number = Date.now()): string | null {
  if (resetAtMs === null || !Number.isFinite(resetAtMs)) return null;
  const resetDate = new Date(resetAtMs);
  const nowDate = new Date(nowMs);
  const isSameDay =
    resetDate.getDate() === nowDate.getDate() &&
    resetDate.getMonth() === nowDate.getMonth() &&
    resetDate.getFullYear() === nowDate.getFullYear();

  return isSameDay
    ? resetTimeFormatSameDay.format(resetDate)
    : resetTimeFormatOtherDay.format(resetDate);
}

export function windowLabel(window: string | null): string {
  if (!window) return "Limit";
  const normalized = window.trim().toLowerCase();
  if (normalized === "weekly") return "Weekly";
  if (normalized === "5h") return "5 hours";
  if (normalized === "daily") return "Daily";
  return window;
}

export function shortGroupName(name: string): string {
  return name
    .replace(/\s+models?$/i, "")
    .replace(/\band\b/gi, "&")
    .replace(/\s+/g, " ")
    .trim();
}

export interface PrimaryLimit {
  percent: number;
  groupName: string;
  window: string | null;
}

/**
 * The tightest reported limit across all pools. Used by the collapsed widget.
 * Ties prefer the weekly window because it is the more meaningful budget.
 */
export function pickPrimaryLimit(snapshot: QuotaSnapshot | null): PrimaryLimit | null {
  if (!snapshot || snapshot.status !== "ok") return null;

  let best: PrimaryLimit | null = null;
  for (const group of snapshot.groups) {
    for (const bucket of group.buckets) {
      if (bucket.remainingPercent === null) continue;
      const candidate: PrimaryLimit = {
        percent: bucket.remainingPercent,
        groupName: shortGroupName(group.name),
        window: bucket.window,
      };
      if (best === null) {
        best = candidate;
        continue;
      }
      const lower = candidate.percent < best.percent;
      const tieWeekly =
        candidate.percent === best.percent &&
        candidate.window === "weekly" &&
        best.window !== "weekly";
      if (lower || tieWeekly) best = candidate;
    }
  }
  return best;
}

export interface ProviderGroup {
  provider: string;
  models: ModelQuota[];
}

export function groupByProvider(models: ModelQuota[]): ProviderGroup[] {
  const groups: ProviderGroup[] = [];
  for (const model of models) {
    const last = groups[groups.length - 1];
    if (last && last.provider === model.provider) {
      last.models.push(model);
    } else {
      groups.push({ provider: model.provider, models: [model] });
    }
  }
  return groups;
}

/** Models whose reported limit is the model's own, not a shared pool. */
export function isModelSpecific(model: ModelQuota): boolean {
  return !model.shared && model.limitId === "model";
}

export interface DualBucketLimit {
  percent: number;
  groupName: string;
  window: string | null;
  resetAtMs: number | null;
}

export interface DualLimits {
  weekly: DualBucketLimit | null;
  hourly: DualBucketLimit | null;
}

/**
 * Extracts both the most constrained Weekly limit and Hourly (5-Hour) limit
 * across all reported Antigravity pools.
 */
export function pickDualLimits(snapshot: QuotaSnapshot | null): DualLimits {
  if (!snapshot || snapshot.status !== "ok") {
    return { weekly: null, hourly: null };
  }

  let bestWeekly: DualBucketLimit | null = null;
  let bestHourly: DualBucketLimit | null = null;

  for (const group of snapshot.groups) {
    for (const bucket of group.buckets) {
      if (bucket.remainingPercent === null) continue;
      const windowStr = (bucket.window || "").trim().toLowerCase();
      const idStr = (bucket.id || "").trim().toLowerCase();
      const labelStr = (bucket.label || "").trim().toLowerCase();

      const isWeekly =
        windowStr === "weekly" ||
        idStr.includes("weekly") ||
        labelStr.includes("weekly");

      const isHourly =
        windowStr === "5h" ||
        windowStr === "hourly" ||
        windowStr === "daily" ||
        idStr.includes("5h") ||
        idStr.includes("hour") ||
        labelStr.includes("hour") ||
        labelStr.includes("five hour");

      const candidate: DualBucketLimit = {
        percent: bucket.remainingPercent,
        groupName: shortGroupName(group.name),
        window: bucket.window,
        resetAtMs: bucket.resetAtMs,
      };

      if (isWeekly) {
        if (!bestWeekly || candidate.percent < bestWeekly.percent) {
          bestWeekly = candidate;
        }
      } else if (isHourly) {
        if (!bestHourly || candidate.percent < bestHourly.percent) {
          bestHourly = candidate;
        }
      } else {
        // Fallback: if not categorized as weekly, treat as short-term/hourly budget
        if (!bestHourly || candidate.percent < bestHourly.percent) {
          bestHourly = candidate;
        }
      }
    }
  }

  return { weekly: bestWeekly, hourly: bestHourly };
}

/**
 * Filters the model catalog to only include genuine models reported
 * inside Antigravity with active quota pool bindings or real limits.
 */
export function filterAntigravityModels(models: ModelQuota[]): ModelQuota[] {
  return models.filter((model) => {
    // Must have a valid display name
    if (!model.name || model.name.trim().length === 0) return false;
    const lower = model.name.toLowerCase();
    // Exclude legacy Gemini versions (1.0, 1.5 legacy)
    if (lower.includes("1.0") || lower.includes("1.5")) return false;

    // Must belong to an active Antigravity pool or have explicit remaining percentage
    const hasPool = Boolean(model.poolId || model.poolName);
    const hasQuota = model.remainingPercent !== null && Number.isFinite(model.remainingPercent);
    return hasPool || hasQuota;
  });
}

