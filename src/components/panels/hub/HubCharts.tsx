import type { BalanceBreakdown, BudgetStatus, ProviderQuota, ProviderQuotaWindow } from "./types";
import { ProviderHealthDot } from "../harness/ProviderHealthChip";
import { useProviderHealth } from "../harness/useProviderHealth";
import { useOrchestrationPolicy } from "../harness/useOrchestrationPolicy";
import { PaygQuotaMeter } from "./PaygQuotaMeter";
import { LocalUsageMeter } from "./LocalUsageMeter";


/**
 * Providers whose adapter queries a live process/API on every call with no
 * staleness risk (Codex's `codex app-server` rate limits, Grok's billing
 * snapshot). Every other provider — including Claude Code and Antigravity
 * CLI, which expose no official usage-budget command — shows a last-refreshed
 * timestamp and a manual refresh control instead.
 */
export const LIVE_QUOTA_AGENT_IDS = new Set(["chat", "grok"]);

/**
 * `PaygQuotaMeter` formats whatever it is given as dollars, falling back to
 * `parseFloat` over the free-form `balance` string when `balance_info` is
 * absent. So only route a provider there when the figure really is currency:
 * structured `balance_info`, or a legacy `balance` string that reads as an
 * amount. Without this, a provider reporting a non-currency number (raw token
 * counts, say) renders as "Available Balance $1234567.00".
 */
export function isCurrencyBalance(balance?: string | null): boolean {
  return !!balance && /[$€£¥]|\b(?:USD|EUR|GBP|CNY)\b/i.test(balance);
}

/**
 * Format a minor currency unit (e.g. cents) into a standard formatted currency string.
 */
export function formatMinorCurrency(minor: number, currency = "USD"): string {
  const isNegative = minor < 0;
  const absMinor = Math.abs(minor);
  const dollars = (absMinor / 100).toFixed(2);
  const prefix = isNegative ? "-" : "";

  const curr = currency.trim().toUpperCase();
  switch (curr) {
    case "USD":
    case "$":
      return `${prefix}$${dollars}`;
    case "EUR":
    case "€":
      return `${prefix}€${dollars}`;
    case "GBP":
    case "£":
      return `${prefix}£${dollars}`;
    case "CNY":
    case "¥":
      return `${prefix}¥${dollars}`;
    default:
      return `${prefix}${dollars} ${curr}`;
  }
}

/**
 * Stacked horizontal breakdown bar showing spent vs budget vs free/bonus tokens (#303).
 */
export function BalanceBreakdownBar({
  breakdown,
  balanceText,
}: {
  breakdown: BalanceBreakdown;
  balanceText?: string | null;
}) {
  const { currency, spent_minor, budget_minor, free_minor } = breakdown;
  const remaining_budget = Math.max(0, budget_minor - spent_minor);
  const isOverBudget = budget_minor > 0 && spent_minor > budget_minor;

  const totalCapacity = Math.max(budget_minor, spent_minor) + free_minor;
  const spentPercent = totalCapacity > 0 ? Math.min(100, Math.max(0, (spent_minor / totalCapacity) * 100)) : 0;
  const remainingBudgetPercent = totalCapacity > 0 ? Math.min(100, Math.max(0, (remaining_budget / totalCapacity) * 100)) : 0;
  const freePercent = totalCapacity > 0 ? Math.min(100, Math.max(0, (free_minor / totalCapacity) * 100)) : 0;

  const remainingRatio = budget_minor > 0 ? (remaining_budget / budget_minor) * 100 : 0;
  const budgetColor =
    remainingRatio === 0
      ? "var(--text-muted)"
      : remainingRatio < 10
      ? "#ef4444"
      : remainingRatio < 25
      ? "#eab308"
      : "var(--primary)";

  const displayBalance =
    balanceText ||
    formatMinorCurrency(remaining_budget + free_minor, currency);

  return (
    <div style={{ display: "grid", gap: "0.35rem" }} data-testid="balance-breakdown-bar">
      <div style={{ display: "flex", justifyContent: "space-between", gap: "1rem", color: "var(--text-muted)", fontSize: "0.8rem", flexWrap: "wrap" }}>
        <span>Account balance</span>
        <strong style={{ color: isOverBudget ? "#ef4444" : "#22c55e" }}>
          {displayBalance}
        </strong>
      </div>
      <div
        role="progressbar"
        aria-label="Balance breakdown"
        style={{
          height: 12,
          background: "#334155",
          borderRadius: 6,
          overflow: "hidden",
          display: "flex",
          width: "100%",
        }}
      >
        {spentPercent > 0 && (
          <div
            data-testid="breakdown-spent"
            title={`Spent: ${formatMinorCurrency(spent_minor, currency)}`}
            style={{
              width: `${spentPercent}%`,
              height: "100%",
              background: isOverBudget ? "#ef4444" : "#38bdf8",
              transition: "width 0.3s ease",
            }}
          />
        )}
        {remainingBudgetPercent > 0 && (
          <div
            data-testid="breakdown-budget"
            title={`Remaining Budget: ${formatMinorCurrency(remaining_budget, currency)}`}
            style={{
              width: `${remainingBudgetPercent}%`,
              height: "100%",
              background: budgetColor,
              transition: "width 0.3s ease",
            }}
          />
        )}
        {freePercent > 0 && (
          <div
            data-testid="breakdown-free"
            title={`Free / Bonus: ${formatMinorCurrency(free_minor, currency)}`}
            style={{
              width: `${freePercent}%`,
              height: "100%",
              background: "#34d399",
              transition: "width 0.3s ease",
            }}
          />
        )}
      </div>
      <div style={{ display: "flex", justifyContent: "flex-start", gap: "0.85rem", flexWrap: "wrap", fontSize: "0.74rem", color: "var(--text-muted)" }}>
        {spent_minor > 0 && (
          <span style={{ display: "inline-flex", alignItems: "center", gap: "0.3rem" }}>
            <i style={{ width: 8, height: 8, borderRadius: 2, background: isOverBudget ? "#ef4444" : "#38bdf8", display: "inline-block" }} />
            Spent {formatMinorCurrency(spent_minor, currency)}
          </span>
        )}
        {budget_minor > 0 && (
          <span style={{ display: "inline-flex", alignItems: "center", gap: "0.3rem" }}>
            <i style={{ width: 8, height: 8, borderRadius: 2, background: budgetColor, display: "inline-block" }} />
            Budget {formatMinorCurrency(remaining_budget, currency)}
          </span>
        )}
        {free_minor > 0 && (
          <span style={{ display: "inline-flex", alignItems: "center", gap: "0.3rem" }}>
            <i style={{ width: 8, height: 8, borderRadius: 2, background: "#34d399", display: "inline-block" }} />
            Free {formatMinorCurrency(free_minor, currency)}
          </span>
        )}
      </div>
    </div>
  );
}

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
  const { healthMap } = useProviderHealth(30_000);
  const { policy, setAllowMeteredProbes } = useOrchestrationPolicy();
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
      <div style={{ display: "flex", justifyContent: "space-between", gap: "1rem", flexWrap: "wrap", alignItems: "center" }}>
        <div>
          <h3 style={{ margin: 0, color: "var(--text-main)" }}>Provider quota remaining</h3>
          <p style={{ margin: "0.35rem 0 0", color: "var(--text-muted)", fontSize: "0.82rem" }}>Account limits reported by each harness provider, separate from local Shared Hub budgets.</p>
        </div>
        <div style={{ display: "flex", gap: "1.25rem", alignItems: "center", flexWrap: "wrap" }}>
          <label
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: "0.45rem",
              fontSize: "0.78rem",
              color: policy.allow_metered_quota_probes ? "#38bdf8" : "var(--text-muted)",
              cursor: "pointer",
              background: policy.allow_metered_quota_probes ? "rgba(56, 189, 248, 0.1)" : "rgba(255, 255, 255, 0.04)",
              border: `1px solid ${policy.allow_metered_quota_probes ? "rgba(56, 189, 248, 0.3)" : "var(--border-color)"}`,
              padding: "0.3rem 0.6rem",
              borderRadius: "8px",
              userSelect: "none",
            }}
            title="Allow metered usage probes. When off, checking Antigravity CLI, OpenCode, and Muse is skipped to save tokens."
          >
            <input
              type="checkbox"
              checked={policy.allow_metered_quota_probes}
              onChange={(e) => void setAllowMeteredProbes(e.target.checked)}
              style={{ accentColor: "var(--primary)", cursor: "pointer" }}
            />
            <span>Allow metered probes</span>
          </label>
          <div style={{ display: "flex", gap: "1rem", color: "var(--text-muted)", fontSize: "0.8rem" }}>
            <span><i style={{ display: "inline-block", width: 10, height: 10, borderRadius: 2, background: "var(--primary)", marginRight: 5 }} />Remaining</span>
            <span><i style={{ display: "inline-block", width: 10, height: 10, borderRadius: 2, background: "#334155", marginRight: 5 }} />Used</span>
          </div>
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
                <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
                  <ProviderHealthDot health={healthMap[quota.agent_id]} titlePrefix={`${quota.harness_title || quota.agent_id} CLI`} />
                  <strong style={{ color: "var(--primary)", fontSize: "1.02rem" }}>
                    {quota.harness_title || `${quota.agent_id} · ${quota.provider}`}
                  </strong>
                </div>
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
              {quota.balance_breakdown && (quota.windows.length > 0 || quota.balance_info?.kind === "spend" || !quota.balance_info) ? (
                <BalanceBreakdownBar breakdown={quota.balance_breakdown} balanceText={quota.balance} />
              ) : quota.balance && (quota.windows.length > 0 || quota.balance_info?.kind === "spend" || quota.balance.toLowerCase().startsWith("spent")) ? (
                <div style={{ display: "grid", gap: "0.25rem" }}>
                  <div style={{ display: "flex", justifyContent: "space-between", gap: "1rem", color: "var(--text-muted)", fontSize: "0.8rem" }}>
                    <span>{quota.balance.toLowerCase().startsWith("spent") || quota.balance_info?.kind === "spend" ? "Period spend" : "Account balance"}</span>
                    <strong style={{ color: quota.balance.toLowerCase().startsWith("spent") || quota.balance_info?.kind === "spend" ? "#38bdf8" : "#22c55e" }}>
                      {quota.balance}
                    </strong>
                  </div>
                </div>
              ) : null}
              {((quota.balance_info && quota.balance_info.kind !== "spend") ||
                (isCurrencyBalance(quota.balance) && !quota.balance?.toLowerCase().startsWith("spent") && quota.windows.length === 0)) ? (
                <>
                  <PaygQuotaMeter quota={quota} />
                  {quota.local_usage && <LocalUsageMeter usage={quota.local_usage} />}
                </>
              ) : quota.windows.length === 0 ? (
                quota.local_usage ? (
                  <LocalUsageMeter usage={quota.local_usage} detail={quota.detail} />
                ) : (quota.balance || quota.balance_breakdown) ? null : (
                  <span style={{ color: "var(--text-muted)", fontSize: "0.82rem" }}>{quota.detail || "No provider quota windows returned."}</span>
                )
              ) : families.length > 0 ? (
                <>
                  {families.map((family) => {
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
                  })}
                  {quota.local_usage && <LocalUsageMeter usage={quota.local_usage} />}
                </>
              ) : (
                <>
                  {quota.windows.map((window) => (
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
                  ))}
                  {quota.local_usage && <LocalUsageMeter usage={quota.local_usage} />}
                </>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
