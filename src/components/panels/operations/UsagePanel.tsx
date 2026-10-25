import { UsageChart, QuotaChart, inputStyle } from "../hub/HubCharts";
import type { AgentRecord, BudgetStatus, ProviderQuota } from "../hub/types";
import { KpiCardGrid, type KpiCardItem } from "../../common/telemetry/status";
import { Meter } from "../../common/telemetry/gauges";
import { nocPanelStyle, nocCardDenseStyle } from "../../common/telemetry/panel";
import { NOC_COLORS, NOC_SURFACES } from "../../common/telemetry/tokens";

export default function UsagePanel({
  agents,
  budgetAgent,
  setBudgetAgent,
  budgetLimit,
  setBudgetLimit,
  setBudget,
  refreshBudgets,
  refreshQuotas,
  refreshStaleQuotas,
  budgets,
  quotas,
  refreshingQuotaIds,
  refreshSingleQuota,
  budgetSpend,
  setBudgetSpend,
  recordSpend,
  resumeBudget,
}: {
  agents: AgentRecord[];
  budgetAgent: string;
  setBudgetAgent: (next: string) => void;
  budgetLimit: string;
  setBudgetLimit: (next: string) => void;
  setBudget: () => void;
  refreshBudgets: () => void;
  refreshQuotas: () => void;
  refreshStaleQuotas: () => void;
  budgets: BudgetStatus[];
  quotas: ProviderQuota[];
  refreshingQuotaIds: Set<string>;
  refreshSingleQuota: (agentId: string) => void;
  budgetSpend: string;
  setBudgetSpend: (next: string) => void;
  recordSpend: (agent: string) => void;
  resumeBudget: (agent: string) => void;
}) {
  const activeBudgets = budgets.filter((b) => !b.paused).length;
  const pausedAgents = budgets.filter((b) => b.paused).length;
  const liveQuotas = quotas.filter((q) => q.status === "ok").length;

  const kpiCards: KpiCardItem[] = [
    {
      title: "Active Budgets",
      subtitle: "Enforcing wake caps",
      value: String(activeBudgets),
      accent: NOC_COLORS.cyan,
      provenance: "measured",
      footer: `${budgets.length} total configured`,
    },
    {
      title: "Paused Agents",
      subtitle: "Limit reached / blocked",
      value: String(pausedAgents),
      accent: pausedAgents > 0 ? NOC_COLORS.warn : NOC_COLORS.ok,
      provenance: "measured",
      footer: pausedAgents > 0 ? "Requires resume action" : "All active agents running",
    },
    {
      title: "Live Quota Providers",
      subtitle: "Harness rate telemetry",
      value: String(liveQuotas),
      accent: NOC_COLORS.indigo,
      provenance: "measured",
      footer: `${quotas.length} total providers`,
    },
  ];

  return (
    <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1.5rem" }}>
      {/* KPI Overview Summary (KIMI-UI-1 / CODEX-UI-3) */}
      <KpiCardGrid cards={kpiCards} testId="usage-kpi-grid" />

      {/* Budget Configuration Bar */}
      <div style={{ ...nocPanelStyle, display: "grid", gap: "1rem" }}>
        <div>
          <h3 style={{ margin: 0, fontSize: "1.2rem", color: NOC_SURFACES.textPrimary }}>Agent Usage</h3>
          <p style={{ margin: "0.25rem 0 0", color: NOC_SURFACES.textMuted, fontSize: "0.88rem" }}>
            Configure caller-defined units such as provider calls, tokens, or spend. Reaching a limit blocks new wakes.
          </p>
        </div>
        <div style={{ display: "flex", gap: "0.75rem", flexWrap: "wrap", alignItems: "center" }}>
          <select value={budgetAgent} onChange={(e) => setBudgetAgent(e.target.value)} style={inputStyle}>
            <option value="">Select agent</option>
            {agents.map((agent) => <option key={agent.id} value={agent.id}>{agent.display_name}</option>)}
          </select>
          <input
            type="number"
            min="0"
            step="1"
            value={budgetLimit}
            onChange={(e) => setBudgetLimit(e.target.value)}
            style={{ ...inputStyle, width: 130 }}
            placeholder="Limit"
          />
          <button className="btn-primary" onClick={setBudget} disabled={!budgetAgent}>
            Set / reset budget
          </button>
          <button className="btn-secondary" onClick={refreshBudgets}>
            Refresh
          </button>
          <button className="btn-secondary" onClick={refreshQuotas}>
            Refresh provider quotas
          </button>
          <button className="btn-secondary" onClick={refreshStaleQuotas}>
            Refresh all stale quotas
          </button>
        </div>
      </div>

      {/* Horizontal Comparison Charts (Primary view) */}
      <UsageChart budgets={budgets} />
      <QuotaChart quotas={quotas} refreshingIds={refreshingQuotaIds} onRefreshOne={refreshSingleQuota} />

      {/* Individual Agent Budget Bento Rows */}
      <div style={{ display: "flex", flexDirection: "column", gap: "0.85rem" }}>
        {budgets.length === 0 && <p style={{ color: NOC_SURFACES.textMuted }}>No budgets configured.</p>}
        {budgets.map((budget) => {
          const ratio = budget.limit_units > 0 ? (budget.spent_units / budget.limit_units) * 100 : 0;
          const tone = budget.paused ? "crit" : ratio >= 90 ? "crit" : ratio >= 75 ? "warn" : "ok";
          return (
            <div
              key={budget.agent_id}
              style={{
                ...nocCardDenseStyle,
                display: "grid",
                gap: "0.75rem",
                borderLeft: `3px solid ${budget.paused ? NOC_COLORS.crit : NOC_COLORS.cyan}`,
              }}
            >
              <div style={{ display: "flex", justifyContent: "space-between", gap: "1rem", alignItems: "center", flexWrap: "wrap" }}>
                <div>
                  <strong style={{ color: NOC_COLORS.cyan, fontSize: "0.95rem" }}>{budget.agent_id}</strong>
                  <div className="tabular-nums" style={{ color: NOC_SURFACES.textMuted, fontSize: "0.84rem", marginTop: "0.15rem" }}>
                    {budget.spent_units} / {budget.limit_units} units · {budget.paused ? "paused" : "active"}
                  </div>
                </div>
                <div style={{ display: "flex", gap: "0.5rem", alignItems: "center" }}>
                  <input
                    type="number"
                    min="0"
                    step="1"
                    value={budgetSpend}
                    onChange={(e) => setBudgetSpend(e.target.value)}
                    style={{ ...inputStyle, width: 90 }}
                  />
                  <button className="btn-secondary" onClick={() => recordSpend(budget.agent_id)}>
                    Record usage
                  </button>
                  {budget.paused && (
                    <button className="btn-primary" onClick={() => resumeBudget(budget.agent_id)}>
                      Resume
                    </button>
                  )}
                </div>
              </div>
              <Meter
                percent={ratio}
                tone={tone}
                label="Budget Utilization"
                detail={`${ratio.toFixed(0)}%`}
              />
            </div>
          );
        })}
      </div>
    </div>
  );
}
