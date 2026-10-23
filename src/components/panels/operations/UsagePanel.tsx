import { UsageChart, QuotaChart, cardStyle, inputStyle } from "../hub/HubCharts";
import type { AgentRecord, BudgetStatus, ProviderQuota } from "../hub/types";

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
  return (
    <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1.5rem" }}>
      <div style={{ ...cardStyle, display: "grid", gap: "1rem" }}>
        <h3 style={{ margin: 0, fontSize: "1.2rem", color: "var(--text-main)" }}>Agent Usage</h3>
        <p style={{ margin: 0, color: "var(--text-muted)", fontSize: "0.9rem" }}>
          Configure caller-defined units such as provider calls, tokens, or spend. Reaching a limit blocks new wakes.
        </p>
        <div style={{ display: "flex", gap: "0.75rem", flexWrap: "wrap" }}>
          <select value={budgetAgent} onChange={(e) => setBudgetAgent(e.target.value)} style={inputStyle}>
            <option value="">Select agent</option>
            {agents.map((agent) => <option key={agent.id} value={agent.id}>{agent.display_name}</option>)}
          </select>
          <input type="number" min="0" step="1" value={budgetLimit} onChange={(e) => setBudgetLimit(e.target.value)} style={{ ...inputStyle, width: 130 }} placeholder="Limit" />
          <button className="btn-primary" onClick={setBudget} disabled={!budgetAgent}>Set / reset budget</button>
          <button className="btn-secondary" onClick={refreshBudgets}>Refresh</button>
          <button className="btn-secondary" onClick={refreshQuotas}>Refresh provider quotas</button>
          <button className="btn-secondary" onClick={refreshStaleQuotas}>Refresh all stale quotas</button>
        </div>
      </div>
      <UsageChart budgets={budgets} />
      <QuotaChart quotas={quotas} refreshingIds={refreshingQuotaIds} onRefreshOne={refreshSingleQuota} />
      <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
        {budgets.length === 0 && <p style={{ color: "var(--text-muted)" }}>No budgets configured.</p>}
        {budgets.map((budget) => (
          <div key={budget.agent_id} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", gap: "1rem", alignItems: "center", flexWrap: "wrap" }}>
            <div>
              <strong style={{ color: "var(--primary)" }}>{budget.agent_id}</strong>
              <div style={{ color: "var(--text-muted)", fontSize: "0.85rem" }}>
                {budget.spent_units} / {budget.limit_units} units · {budget.paused ? "paused" : "active"}
              </div>
            </div>
            <div style={{ display: "flex", gap: "0.5rem", alignItems: "center" }}>
              <input type="number" min="0" step="1" value={budgetSpend} onChange={(e) => setBudgetSpend(e.target.value)} style={{ ...inputStyle, width: 90 }} />
              <button className="btn-secondary" onClick={() => recordSpend(budget.agent_id)}>Record usage</button>
              {budget.paused && <button className="btn-primary" onClick={() => resumeBudget(budget.agent_id)}>Resume</button>}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
