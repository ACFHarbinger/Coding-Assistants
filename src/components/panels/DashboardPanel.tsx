import { useCallback, useEffect, useState } from "react";
import { invoke } from "../../lib/tauri";

interface AgentRecord { id: string; display_name: string; }
interface BudgetStatus { agent_id: string; limit_units: number; spent_units: number; paused: boolean; updated_at: string; }
interface AgentMetrics { agent_id: string; lines_written: number; tokens_used: number; tokens_cached: number; provider_calls: number; output_chars: number; updated_at: string; }

const cardStyle: React.CSSProperties = { border: "1px solid var(--border-color)", borderRadius: "12px", padding: "1.25rem", background: "rgba(0,0,0,0.3)" };
const number = (value: number) => new Intl.NumberFormat().format(value);

export default function DashboardPanel({ agents }: { agents: AgentRecord[] }) {
  const [budgets, setBudgets] = useState<BudgetStatus[]>([]);
  const [metrics, setMetrics] = useState<AgentMetrics[]>([]);
  const [error, setError] = useState("");
  const [updated, setUpdated] = useState("");

  const refresh = useCallback(async () => {
    setError("");
    try {
      const [nextMetrics, nextBudgets] = await Promise.all([
        invoke<AgentMetrics[]>("hub_list_agent_metrics"),
        Promise.all(agents.map((agent) => invoke<BudgetStatus | null>("hub_get_budget", { agent: agent.id }))),
      ]);
      setMetrics(nextMetrics);
      setBudgets(nextBudgets.filter((budget): budget is BudgetStatus => budget !== null));
      setUpdated(new Date().toLocaleTimeString());
    } catch (e) { setError(String(e)); }
  }, [agents]);

  useEffect(() => { if (agents.length > 0) void refresh(); }, [agents, refresh]);

  const metricFor = (id: string) => metrics.find((metric) => metric.agent_id === id);
  const budgetFor = (id: string) => budgets.find((budget) => budget.agent_id === id);
  const total = (field: keyof AgentMetrics) => metrics.reduce((sum, metric) => sum + Number(metric[field] || 0), 0);

  return <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1.5rem" }}>
    <div style={{ ...cardStyle, display: "flex", justifyContent: "space-between", gap: "1rem", alignItems: "center", flexWrap: "wrap" }}>
      <div><h3 style={{ margin: 0, color: "var(--text-main)" }}>Agent Telemetry</h3><p style={{ margin: "0.4rem 0 0", color: "var(--text-muted)", fontSize: "0.85rem" }}>Local cumulative usage from the Shared Hub. Token counts are whitespace estimates until providers report exact usage.</p></div>
      <button className="btn-secondary" onClick={() => void refresh()}>Refresh{updated ? ` · ${updated}` : ""}</button>
    </div>
    {error && <div style={{ ...cardStyle, color: "#ef4444" }}>{error}</div>}
    <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))", gap: "1rem" }}>
      {["provider_calls", "lines_written", "tokens_used", "tokens_cached"].map((field) => <div key={field} style={cardStyle}>
        <div style={{ color: "var(--text-muted)", fontSize: "0.78rem", textTransform: "uppercase" }}>{field.replace(/_/g, " ")}</div>
        <strong style={{ display: "block", color: "var(--primary)", fontSize: "1.7rem", marginTop: "0.4rem" }}>{number(total(field as keyof AgentMetrics))}</strong>
      </div>)}
    </div>
    <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
      {agents.map((agent) => { const metric = metricFor(agent.id); const budget = budgetFor(agent.id); const percent = budget ? Math.min(100, budget.spent_units / budget.limit_units * 100) : 0; return <div key={agent.id} style={cardStyle}>
        <div style={{ display: "flex", justifyContent: "space-between", gap: "1rem", flexWrap: "wrap" }}><strong style={{ color: "var(--primary)" }}>{agent.display_name}</strong><span style={{ color: budget?.paused ? "#ef4444" : "var(--text-muted)", fontSize: "0.85rem" }}>{budget ? `${budget.spent_units} / ${budget.limit_units} budget units${budget.paused ? " · paused" : ""}` : "No budget configured"}</span></div>
        {budget && <div style={{ height: 7, background: "rgba(255,255,255,0.1)", borderRadius: 4, margin: "0.8rem 0" }}><div style={{ width: `${percent}%`, height: "100%", borderRadius: 4, background: budget.paused ? "#ef4444" : "var(--primary)" }} /></div>}
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(120px, 1fr))", gap: "0.75rem", color: "var(--text-muted)", fontSize: "0.85rem" }}>
          <span>Lines written<br /><b style={{ color: "var(--text-main)" }}>{number(metric?.lines_written || 0)}</b></span><span>Tokens used<br /><b style={{ color: "var(--text-main)" }}>{number(metric?.tokens_used || 0)} est.</b></span><span>Cached tokens<br /><b style={{ color: "var(--text-main)" }}>{number(metric?.tokens_cached || 0)} reported</b></span><span>Provider calls<br /><b style={{ color: "var(--text-main)" }}>{number(metric?.provider_calls || 0)}</b></span>
        </div>
      </div>; })}
      {agents.length === 0 && <div style={{ ...cardStyle, color: "var(--text-muted)" }}>No agents registered yet.</div>}
    </div>
  </div>;
}
