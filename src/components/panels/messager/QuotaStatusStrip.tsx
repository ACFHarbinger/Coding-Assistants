import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "../../../lib/tauri";
import type { ProviderQuota } from "../hub/types";
import { ProviderHealthDot } from "../harness/ProviderHealthChip";
import { useProviderHealth } from "../harness/useProviderHealth";
import { useOrchestrationPolicy } from "../harness/useOrchestrationPolicy";

/**
 * Compact provider-quota read-out for the agents/status area (Messager
 * sidebar). Mirrors DeepSeek + OpenCode Go usage from `hub_get_provider_quotas`.
 *
 * Polling is governed by `quota_auto_refresh_enabled` / `quota_auto_refresh_interval_secs`:
 * - Disabled by default: exactly one fetch on mount, no setInterval.
 * - Enabled: polls on the configured interval.
 * - Manual refresh button and metered-probes toggle are surfaced directly in the header.
 */
const MIRRORED_AGENT_IDS = ["deepseek", "opencode"] as const;

export function QuotaStatusStrip() {
  const [quotas, setQuotas] = useState<ProviderQuota[]>([]);
  const [refreshing, setRefreshing] = useState(false);
  const { healthMap } = useProviderHealth(30_000);
  const { policy, setAllowMeteredProbes } = useOrchestrationPolicy();
  const refreshInFlight = useRef(false);

  const refresh = useCallback(async () => {
    if (refreshInFlight.current) return;
    refreshInFlight.current = true;
    setRefreshing(true);
    try {
      // Fetch only the providers rendered here. The aggregate command also
      // launches the unrelated Codex/Gemini adapters and can exceed a poll
      // interval when several CLIs are unavailable.
      const results = await Promise.allSettled(
        MIRRORED_AGENT_IDS.map((agentId) =>
          invoke<ProviderQuota>("hub_refresh_provider_quota", { agentId })
        )
      );
      const next = results.flatMap((result) =>
        result.status === "fulfilled" ? [result.value] : []
      );
      setQuotas(next);
    } finally {
      refreshInFlight.current = false;
      setRefreshing(false);
    }
  }, []);

  // Exactly one fetch on mount
  useEffect(() => {
    void refresh();
  }, [refresh]);

  // Timed refresh if and only if quota_auto_refresh_enabled is true
  useEffect(() => {
    if (!policy.quota_auto_refresh_enabled) {
      return; // Disabled by default: exactly one fetch on mount, never call setInterval.
    }
    const cadenceMs = Math.max(30, policy.quota_auto_refresh_interval_secs) * 1000;
    const interval = window.setInterval(() => {
      void refresh();
    }, cadenceMs);
    return () => {
      window.clearInterval(interval);
    };
  }, [policy.quota_auto_refresh_enabled, policy.quota_auto_refresh_interval_secs, refresh]);

  const toggleMetered = async () => {
    const next = !policy.allow_metered_quota_probes;
    await setAllowMeteredProbes(next);
    void refresh();
  };

  if (quotas.length === 0) return null;

  const summary = (quota: ProviderQuota): string => {
    if (quota.status === "ok" && quota.balance) return quota.balance;
    if (quota.status === "ok" && quota.windows.length > 0) {
      const used = quota.windows.map((w) => w.used_percent).sort((a, b) => b - a)[0];
      return `${used}% used`;
    }
    return "unavailable";
  };

  return (
    <div
      style={{
        display: "grid",
        gap: "0.35rem",
        paddingTop: "0.75rem",
        borderTop: "1px solid var(--border-color)",
      }}
    >
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          paddingLeft: "0.5rem",
          paddingRight: "0.2rem",
        }}
      >
        <div
          style={{
            fontSize: "0.72rem",
            fontWeight: 700,
            color: "var(--text-muted)",
            letterSpacing: "0.05em",
            textTransform: "uppercase",
          }}
        >
          Provider usage
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: "0.35rem" }}>
          <button
            type="button"
            onClick={() => void toggleMetered()}
            title={`Allow metered usage probes (currently ${policy.allow_metered_quota_probes ? "ON" : "OFF"}). Checking Antigravity, OpenCode, or Muse costs tokens.`}
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: "0.25rem",
              background: policy.allow_metered_quota_probes ? "rgba(56, 189, 248, 0.15)" : "rgba(255,255,255,0.05)",
              border: `1px solid ${policy.allow_metered_quota_probes ? "rgba(56, 189, 248, 0.4)" : "var(--border-color)"}`,
              borderRadius: "12px",
              padding: "0.15rem 0.45rem",
              fontSize: "0.68rem",
              color: policy.allow_metered_quota_probes ? "#38bdf8" : "var(--text-muted)",
              cursor: "pointer",
            }}
          >
            <span
              style={{
                width: 6,
                height: 6,
                borderRadius: "50%",
                background: policy.allow_metered_quota_probes ? "#38bdf8" : "#64748b",
              }}
            />
            Metered: {policy.allow_metered_quota_probes ? "ON" : "OFF"}
          </button>
          <button
            type="button"
            onClick={() => void refresh()}
            disabled={refreshing}
            title="Refresh provider usage quotas"
            style={{
              background: "transparent",
              border: "none",
              color: "var(--text-muted)",
              cursor: refreshing ? "wait" : "pointer",
              fontSize: "0.85rem",
              padding: "0.1rem 0.25rem",
              lineHeight: 1,
            }}
          >
            ↻
          </button>
        </div>
      </div>

      {quotas.map((quota) => {
        const hasBalanceInfo = Boolean(quota.status === "ok" && quota.balance_info);
        const info = quota.balance_info;
        const paid = info?.paid ?? info?.topped_up ?? null;
        const gift = info?.gift ?? info?.granted ?? null;
        const total = info?.total ?? 0;
        const hasBreakdown = paid !== null && gift !== null && total > 0;
        const paidPercent = hasBreakdown ? Math.max(0, Math.min(100, (paid / total) * 100)) : 100;
        const giftPercent = hasBreakdown ? Math.max(0, Math.min(100, (gift / total) * 100)) : 0;

        return (
          <div
            key={quota.agent_id}
            title={quota.detail || quota.harness_title}
            style={{
              display: "grid",
              gap: "0.35rem",
              padding: "0.45rem 0.65rem",
              borderRadius: "8px",
              background: "rgba(0,0,0,0.25)",
              fontSize: "0.78rem",
              color: "var(--text-muted)",
              border: "1px solid rgba(255,255,255,0.05)",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
              <span
                style={{
                  width: "8px",
                  height: "8px",
                  borderRadius: "50%",
                  flexShrink: 0,
                  background: quota.status === "ok" ? "#22c55e" : "#64748b",
                }}
              />
              <span style={{ fontWeight: 600, color: "var(--text-main)" }}>
                {quota.harness_title || quota.agent_id}
              </span>
              <span
                style={{
                  marginLeft: "auto",
                  display: "inline-flex",
                  alignItems: "center",
                  gap: "0.45rem",
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  whiteSpace: "nowrap",
                }}
              >
                <ProviderHealthDot
                  health={healthMap[quota.agent_id]}
                  titlePrefix={`${quota.harness_title || quota.agent_id} CLI`}
                />
                {hasBalanceInfo ? (
                  <strong style={{ color: "#22c55e", fontSize: "0.82rem" }}>
                    ${info!.total.toFixed(2)} {info!.currency}
                  </strong>
                ) : (
                  <span>{summary(quota)}</span>
                )}
              </span>
            </div>

            {hasBreakdown && (
              <div style={{ display: "grid", gap: "0.2rem", paddingTop: "0.15rem" }}>
                <div
                  style={{
                    display: "flex",
                    height: "5px",
                    borderRadius: "3px",
                    overflow: "hidden",
                    background: "#334155",
                  }}
                  title={`Paid: $${paid.toFixed(2)} (${paidPercent.toFixed(1)}%) | Gift: $${gift.toFixed(2)} (${giftPercent.toFixed(1)}%)`}
                >
                  <div
                    style={{
                      width: `${paidPercent}%`,
                      background: "#38bdf8",
                      transition: "width 0.3s ease",
                    }}
                  />
                  <div
                    style={{
                      width: `${giftPercent}%`,
                      background: "#34d399",
                      transition: "width 0.3s ease",
                    }}
                  />
                </div>
                <div
                  style={{
                    display: "flex",
                    justifyContent: "space-between",
                    fontSize: "0.7rem",
                    color: "var(--text-muted)",
                    padding: "0 0.1rem",
                  }}
                >
                  <span style={{ display: "inline-flex", alignItems: "center", gap: "0.25rem" }}>
                    <i style={{ display: "inline-block", width: 6, height: 6, borderRadius: "50%", background: "#38bdf8" }} />
                    Paid ${paid.toFixed(2)}
                  </span>
                  <span style={{ display: "inline-flex", alignItems: "center", gap: "0.25rem" }}>
                    <i style={{ display: "inline-block", width: 6, height: 6, borderRadius: "50%", background: "#34d399" }} />
                    Gift ${gift.toFixed(2)}
                  </span>
                </div>
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
