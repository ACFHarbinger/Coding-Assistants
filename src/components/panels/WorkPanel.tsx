import { startTransition, useCallback, useEffect, useState } from "react";
import { invoke } from "../../lib/tauri";
import DashboardPanel from "./DashboardPanel";
import TaskTab from "../TaskTab";
import BoardTab from "./hub/BoardTab";
import BranchesTab from "./hub/BranchesTab";
import type { AgentRecord } from "./hub/types";
import type { TeamMember } from "./config/types";
import { SubtabBar, SubtabItem } from "../common/telemetry/SubtabBar";

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
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "1rem", flexWrap: "wrap", marginBottom: "1rem" }}>
        <h2 style={{ margin: 0, fontSize: "1.5rem", background: "linear-gradient(to right, #fff, var(--primary))", WebkitBackgroundClip: "text", WebkitTextFillColor: "transparent" }}>
          Work
        </h2>
        <SubtabBar
          ariaLabel="Work subtabs"
          tabs={WORK_SUBTABS}
          activeId={workTab}
          onSelect={(id) => startTransition(() => setWorkTab(id))}
        />
      </div>

      {error && (
        <div style={{ marginBottom: "1rem" }}>
          <span style={{ color: "#ef4444", fontSize: "0.85rem", background: "rgba(239, 68, 68, 0.1)", padding: "0.2rem 0.5rem", borderRadius: "4px" }}>Error: {error}</span>
        </div>
      )}

      {workTab === "dashboard" && (
        <DashboardPanel agents={agents} teamMemberIds={teamMemberIds} onAddAgent={enrollAgent} onRemoveAgent={unenrollAgent} />
      )}
      {workTab === "tasks" && <TaskTab />}
      {workTab === "board" && <BoardTab />}
      {workTab === "branches" && <BranchesTab />}
    </div>
  );
}
