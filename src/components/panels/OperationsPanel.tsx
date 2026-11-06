import { startTransition, useCallback, useEffect, useState } from "react";
import { invoke } from "../../lib/tauri";
import SystemTab from "./hub/SystemTab";
import McpToolsPanel from "./hub/McpToolsPanel";
import UsagePanel from "./operations/UsagePanel";
import WakesPanel from "./operations/WakesPanel";
import { ScorecardTab } from "./hub/ScorecardTab";
import type { AgentRecord, BudgetStatus, ProviderQuota, WakeRecord } from "./hub/types";
import { SubtabBar, SubtabItem } from "../common/telemetry/SubtabBar";
import { NOC_COLORS, NOC_SURFACES } from "../common/telemetry/tokens";

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
  const [visitedTabs, setVisitedTabs] = useState<Set<OpsTab>>(() => new Set(["system"]));
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
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "1rem", flexWrap: "wrap", marginBottom: "1rem", paddingBottom: "0.75rem", borderBottom: "1px solid rgba(148, 163, 184, 0.14)" }}>
        <div style={{ display: "flex", alignItems: "baseline", gap: "0.75rem", flexWrap: "wrap" }}>
          <h2 style={{ margin: 0, fontSize: "1.4rem", fontWeight: 800, color: NOC_SURFACES.textPrimary, letterSpacing: "-0.01em" }}>
            Operations
          </h2>
          <span style={{ fontSize: "0.78rem", color: NOC_SURFACES.textMuted }}>
            System telemetry, provider budgets &amp; quotas, MCP tools, wakes &amp; team scorecard
          </span>
        </div>
        <SubtabBar
          ariaLabel="Operations subtabs"
          tabs={OPS_SUBTABS}
          activeId={opsTab}
          onSelect={(id) =>
            startTransition(() => {
              setVisitedTabs((prev) => (prev.has(id) ? prev : new Set(prev).add(id)));
              setOpsTab(id);
            })
          }
        />
      </div>

      <div style={{ marginBottom: "1rem" }}>
        {error && <span style={{ color: NOC_COLORS.crit, fontSize: "0.85rem", background: "rgba(239, 68, 68, 0.1)", padding: "0.25rem 0.6rem", borderRadius: "6px", border: "1px solid rgba(239, 68, 68, 0.3)" }}>Error: {error}</span>}
        {status && !error && <span style={{ color: NOC_COLORS.ok, fontSize: "0.85rem", background: "rgba(16, 185, 129, 0.12)", padding: "0.25rem 0.6rem", borderRadius: "6px", border: "1px solid rgba(16, 185, 129, 0.3)", display: "inline-flex", alignItems: "center", gap: "0.35rem" }}>
          <div style={{ width: "6px", height: "6px", borderRadius: "50%", background: NOC_COLORS.ok }} /> {status}
        </span>}
      </div>

      <div style={{ display: opsTab === "system" ? "block" : "none" }}>
        <SystemTab />
      </div>
      {visitedTabs.has("tools") && (
        <div style={{ display: opsTab === "tools" ? "block" : "none" }}>
          <McpToolsPanel />
        </div>
      )}
      {visitedTabs.has("usage") && (
        <div style={{ display: opsTab === "usage" ? "block" : "none" }}>
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
        </div>
      )}
      {visitedTabs.has("wakes") && (
        <div style={{ display: opsTab === "wakes" ? "block" : "none" }}>
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
        </div>
      )}
      {visitedTabs.has("scorecard") && (
        <div style={{ display: opsTab === "scorecard" ? "block" : "none" }}>
          <ScorecardTab />
        </div>
      )}
    </div>
  );
}
