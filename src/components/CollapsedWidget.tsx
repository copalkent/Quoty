import React from "react";
import type { QuotaProblem, QuotaSnapshot } from "../lib/quota";
import { formatPercent, pickDualLimits, pickPrimaryLimit } from "../lib/quota";
import { useWindowDrag } from "../lib/useWindowDrag";
import { ChevronIcon, Mark } from "./Icons";

interface Props {
  snapshot: QuotaSnapshot | null;
  problem: QuotaProblem | null;
  refreshing: boolean;
  onExpand: () => void;
}

export const CollapsedWidget: React.FC<Props> = ({
  snapshot,
  problem,
  refreshing,
  onExpand,
}) => {
  const drag = useWindowDrag(onExpand);
  const dual = pickDualLimits(snapshot);
  const primary = pickPrimaryLimit(snapshot);
  const connected = !problem && snapshot?.status === "ok";

  const hasDual = connected && (dual.weekly !== null || dual.hourly !== null);
  const weeklyPct = dual.weekly ? formatPercent(dual.weekly.percent) : "—";
  const hourlyPct = dual.hourly ? formatPercent(dual.hourly.percent) : "—";

  const weeklyVal = dual.weekly?.percent ?? 0;
  const hourlyVal = dual.hourly?.percent ?? 0;

  const caption = connected && primary
    ? `${primary.groupName} ${primary.window || "limit"}`
    : problem
      ? "Not connected"
      : "Reading quota…";

  const ariaLabel = hasDual
    ? `Antigravity quota: Weekly ${weeklyPct}, 5-Hour ${hourlyPct}. Open widget.`
    : `Antigravity quota unavailable: ${caption}. Open widget.`;

  return (
    <div
      className="pill"
      role="button"
      tabIndex={0}
      aria-label={ariaLabel}
      title="Click to open big widget · Drag to move"
      onPointerDown={drag.onPointerDown}
      onKeyDown={(event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          onExpand();
        }
      }}
    >
      <span className={`pill-mark ${refreshing ? "is-live" : ""}`} title="Google Antigravity">
        <Mark size={18} />
      </span>

      {hasDual ? (
        <div className="pill-dual-container">
          {/* Weekly percentage & bar */}
          <div className="pill-quota-col" title={`Weekly Limit: ${weeklyPct} remaining`}>
            <div className="pill-quota-top">
              <span className="pill-quota-tag">WEEKLY</span>
              <span className="pill-quota-val">{weeklyPct}</span>
            </div>
            <div className="pill-mini-track">
              <div
                className="pill-mini-fill"
                style={{ width: `${Math.min(100, Math.max(0, weeklyVal))}%` }}
              />
            </div>
          </div>

          <div className="pill-dual-divider" aria-hidden />

          {/* 5-Hour / Hourly percentage & bar */}
          <div className="pill-quota-col" title={`5-Hour Limit: ${hourlyPct} remaining`}>
            <div className="pill-quota-top">
              <span className="pill-quota-tag">5-HOUR</span>
              <span className="pill-quota-val">{hourlyPct}</span>
            </div>
            <div className="pill-mini-track">
              <div
                className="pill-mini-fill"
                style={{ width: `${Math.min(100, Math.max(0, hourlyVal))}%` }}
              />
            </div>
          </div>
        </div>
      ) : (
        <span className="pill-copy">
          <span className="pill-value">{connected && primary ? formatPercent(primary.percent) : "—"}</span>
          <span className="pill-caption">
            <span
              className={`status-dot ${connected ? "is-on" : "is-off"}`}
              aria-hidden
            />
            <span className="pill-caption-text">{caption}</span>
          </span>
        </span>
      )}

      <span className="pill-chevron" aria-hidden title="Expand into big widget">
        <ChevronIcon size={14} />
      </span>
    </div>
  );
};
