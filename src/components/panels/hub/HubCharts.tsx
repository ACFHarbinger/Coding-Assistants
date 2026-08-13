import type { BudgetStatus, ProviderQuota, ProviderQuotaWindow } from "./types";


/**
 * Providers whose adapter queries a live process/API on every call with no
 * staleness risk (Codex's `codex app-server` rate limits, Grok's billing
 * snapshot). Every other provider — including Claude Code and Antigravity
 * CLI, which expose no official usage-budget command — shows a last-refreshed
 * timestamp and a manual refresh control instead.
 */
export const LIVE_QUOTA_AGENT_IDS = new Set(["chat", "grok"]);

export const cardStyle: React.CSSProperties = {
  border: "1px solid var(--border-color)",
  borderRadius: "12px",
  padding: "1.5rem",
  background: "rgba(0, 0, 0, 0.3)",
  boxShadow: "0 4px 6px rgba(0,0,0,0.1)",
  /* Narrow to compositable properties only — avoids layout thrash on hover. */
  transition: "border-color 0.2s ease, box-shadow 0.2s ease"
};

export const inputStyle: React.CSSProperties = {
  padding: '0.75rem',
  borderRadius: '8px',
  background: 'rgba(0,0,0,0.4)',
  color: 'white',
  border: '1px solid var(--border-color)',
  outline: 'none',
  transition: 'border-color 0.2s'
};

export function WakePolicyCheckbox({
  checked,
  onChange,
  title,
  description,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  title: string;
  description: string;
}) {
  return (
    <label style={{ display: "flex", alignItems: "flex-start", gap: "0.9rem", cursor: "pointer", padding: "0.8rem", borderRadius: "10px", border: checked ? "1px solid #a78bfa" : "1px solid rgba(100, 116, 139, 0.65)", background: checked ? "rgba(124, 58, 237, 0.18)" : "rgba(15, 23, 42, 0.58)", transition: "background 0.15s ease, border-color 0.15s ease" }}>
      <input
        type="checkbox"
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
        style={{ position: "absolute", opacity: 0, width: 1, height: 1 }}
      />
      <span aria-hidden="true" style={{ display: "grid", placeItems: "center", flex: "0 0 auto", width: "1.35rem", height: "1.35rem", marginTop: "0.1rem", borderRadius: "0.35rem", border: checked ? "2px solid #c4b5fd" : "2px solid #64748b", background: checked ? "#7c3aed" : "#0f172a", color: "#fff", fontSize: "0.95rem", fontWeight: 800, boxShadow: checked ? "0 0 0 3px rgba(167, 139, 250, 0.24)" : "inset 0 0 0 1px rgba(255, 255, 255, 0.04)", transition: "background 0.15s ease, border-color 0.15s ease, box-shadow 0.15s ease" }}>
        {checked ? "✓" : ""}
      </span>
      <span>
        <span style={{ display: "block", fontSize: "1rem", fontWeight: 600, color: checked ? "#ede9fe" : "var(--text-main)", marginBottom: "0.25rem" }}>{title}</span>
        <span style={{ display: "block", fontSize: "0.85rem", color: "var(--text-muted)", lineHeight: 1.45 }}>{description}</span>
      </span>
    </label>
  );
}

export function UsageChart({ budgets }: { budgets: BudgetStatus[] }) {
  if (budgets.length === 0) return null;
  const chartWidth = 760;
  const rowHeight = 42;
  const labelWidth = 120;
  const barWidth = chartWidth - labelWidth - 150;
  return (
    <div style={{ ...cardStyle, display: "grid", gap: "0.75rem" }}>
      <div style={{ display: "flex", justifyContent: "space-between", gap: "1rem", flexWrap: "wrap" }}>
        <h3 style={{ margin: 0, color: "var(--text-main)" }}>Budget utilization</h3>
        <div style={{ display: "flex", gap: "1rem", color: "var(--text-muted)", fontSize: "0.8rem" }}>
          <span><i style={{ display: "inline-block", width: 10, height: 10, borderRadius: 2, background: "var(--primary)", marginRight: 5 }} />Used</span>
          <span><i style={{ display: "inline-block", width: 10, height: 10, borderRadius: 2, background: "#334155", marginRight: 5 }} />Available</span>
        </div>
      </div>
      <svg viewBox={`0 0 ${chartWidth} ${budgets.length * rowHeight}`} width="100%" role="img" aria-label="Used and available budget by agent" style={{ minHeight: 120 }}>
        {budgets.map((budget, index) => {
          const used = Math.min(budget.limit_units, Math.max(0, budget.spent_units));
          const usedWidth = budget.limit_units > 0 ? (used / budget.limit_units) * barWidth : 0;
          const y = index * rowHeight + 8;
          return <g key={budget.agent_id}>
            <text x="0" y={y + 17} fill="var(--text-main)" fontSize="13">{budget.agent_id}</text>
            <rect x={labelWidth} y={y} width={barWidth} height="22" rx="5" fill="#334155" />
            <rect x={labelWidth} y={y} width={usedWidth} height="22" rx="5" fill="var(--primary)" />
            <text x={labelWidth + barWidth + 12} y={y + 15} fill="var(--text-muted)" fontSize="12">{budget.spent_units} / {budget.limit_units}</text>
          </g>;
        })}
      </svg>
    </div>
  );
}

export function QuotaChart({
  quotas,
  refreshingIds,
  onRefreshOne,
}: {
  quotas: ProviderQuota[];
  refreshingIds: Set<string>;
  onRefreshOne: (agentId: string) => void;
}) {
  const formatReset = (timestamp?: number | null) => timestamp
    ? `resets ${new Date(timestamp * 1000).toLocaleString()}`
    : "reset time unavailable";
  const formatFetchedAt = (timestamp: number) =>
    `last refreshed ${new Date(timestamp * 1000).toLocaleString()}`;
  const windowName = (window: ProviderQuotaWindow) => {
    if (!window.window_minutes) return window.label;
    if (window.window_minutes <= 360) return `${window.label} · hourly window`;
    if (window.window_minutes >= 28 * 24 * 60) return `${window.label} · monthly window`;
    if (window.window_minutes >= 6 * 24 * 60) return `${window.label} · weekly window`;
    return `${window.label} · ${window.window_minutes} min`;
  };
  return (
    <div style={{ ...cardStyle, display: "grid", gap: "1rem" }}>
      <div style={{ display: "flex", justifyContent: "space-between", gap: "1rem", flexWrap: "wrap" }}>
        <div>
          <h3 style={{ margin: 0, color: "var(--text-main)" }}>Provider quota remaining</h3>
          <p style={{ margin: "0.35rem 0 0", color: "var(--text-muted)", fontSize: "0.82rem" }}>Account limits reported by each harness provider, separate from local Shared Hub budgets.</p>
        </div>
        <div style={{ display: "flex", gap: "1rem", color: "var(--text-muted)", fontSize: "0.8rem" }}>
          <span><i style={{ display: "inline-block", width: 10, height: 10, borderRadius: 2, background: "var(--primary)", marginRight: 5 }} />Remaining</span>
          <span><i style={{ display: "inline-block", width: 10, height: 10, borderRadius: 2, background: "#334155", marginRight: 5 }} />Used</span>
        </div>
      </div>
      <div style={{ display: "grid", gap: "1.25rem" }}>
        {quotas.map((quota) => {
          const families = Array.from(
            new Set(quota.windows.map((w) => w.family).filter(Boolean))
          ) as string[];

          return (
            <div key={quota.agent_id} style={{ display: "grid", gap: "0.6rem", background: "rgba(0, 0, 0, 0.2)", padding: "0.85rem 1rem", borderRadius: "10px", border: "1px solid var(--border-color)" }}>
              <div style={{ display: "flex", justifyContent: "space-between", gap: "1rem", flexWrap: "wrap", alignItems: "center" }}>
                <strong style={{ color: "var(--primary)", fontSize: "1.02rem" }}>
                  {quota.harness_title || `${quota.agent_id} · ${quota.provider}`}
                </strong>
                <div style={{ display: "flex", alignItems: "center", gap: "0.6rem" }}>
                  <span style={{ color: quota.status === "ok" ? "#22c55e" : "var(--text-muted)", fontSize: "0.82rem", fontWeight: 500 }}>
                    {LIVE_QUOTA_AGENT_IDS.has(quota.agent_id)
                      ? (quota.status === "ok" ? "live quota" : "unavailable")
                      : formatFetchedAt(quota.fetched_at)}
                  </span>
                  {!LIVE_QUOTA_AGENT_IDS.has(quota.agent_id) && (
                    <button
                      className="btn-secondary"
                      style={{ padding: "0.25rem 0.6rem", fontSize: "0.78rem" }}
                      disabled={refreshingIds.has(quota.agent_id)}
                      onClick={() => onRefreshOne(quota.agent_id)}
                    >
                      {refreshingIds.has(quota.agent_id) ? "Refreshing…" : "Refresh"}
                    </button>
                  )}
                </div>
              </div>
              {quota.windows.length === 0 ? (
                <span style={{ color: "var(--text-muted)", fontSize: "0.82rem" }}>{quota.detail || "No provider quota windows returned."}</span>
              ) : families.length > 0 ? (
                families.map((family) => {
                  const familyWindows = quota.windows.filter((w) => w.family === family);
                  return (
                    <div key={`${quota.agent_id}-${family}`} style={{ display: "grid", gap: "0.5rem", marginTop: "0.25rem" }}>
                      <div style={{ fontSize: "0.86rem", fontWeight: 600, color: "var(--text-main)", opacity: 0.9, letterSpacing: "0.02em" }}>
                        {family}
                      </div>
                      {familyWindows.map((window) => (
                        <div key={`${quota.agent_id}-${family}-${window.label}`} style={{ display: "grid", gap: "0.25rem", paddingLeft: "0.5rem" }}>
                          <div style={{ display: "flex", justifyContent: "space-between", gap: "1rem", color: "var(--text-muted)", fontSize: "0.8rem" }}>
                            <span>{windowName(window)} · {formatReset(window.resets_at)}</span>
                            <strong style={{ color: window.remaining_percent === 0 ? "#ef4444" : "var(--text-main)" }}>
                              {window.remaining_percent}% remaining
                            </strong>
                          </div>
                          <div style={{ height: 12, background: "#334155", borderRadius: 6, overflow: "hidden" }}>
                            <div
                              style={{
                                width: `${window.remaining_percent}%`,
                                height: "100%",
                                background: window.remaining_percent < 10 ? "#ef4444" : window.remaining_percent < 25 ? "#eab308" : "var(--primary)",
                                transition: "width 0.3s ease",
                              }}
                            />
                          </div>
                        </div>
                      ))}
                    </div>
                  );
                })
              ) : (
                quota.windows.map((window) => (
                  <div key={`${quota.agent_id}-${window.label}`} style={{ display: "grid", gap: "0.25rem" }}>
                    <div style={{ display: "flex", justifyContent: "space-between", gap: "1rem", color: "var(--text-muted)", fontSize: "0.8rem" }}>
                      <span>{windowName(window)} · {formatReset(window.resets_at)}</span>
                      <strong style={{ color: window.remaining_percent === 0 ? "#ef4444" : "var(--text-main)" }}>
                        {window.remaining_percent}% remaining
                      </strong>
                    </div>
                    <div style={{ height: 12, background: "#334155", borderRadius: 6, overflow: "hidden" }}>
                      <div
                        style={{
                          width: `${window.remaining_percent}%`,
                          height: "100%",
                          background: window.remaining_percent < 10 ? "#ef4444" : window.remaining_percent < 25 ? "#eab308" : "var(--primary)",
                          transition: "width 0.3s ease",
                        }}
                      />
                    </div>
                  </div>
                ))
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

