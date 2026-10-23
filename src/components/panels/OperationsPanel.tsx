import { startTransition, useCallback, useEffect, useState } from "react";
import { invoke } from "../../lib/tauri";
import SystemTab from "./hub/SystemTab";
import McpToolsPanel from "./hub/McpToolsPanel";
import UsagePanel from "./operations/UsagePanel";
import WakesPanel from "./operations/WakesPanel";
import { ScorecardTab } from "./hub/ScorecardTab";
import type { AgentRecord, BudgetStatus, ProviderQuota, WakeRecord } from "./hub/types";
import { SubtabBar, SubtabItem } from "../common/telemetry/SubtabBar";

type OpsTab = "system" | "usage" | "tools" | "wakes" | "scorecard";

const OPS_SUBTABS: SubtabItem<OpsTab>[] = [
  { id: "system", label: "System" },
  { id: "usage", label: "Usage" },
  { id: "tools", label: "Tools" },
  { id: "wakes", label: "Wakes" },
  { id: "scorecard", label: "Scorecard" },
];

export default function OperationsPanel() {
  const [opsTab, setOpsTab] = useState<OpsTab>("system");
  const [agents, setAgents] = useState<AgentRecord[]>([]);
  const [error, setError] = useState<string>("");
  const [status, setStatus] = useState<string>("");

  const [budgets, setBudgets] = useState<BudgetStatus[]>([]);
  const [quotas, setQuotas] = useState<ProviderQuota[]>([]);
  const [refreshingQuotaIds, setRefreshingQuotaIds] = useState<Set<string>>(new Set());
  const [budgetAgent, setBudgetAgent] = useState("");
  const [budgetLimit, setBudgetLimit] = useState("100");
  const [budgetSpend, setBudgetSpend] = useState("1");

  const [wakes, setWakes] = useState<WakeRecord[]>([]);
  const [wakeTarget, setWakeTarget] = useState("claude");
  const [wakeReason, setWakeReason] = useState("");

  const run = useCallback(async <T,>(label: string, fn: () => Promise<T>, pendingMessage?: string): Promise<T | null> => {
    setError("");
    if (pendingMessage) {
      setStatus(pendingMessage);
    }
    try {
      const result = await fn();
      setStatus(label);
      return result;
    } catch (e) {
      setError(String(e));
      setStatus("");
      return null;
    }
  }, []);

  const refreshAgents = useCallback(async () => {
    const list = await invoke<AgentRecord[]>("hub_list_agents");
    setAgents(list);
    return list;
  }, []);

  const refreshBudgets = useCallback(async () => {
    const statuses = await Promise.all(
      agents.map(async (agent) =>
        invoke<BudgetStatus | null>("hub_get_budget", { agent: agent.id })
      ),
    );
    setBudgets(statuses.filter((status): status is BudgetStatus => status !== null));
  }, [agents]);

  const refreshQuotas = useCallback(async () => {
    const list = await run("quotas refreshed", () => invoke<ProviderQuota[]>("hub_get_provider_quotas", { forceRefresh: false }));
    if (list) setQuotas(list);
  }, [run]);

  const refreshStaleQuotas = useCallback(async () => {
    const list = await run("stale quotas refreshed", () => invoke<ProviderQuota[]>("hub_get_provider_quotas", { forceRefresh: true }), "Probing provider quotas…");
    if (list) setQuotas(list);
  }, [run]);

  const refreshSingleQuota = useCallback(async (agentId: string) => {
    setRefreshingQuotaIds((prev) => new Set(prev).add(agentId));
    try {
      const updated = await invoke<ProviderQuota>("hub_refresh_provider_quota", { agentId });
      setQuotas((prev) => prev.map((q) => (q.agent_id === agentId ? updated : q)));
    } catch (e) {
      setError(String(e));
    } finally {
      setRefreshingQuotaIds((prev) => {
        const next = new Set(prev);
        next.delete(agentId);
        return next;
      });
    }
  }, []);

  const setBudget = async () => {
    if (!budgetAgent || !Number.isFinite(Number(budgetLimit))) return;
    await run("budget saved", () => invoke("hub_set_agent_budget", {
      agent: budgetAgent,
      limit: Number(budgetLimit),
    }));
    await refreshBudgets();
  };

  const recordSpend = async (agent: string) => {
    const amount = Number(budgetSpend);
    if (!Number.isFinite(amount) || amount < 0) return;
    await run("budget usage recorded", () => invoke("hub_record_budget_usage", { agent, amount }));
    await refreshBudgets();
  };

  const resumeBudget = async (agent: string) => {
    await run("agent resumed", () => invoke("hub_resume_agent", { agent }));
    await refreshBudgets();
  };

  const refreshWakes = useCallback(async () => {
    const list = await run("wakes refreshed", () => invoke<WakeRecord[]>("hub_list_wakes"));
    if (list) setWakes(list);
  }, [run]);

  const requestWake = async () => {
    await run("wake requested", () =>
      invoke("hub_request_wake", {
        target: wakeTarget,
        reason: wakeReason || null,
        messageId: null,
        humanGate: true,
      })
    );
    setWakeReason("");
    await refreshWakes();
  };

  useEffect(() => {
    refreshAgents().then((list) => {
      const firstAgent = list.find((a) => a.id !== "human");
      if (firstAgent) setWakeTarget(firstAgent.id);
    }).catch((e) => setError(String(e)));
  }, [refreshAgents]);

  useEffect(() => {
    if (opsTab === "usage") {
      refreshBudgets();
      refreshQuotas();
    } else if (opsTab === "wakes") {
      refreshWakes();
    }
  }, [opsTab, refreshBudgets, refreshQuotas, refreshWakes]);

  return (
    <div className="glass-card fade-in" style={{ animationDelay: "0.1s" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "1rem", flexWrap: "wrap", marginBottom: "1rem" }}>
        <h2 style={{ margin: 0, fontSize: "1.5rem", background: "linear-gradient(to right, #fff, var(--primary))", WebkitBackgroundClip: "text", WebkitTextFillColor: "transparent" }}>
          Operations
        </h2>
        <SubtabBar
          ariaLabel="Operations subtabs"
          tabs={OPS_SUBTABS}
          activeId={opsTab}
          onSelect={(id) => startTransition(() => setOpsTab(id))}
        />
      </div>

      <div style={{ marginBottom: "1rem" }}>
        {error && <span style={{ color: "#ef4444", fontSize: "0.85rem", background: "rgba(239, 68, 68, 0.1)", padding: "0.2rem 0.5rem", borderRadius: "4px" }}>Error: {error}</span>}
        {status && !error && <span style={{ color: "#22c55e", fontSize: "0.85rem", background: "rgba(34, 197, 94, 0.1)", padding: "0.2rem 0.5rem", borderRadius: "4px", display: "inline-flex", alignItems: "center", gap: "0.25rem" }}>
          <div style={{ width: "6px", height: "6px", borderRadius: "50%", background: "#22c55e" }} /> {status}
        </span>}
      </div>

      {opsTab === "system" && <SystemTab />}
      {opsTab === "tools" && <McpToolsPanel />}
      {opsTab === "usage" && (
        <UsagePanel
          agents={agents}
          budgetAgent={budgetAgent}
          setBudgetAgent={setBudgetAgent}
          budgetLimit={budgetLimit}
          setBudgetLimit={setBudgetLimit}
          setBudget={setBudget}
          refreshBudgets={refreshBudgets}
          refreshQuotas={refreshQuotas}
          refreshStaleQuotas={refreshStaleQuotas}
          budgets={budgets}
          quotas={quotas}
          refreshingQuotaIds={refreshingQuotaIds}
          refreshSingleQuota={refreshSingleQuota}
          budgetSpend={budgetSpend}
          setBudgetSpend={setBudgetSpend}
          recordSpend={recordSpend}
          resumeBudget={resumeBudget}
        />
      )}
      {opsTab === "wakes" && (
        <WakesPanel
          agents={agents}
          wakeTarget={wakeTarget}
          setWakeTarget={setWakeTarget}
          wakeReason={wakeReason}
          setWakeReason={setWakeReason}
          requestWake={requestWake}
          refreshWakes={refreshWakes}
          wakes={wakes}
        />
      )}
      {opsTab === "scorecard" && <ScorecardTab />}
    </div>
  );
}
