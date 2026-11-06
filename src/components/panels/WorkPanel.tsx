import { startTransition, useCallback, useEffect, useState } from "react";
import { invoke } from "../../lib/tauri";
import DashboardPanel from "./DashboardPanel";
import TaskTab from "../TaskTab";
import BoardTab from "./hub/BoardTab";
import BranchesTab from "./hub/BranchesTab";
import type { AgentRecord } from "./hub/types";
import type { TeamMember } from "./config/types";
import { SubtabBar, SubtabItem } from "../common/telemetry/SubtabBar";
import { NOC_COLORS, NOC_SURFACES } from "../common/telemetry/tokens";

type WorkTab = "dashboard" | "tasks" | "board" | "branches";

const WORK_SUBTABS: SubtabItem<WorkTab>[] = [
  { id: "dashboard", label: "Dashboard" },
  { id: "tasks", label: "Tasks" },
  { id: "board", label: "Board" },
  { id: "branches", label: "Branches" },
];

export default function WorkPanel({ teamMemberIds, onAddAgent, onRemoveAgent }: {
  teamMemberIds: string[];
  onAddAgent: (agent: TeamMember) => Promise<void> | void;
  onRemoveAgent: (agent: TeamMember) => Promise<void> | void;
}) {
  const [workTab, setWorkTab] = useState<WorkTab>("dashboard");
  const [visitedTabs, setVisitedTabs] = useState<Set<WorkTab>>(() => new Set(["dashboard"]));
  const [agents, setAgents] = useState<AgentRecord[]>([]);
  const [error, setError] = useState<string>("");

  const refreshAgents = useCallback(async () => {
    const list = await invoke<AgentRecord[]>("hub_list_agents");
    setAgents(list);
    return list;
  }, []);

  const enrollAgent = useCallback(async (agent: TeamMember) => {
    try {
      await onAddAgent(agent);
    } catch (e) {
      setError(String(e));
      throw e;
    } finally {
      await refreshAgents().catch((e) => setError(String(e)));
    }
  }, [onAddAgent, refreshAgents]);

  const unenrollAgent = useCallback(async (agent: TeamMember) => {
    try {
      await onRemoveAgent(agent);
    } catch (e) {
      setError(String(e));
      throw e;
    } finally {
      await refreshAgents().catch((e) => setError(String(e)));
    }
  }, [onRemoveAgent, refreshAgents]);

  useEffect(() => {
    refreshAgents().catch((e) => setError(String(e)));
  }, [refreshAgents]);

  return (
    <div className="glass-card fade-in" style={{ animationDelay: "0.1s" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "1rem", flexWrap: "wrap", marginBottom: "1rem", paddingBottom: "0.75rem", borderBottom: "1px solid rgba(148, 163, 184, 0.14)" }}>
        <div style={{ display: "flex", alignItems: "baseline", gap: "0.75rem", flexWrap: "wrap" }}>
          <h2 style={{ margin: 0, fontSize: "1.4rem", fontWeight: 800, color: NOC_SURFACES.textPrimary, letterSpacing: "-0.01em" }}>
            Work
          </h2>
          <span style={{ fontSize: "0.78rem", color: NOC_SURFACES.textMuted }}>
            Roster telemetry, workflow pipelines, project board &amp; branch status
          </span>
        </div>
        <SubtabBar
          ariaLabel="Work subtabs"
          tabs={WORK_SUBTABS}
          activeId={workTab}
          onSelect={(id) =>
            startTransition(() => {
              setVisitedTabs((prev) => (prev.has(id) ? prev : new Set(prev).add(id)));
              setWorkTab(id);
            })
          }
        />
      </div>

      {error && (
        <div style={{ marginBottom: "1rem" }}>
          <span style={{ color: NOC_COLORS.crit, fontSize: "0.85rem", background: "rgba(239, 68, 68, 0.1)", padding: "0.25rem 0.6rem", borderRadius: "6px", border: "1px solid rgba(239, 68, 68, 0.3)" }}>Error: {error}</span>
        </div>
      )}

      <div style={{ display: workTab === "dashboard" ? "block" : "none" }}>
        <DashboardPanel agents={agents} teamMemberIds={teamMemberIds} onAddAgent={enrollAgent} onRemoveAgent={unenrollAgent} />
      </div>
      {visitedTabs.has("tasks") && (
        <div style={{ display: workTab === "tasks" ? "block" : "none" }}>
          <TaskTab />
        </div>
      )}
      {visitedTabs.has("board") && (
        <div style={{ display: workTab === "board" ? "block" : "none" }}>
          <BoardTab />
        </div>
      )}
      {visitedTabs.has("branches") && (
        <div style={{ display: workTab === "branches" ? "block" : "none" }}>
          <BranchesTab />
        </div>
      )}
    </div>
  );
}
