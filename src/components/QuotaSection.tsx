import React, { useEffect, useState } from "react";
import type {
  Bucket,
  ModelQuota,
  QuotaGroup,
  QuotaProblem,
  QuotaSnapshot,
} from "../lib/quota";
import {
  formatClock,
  formatCountdown,
  formatPercent,
  formatResetTime,
  groupByProvider,
  isModelSpecific,
  shortGroupName,
  windowLabel,
} from "../lib/quota";
import { RefreshIcon } from "./Icons";

interface QuotaSectionProps {
  snapshot: QuotaSnapshot | null;
  problem: QuotaProblem | null;
  refreshing: boolean;
  onRefresh: () => void;
}

function useCountdownTick() {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    // Minute or 15s interval is sufficient for countdowns
    const id = window.setInterval(() => setNow(Date.now()), 15000);
    return () => window.clearInterval(id);
  }, []);
  return now;
}

function BucketRow({ bucket, now }: { bucket: Bucket; now: number }) {
  const countdown = formatCountdown(bucket.resetAtMs, now);
  const resetClock = formatResetTime(bucket.resetAtMs, now);
  const label = windowLabel(bucket.window);
  const value = bucket.remainingPercent;

  let resetLabel = "No reset time reported";
  if (countdown !== null) {
    if (countdown === "resetting") {
      resetLabel = "Resetting now";
    } else if (resetClock !== null) {
      resetLabel = `Resets in ${countdown} (${resetClock})`;
    } else {
      resetLabel = `Resets in ${countdown}`;
    }
  }

  return (
    <div className="bucket-item">
      <div className="bucket-info">
        <span className="bucket-label">{label}</span>
        <span className="bucket-percentage">{formatPercent(value)}</span>
      </div>
      <div
        className={`compact-progress${value === null ? " progress--unknown" : ""}`}
        role="progressbar"
        aria-label={`${label} quota remaining`}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={value === null ? undefined : Math.round(value)}
        aria-valuetext={value === null ? "Not reported" : formatPercent(value)}
      >
        <span
          className="progress-fill"
          style={{ width: `${value ?? 0}%` }}
        />
      </div>
      <div className="bucket-reset-meta" title={bucket.description ?? undefined}>
        {resetLabel}
      </div>
    </div>
  );
}

function PoolGroup({ group, now }: { group: QuotaGroup; now: number }) {
  const name = shortGroupName(group.name) || group.name;

  return (
    <div className="quota-pool" aria-label={`${name} quota pool`}>
      <div className="quota-pool-header">
        <span className="quota-pool-title">{name}</span>
        <span
          className="quota-badge quota-badge--shared"
          title={group.description ?? "These models share this quota pool."}
        >
          Shared Pool
        </span>
      </div>
      <div className="quota-pool-buckets">
        {group.buckets.map((bucket) => (
          <BucketRow key={bucket.id} bucket={bucket} now={now} />
        ))}
      </div>
    </div>
  );
}

function ModelItem({ model, now }: { model: ModelQuota; now: number }) {
  const isOwn = isModelSpecific(model);
  const countdown = formatCountdown(model.resetAtMs, now);
  const resetClock = formatResetTime(model.resetAtMs, now);

  let resetInfo: string | null = null;
  if (countdown !== null) {
    resetInfo = resetClock ? `${countdown} (${resetClock})` : countdown;
  }

  return (
    <div className="model-quota-item">
      <div className="model-quota-main">
        <span className="model-name-text" title={model.name}>
          {model.name}
        </span>
        {isOwn ? (
          <span
            className="quota-badge quota-badge--own"
            title="This model has an independent quota distinct from shared pools."
          >
            Independent
          </span>
        ) : (
          <span
            className="quota-badge quota-badge--pool"
            title={model.poolName ? `Shares ${model.poolName}` : "Shared pool"}
          >
            {model.poolName ? shortGroupName(model.poolName) : "Shared"}
          </span>
        )}
      </div>
      <div className="model-quota-aside">
        {resetInfo ? (
          <span className="model-reset-text" title="Reset countdown and time">
            {resetInfo}
          </span>
        ) : model.limitLabel ? (
          <span className="model-window-tag">{windowLabel(model.limitLabel)}</span>
        ) : null}
        <span className="model-quota-value">
          {formatPercent(model.remainingPercent)}
        </span>
      </div>
    </div>
  );
}

export const QuotaSection = React.memo(function QuotaSection({
  snapshot,
  problem,
  refreshing,
  onRefresh,
}: QuotaSectionProps) {
  const now = useCountdownTick();

  return (
    <section className="quota-section" aria-label="Antigravity model quotas">
      <div className="quota-section-header">
        <div className="quota-header-left">
          <span className="quota-section-title">Model Quotas</span>
          {snapshot?.groups && snapshot.groups.length > 0 ? (
            <span className="quota-count-tag">
              {snapshot.groups.length} {snapshot.groups.length === 1 ? "pool" : "pools"}
            </span>
          ) : null}
        </div>
        <div className="quota-header-right">
          <span className="quota-updated-text">
            {snapshot ? `Updated ${formatClock(snapshot.fetchedAtMs)}` : "Checking…"}
          </span>
          <button
            type="button"
            className="quota-refresh-btn"
            data-interactive
            title="Refresh quotas"
            aria-label="Refresh quotas"
            aria-busy={refreshing}
            onClick={onRefresh}
          >
            <RefreshIcon size={12} className={refreshing ? "is-spinning" : undefined} />
          </button>
        </div>
      </div>

      {problem ? (
        <div className="quota-state-card quota-state--problem">
          <span className="state-badge">Quota Unavailable</span>
          <p className="state-message-title">{problem.title}</p>
          <p className="state-message-detail">{problem.detail}</p>
          <button
            type="button"
            className="state-retry-btn"
            data-interactive
            onClick={onRefresh}
            disabled={refreshing}
          >
            {refreshing ? "Reconnecting…" : "Retry Connection"}
          </button>
        </div>
      ) : snapshot ? (
        <div className="quota-content">
          {snapshot.groups.length > 0 ? (
            <div className="quota-pools-list">
              {snapshot.groups.map((group) => (
                <PoolGroup key={group.id} group={group} now={now} />
              ))}
            </div>
          ) : null}

          {snapshot.models && snapshot.models.length > 0 ? (
            <div className="quota-models-wrapper">
              <div className="quota-models-header">
                <span className="models-heading">Available Models</span>
                <span className="models-count">{snapshot.models.length}</span>
              </div>
              <div className="quota-models-scroll">
                {groupByProvider(snapshot.models).map((group) => (
                  <div className="provider-subgroup" key={group.provider}>
                    <div className="provider-subgroup-title">{group.provider}</div>
                    {group.models.map((model) => (
                      <ModelItem key={model.id} model={model} now={now} />
                    ))}
                  </div>
                ))}
              </div>
            </div>
          ) : (
            <div className="quota-models-empty">No detected models reported.</div>
          )}
        </div>
      ) : (
        <div className="quota-state-card quota-state--connecting">
          <span className="state-badge">Connecting</span>
          <p className="state-message-detail">
            Connecting to Antigravity's local language server…
          </p>
        </div>
      )}
    </section>
  );
});
