import { useEffect, useState } from "react";
import { invoke } from "../../../lib/tauri";
import type { ProviderQuota } from "../hub/types";

/**
 * Compact provider-quota read-out for the agents/status area (Messager
 * sidebar). Mirrors DeepSeek + OpenCode Go usage from `hub_get_provider_quotas`
 * on a sane poll interval — not tight — so the slow `opencode run "/ogc-usage"`
 * backend call is not hammered. Failures degrade to a muted "unavailable"
 * dot rather than surfacing an error.
 */
const POLL_MS = 60_000;
const MIRRORED_AGENT_IDS = ["deepseek", "opencode"] as const;

export function QuotaStatusStrip() {
  const [quotas, setQuotas] = useState<ProviderQuota[]>([]);

  useEffect(() => {
    let disposed = false;
    let refreshInFlight = false;
    const refresh = async () => {
      if (refreshInFlight) return;
      refreshInFlight = true;
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
        if (!disposed) setQuotas(next);
      } finally {
        refreshInFlight = false;
      }
    };
    void refresh();
    const interval = window.setInterval(() => { void refresh(); }, POLL_MS);
    return () => {
      disposed = true;
      window.clearInterval(interval);
    };
  }, []);

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
        gap: "0.3rem",
        paddingTop: "0.75rem",
        borderTop: "1px solid var(--border-color)",
      }}
    >
      <div style={{ fontSize: "0.75rem", fontWeight: 700, color: "var(--text-muted)", letterSpacing: "0.05em", textTransform: "uppercase", paddingLeft: "0.5rem" }}>
        Provider usage
      </div>
      {quotas.map((quota) => (
        <div key={quota.agent_id} title={quota.detail || quota.harness_title} style={{ display: "flex", alignItems: "center", gap: "0.5rem", padding: "0.4rem 0.75rem", borderRadius: "8px", background: "rgba(0,0,0,0.2)", fontSize: "0.78rem", color: "var(--text-muted)" }}>
          <span
            style={{
              width: "8px",
              height: "8px",
              borderRadius: "50%",
              flexShrink: 0,
              background: quota.status === "ok" ? "#22c55e" : "#64748b",
            }}
          />
          <span style={{ fontWeight: 600, color: "var(--text-main)" }}>{quota.harness_title || quota.agent_id}</span>
          <span style={{ marginLeft: "auto", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            {summary(quota)}
          </span>
        </div>
      ))}
    </div>
  );
}
