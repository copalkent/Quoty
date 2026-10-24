import React, { useEffect, useState } from "react";
import type {
  QuotaProblem,
  QuotaSnapshot,
} from "../lib/quota";
import {
  formatClock,
  formatCountdown,
  formatPercent,
  pickDualLimits,
} from "../lib/quota";
import { RefreshIcon } from "./Icons";

interface Props {
  snapshot: QuotaSnapshot | null;
  problem: QuotaProblem | null;
  refreshing: boolean;
  onRefresh: () => void;
}

function useCountdownTick() {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = window.setInterval(() => setNow(Date.now()), 15000);
    return () => window.clearInterval(id);
  }, []);
  return now;
}

function QuotaLimitRow({
  label,
  percent,
  resetCountdown,
}: {
  label: string;
  percent: number | null;
  resetCountdown: string | null;
}) {
  const val = percent ?? 0;
  const clamped = Math.min(100, Math.max(0, val));

  return (
    <div className="hud-limit-row">
      <div className="hud-limit-row-top">
        <span className="hud-limit-name">{label}</span>
        <span className="hud-limit-percent">{formatPercent(percent)}</span>
      </div>

      <div
        className="hud-limit-track"
        role="progressbar"
        aria-label={label}
        aria-valuenow={percent ?? undefined}
      >
        <div className="hud-limit-fill" style={{ width: `${clamped}%` }} />
      </div>

      <div className="hud-limit-footer">
        {resetCountdown ? (
          <span className="hud-limit-reset">
            Fully refreshes in {resetCountdown}
          </span>
        ) : (
          <span className="hud-limit-reset hud-limit-reset--idle">100% available</span>
        )}
      </div>
    </div>
  );
}

export const RainmeterQuota: React.FC<Props> = ({
  snapshot,
  problem,
  refreshing,
  onRefresh,
}) => {
  const now = useCountdownTick();

  // Extract Antigravity groups: Gemini Models and Claude/GPT models
  const groups = snapshot?.groups ?? [];

  // Fallback dual limits if groups aren't individually split
  const dual = pickDualLimits(snapshot);

  const weeklyCountdown = dual.weekly
    ? formatCountdown(dual.weekly.resetAtMs, now)
    : null;
  const hourlyCountdown = dual.hourly
    ? formatCountdown(dual.hourly.resetAtMs, now)
    : null;

  return (
    <div className="rainmeter-quota-block" aria-label="Antigravity Quota">
      {/* Top HUD Bar */}
      <div className="hud-quota-header">
        <div className="hud-quota-title-row">
          <span className="hud-quota-tag">ANTIGRAVITY QUOTA</span>
          {snapshot?.account?.plan ? (
            <span className="hud-plan-tag">{snapshot.account.plan}</span>
          ) : null}
        </div>

        <div className="hud-quota-actions">
          <span className="hud-quota-time">
            {snapshot ? formatClock(snapshot.fetchedAtMs) : "Checking…"}
          </span>
          <button
            type="button"
            className="hud-refresh-btn"
            onClick={onRefresh}
            title="Refresh quotas"
            aria-label="Refresh quotas"
            aria-busy={refreshing}
            data-interactive
          >
            <RefreshIcon size={12} className={refreshing ? "is-spinning" : undefined} />
          </button>
        </div>
      </div>

      {problem ? (
        <div className="hud-quota-state hud-quota-state--error">
          <span className="hud-state-kicker">Antigravity Offline</span>
          <p className="hud-state-msg">{problem.title}</p>
          <button
            type="button"
            className="hud-retry-btn"
            onClick={onRefresh}
            disabled={refreshing}
            data-interactive
          >
            {refreshing ? "Reconnecting…" : "Retry"}
          </button>
        </div>
      ) : groups.length > 0 ? (
        <div className="hud-groups-2col">
          {groups.map((group) => {
            const weekly = group.buckets.find(
              (b) =>
                (b.window || "").toLowerCase() === "weekly" ||
                (b.id || "").toLowerCase().includes("weekly")
            );
            const hourly = group.buckets.find(
              (b) =>
                (b.window || "").toLowerCase() === "5h" ||
                (b.id || "").toLowerCase().includes("5h") ||
                (b.id || "").toLowerCase().includes("hourly")
            );

            return (
              <div className="hud-group-block" key={group.id}>
                <div className="hud-group-header">
                  <span className="hud-group-title">{group.name}</span>
                </div>

                <div className="hud-group-limits">
                  {weekly ? (
                    <QuotaLimitRow
                      label="Weekly Limit Remaining"
                      percent={weekly.remainingPercent}
                      resetCountdown={formatCountdown(weekly.resetAtMs, now)}
                    />
                  ) : null}

                  {hourly ? (
                    <QuotaLimitRow
                      label="Five Hour Limit Remaining"
                      percent={hourly.remainingPercent}
                      resetCountdown={formatCountdown(hourly.resetAtMs, now)}
                    />
                  ) : null}
                </div>
              </div>
            );
          })}
        </div>
      ) : dual.weekly || dual.hourly ? (
        <div className="hud-groups-2col">
          {/* Gemini Models Column */}
          <div className="hud-group-block">
            <div className="hud-group-header">
              <span className="hud-group-title">Gemini Models</span>
            </div>
            <div className="hud-group-limits">
              <QuotaLimitRow
                label="Weekly Limit Remaining"
                percent={dual.weekly?.percent ?? null}
                resetCountdown={weeklyCountdown}
              />
              <QuotaLimitRow
                label="Five Hour Limit Remaining"
                percent={dual.hourly?.percent ?? null}
                resetCountdown={hourlyCountdown}
              />
            </div>
          </div>

          {/* Claude & GPT Models Column */}
          <div className="hud-group-block">
            <div className="hud-group-header">
              <span className="hud-group-title">Claude and GPT models</span>
            </div>
            <div className="hud-group-limits">
              <QuotaLimitRow
                label="Weekly Limit Remaining"
                percent={100}
                resetCountdown={null}
              />
              <QuotaLimitRow
                label="Five Hour Limit Remaining"
                percent={100}
                resetCountdown={null}
              />
            </div>
          </div>
        </div>
      ) : (
        <div className="hud-quota-state">
          <span className="hud-state-kicker">Connecting to Antigravity</span>
          <p className="hud-state-msg">Reading local language server…</p>
        </div>
      )}
    </div>
  );
};
