import { useState, useEffect, useCallback, useMemo } from "react";
import { invoke } from "../lib/tauri";
import { nocPanelStyle, nocCardDenseStyle } from "./common/telemetry/panel";
import { EmptyStateChip, KpiCardGrid, StalenessDot, type KpiCardItem } from "./common/telemetry/status";
import { Meter } from "./common/telemetry/gauges";
import { NOC_COLORS, NOC_SURFACES } from "./common/telemetry/tokens";

export interface TaskRecord {
  id: string;
  title: string;
  workspace_path: string | null;
  status: string;
  step_index: number;
  steps: any[];
  created_at: string;
  updated_at: string;
  last_message_id: string | null;
  attempts: Record<string, number>;
  open_agents: string[];
  pending_agents: string[];
  max_parallel: number;
}

export interface MessageRecord {
  id: string;
  from_agent: string;
  to_agent: string;
  workspace_path: string | null;
  task_id: string | null;
  kind: string;
  status: string;
  subject: string | null;
  body: string;
  created_at: string;
  acked_at: string | null;
}

function resolveStageState(
  idx: number,
  currentIdx: number,
  taskStatus: string,
  openAgents: string[] = [],
  pendingAgents: string[] = []
): { label: string; color: string; bg: string; icon: string } {
  if (idx < currentIdx || taskStatus === "completed") {
    return { label: "Done", color: NOC_COLORS.ok, bg: "rgba(16, 185, 129, 0.2)", icon: "✓" };
  }
  if (idx === currentIdx) {
    if (taskStatus === "failed") {
      return { label: "Failed", color: NOC_COLORS.crit, bg: "rgba(239, 68, 68, 0.2)", icon: "✕" };
    }
    if (taskStatus === "auth_required") {
      return { label: "Auth Required", color: NOC_COLORS.purple, bg: "rgba(168, 85, 247, 0.2)", icon: "🔒" };
    }
    if (taskStatus === "input_required") {
      return { label: "Input Required", color: NOC_COLORS.warn, bg: "rgba(234, 179, 8, 0.2)", icon: "💬" };
    }
    if (openAgents.length > 0) {
      return { label: "Open", color: NOC_COLORS.cyan, bg: "rgba(56, 189, 248, 0.2)", icon: "⚡" };
    }
    if (pendingAgents.length > 0) {
      return { label: "Pending", color: NOC_COLORS.warn, bg: "rgba(234, 179, 8, 0.2)", icon: "⏳" };
    }
    return { label: "In Progress", color: NOC_COLORS.cyan, bg: "rgba(56, 189, 248, 0.2)", icon: "●" };
  }
  return { label: "Upcoming", color: NOC_COLORS.idle, bg: "rgba(148, 163, 184, 0.15)", icon: `${idx + 1}` };
}

export default function TaskTab() {
  const [tasks, setTasks] = useState<TaskRecord[]>([]);
  const [selectedTask, setSelectedTask] = useState<TaskRecord | null>(null);
  const [taskMessages, setTaskMessages] = useState<MessageRecord[]>([]);
  const [lastUpdatedSec, setLastUpdatedSec] = useState<number | null>(null);

  const refreshTasks = useCallback(async () => {
    try {
      const res = await invoke<TaskRecord[]>("hub_list_tasks", { status: null });
      setTasks(res);
      setLastUpdatedSec(Math.floor(Date.now() / 1000));
    } catch (e) {
      console.error(e);
    }
  }, []);

  const kpis = useMemo<KpiCardItem[]>(() => {
    const completed = tasks.filter((t) => t.status === "completed").length;
    const waiting = tasks.filter((t) => t.status === "input_required" || t.status === "auth_required").length;
    const failed = tasks.filter((t) => t.status === "failed").length;
    const active = tasks.length - completed - failed;
    return [
      { title: "Total Workflows", subtitle: "Hub history", value: String(tasks.length), footer: "Recorded pipelines", accent: NOC_COLORS.indigo },
      { title: "Active / Open", subtitle: "In flight", value: String(active), footer: "Executing stages", accent: NOC_COLORS.cyan },
      { title: "Waiting on Gate", subtitle: "Human / Auth", value: String(waiting), footer: waiting > 0 ? "Input or auth required" : "No blocked gates", accent: waiting > 0 ? NOC_COLORS.warn : NOC_COLORS.idle },
      { title: "Completed", subtitle: "Finished", value: String(completed), footer: failed > 0 ? `${failed} failed` : "All finished clean", accent: NOC_COLORS.ok },
    ];
  }, [tasks]);

  const selectTask = async (task: TaskRecord) => {
    setSelectedTask(task);
    try {
      const msgs = await invoke<MessageRecord[]>("hub_list_messages", { to: null, status: null });
      setTaskMessages(
        msgs
          .filter((m) => m.task_id === task.id)
          .sort((a, b) => a.created_at.localeCompare(b.created_at))
      );
    } catch (e) {
      console.error(e);
    }
  };

  useEffect(() => {
    void refreshTasks();
  }, [refreshTasks]);

  if (selectedTask) {
    const totalSteps = Math.max(1, selectedTask.steps?.length || 1);
    const stepsArray = selectedTask.steps?.length ? selectedTask.steps : [{ name: selectedTask.title }];

    return (
      <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1.25rem" }}>
        {/* Header Bar */}
        <div style={{ ...nocPanelStyle, display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: "0.75rem" }}>
          <div>
            <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
              <h3 style={{ margin: 0, fontSize: "1.15rem", fontWeight: 700, color: NOC_SURFACES.textPrimary }}>
                {selectedTask.title}
              </h3>
              <span
                style={{
                  fontSize: "0.72rem",
                  fontWeight: 700,
                  padding: "0.15rem 0.5rem",
                  borderRadius: "6px",
                  background: selectedTask.status === "completed" ? "rgba(16, 185, 129, 0.15)" : "rgba(56, 189, 248, 0.15)",
                  color: selectedTask.status === "completed" ? NOC_COLORS.ok : NOC_COLORS.cyan,
                  border: `1px solid ${selectedTask.status === "completed" ? "rgba(16, 185, 129, 0.3)" : "rgba(56, 189, 248, 0.3)"}`,
                }}
              >
                {selectedTask.status.toUpperCase()}
              </span>
            </div>
            <div style={{ display: "flex", gap: "1rem", flexWrap: "wrap", fontSize: "0.8rem", color: NOC_SURFACES.textMuted, marginTop: "0.35rem" }}>
              <span>ID: <code style={{ color: NOC_COLORS.cyan }}>{selectedTask.id.slice(0, 8)}</code></span>
              {selectedTask.workspace_path && <span>Workspace: {selectedTask.workspace_path.split("/").pop()}</span>}
              <span className="tabular-nums">Stage: {selectedTask.step_index + 1} / {totalSteps}</span>
            </div>
          </div>
          <button className="btn-secondary noc-focusable" onClick={() => setSelectedTask(null)}>
            ← Back to Tasks
          </button>
        </div>

        {/* Visual Workflow Pipeline Stepper */}
        <div style={{ ...nocPanelStyle, display: "flex", flexDirection: "column", gap: "0.75rem" }}>
          <div style={{ fontSize: "0.76rem", color: NOC_SURFACES.textMuted, textTransform: "uppercase", fontWeight: 700, letterSpacing: "0.04em" }}>
            Workflow Stage Pipeline
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", overflowX: "auto", paddingBottom: "0.25rem" }}>
            {stepsArray.map((step: any, idx: number) => {
              const state = resolveStageState(
                idx,
                selectedTask.step_index,
                selectedTask.status,
                selectedTask.open_agents,
                selectedTask.pending_agents
              );
              const title = step.name || step.agent || `Stage ${idx + 1}`;
              return (
                <div key={idx} style={{ display: "flex", alignItems: "center", gap: "0.5rem", minWidth: 150, flex: 1 }}>
                  <div
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: "0.5rem",
                      padding: "0.45rem 0.65rem",
                      borderRadius: "8px",
                      background: state.bg,
                      border: `1px solid ${state.color}55`,
                      width: "100%",
                    }}
                  >
                    <span
                      style={{
                        width: 22,
                        height: 22,
                        borderRadius: "50%",
                        background: state.color,
                        color: "#051626",
                        display: "inline-flex",
                        alignItems: "center",
                        justifyContent: "center",
                        fontSize: "0.72rem",
                        fontWeight: 800,
                        flexShrink: 0,
                      }}
                    >
                      {state.icon}
                    </span>
                    <div style={{ minWidth: 0, overflow: "hidden" }}>
                      <div style={{ fontSize: "0.78rem", fontWeight: 700, color: NOC_SURFACES.textPrimary, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                        {title}
                      </div>
                      <div style={{ fontSize: "0.7rem", color: state.color, fontWeight: 600 }}>
                        {state.label}
                      </div>
                    </div>
                  </div>
                  {idx < stepsArray.length - 1 && (
                    <span style={{ color: "rgba(148, 163, 184, 0.4)", fontWeight: 700, fontSize: "0.9rem" }}>→</span>
                  )}
                </div>
              );
            })}
          </div>
        </div>

        {/* Transcript & Handoffs */}
        <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
          <h4 style={{ margin: "0.25rem 0 0", color: NOC_SURFACES.textPrimary, fontSize: "0.95rem" }}>Transcript & Handoffs</h4>
          {taskMessages.length === 0 ? (
            <EmptyStateChip
              title="No messages or handoffs recorded"
              detail="Agent communication during this workflow will be captured here."
              tone="idle"
            />
          ) : (
            taskMessages.map((m) => (
              <div key={m.id} style={nocCardDenseStyle}>
                <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "0.4rem", alignItems: "center" }}>
                  <div>
                    <span style={{ color: NOC_COLORS.cyan, fontWeight: 700, fontSize: "0.88rem" }}>{m.from_agent}</span>
                    <span style={{ color: NOC_SURFACES.textMuted, margin: "0 0.4rem" }}>→</span>
                    <span style={{ color: NOC_COLORS.indigo, fontWeight: 700, fontSize: "0.88rem" }}>{m.to_agent}</span>
                  </div>
                  <div style={{ display: "flex", gap: "0.45rem", fontSize: "0.72rem", alignItems: "center" }}>
                    <span style={{ background: "rgba(255,255,255,0.08)", padding: "0.1rem 0.4rem", borderRadius: "4px", color: NOC_SURFACES.textSecondary }}>
                      {m.kind}
                    </span>
                    <span className="tabular-nums" style={{ color: NOC_SURFACES.textMuted, fontFamily: "var(--font-mono)" }}>
                      {new Date(m.created_at).toLocaleTimeString()}
                    </span>
                  </div>
                </div>
                {m.subject && <div style={{ fontWeight: 600, marginBottom: "0.4rem", color: NOC_SURFACES.textPrimary, fontSize: "0.88rem" }}>{m.subject}</div>}
                <pre style={{ margin: 0, whiteSpace: "pre-wrap", fontFamily: "var(--font-sans)", fontSize: "0.86rem", color: NOC_SURFACES.textSecondary, lineHeight: 1.5 }}>
                  {m.body}
                </pre>
              </div>
            ))
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
      <div style={{ ...nocPanelStyle, display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: "0.75rem" }}>
        <div>
          <div style={{ display: "flex", alignItems: "center", gap: "0.65rem", flexWrap: "wrap" }}>
            <h3 style={{ margin: 0, fontSize: "1.05rem", fontWeight: 700, color: NOC_SURFACES.textPrimary }}>Task Browser</h3>
            <StalenessDot lastUpdatedUnixSeconds={lastUpdatedSec} pollIntervalSeconds={30} />
          </div>
          <p style={{ margin: "0.2rem 0 0", color: NOC_SURFACES.textMuted, fontSize: "0.82rem" }}>
            Inspect ongoing and past workflow execution pipelines.
          </p>
        </div>
        <button className="btn-secondary noc-focusable" onClick={refreshTasks}>
          Refresh
        </button>
      </div>

      {tasks.length > 0 && <KpiCardGrid cards={kpis} />}

      {tasks.length === 0 ? (
        <EmptyStateChip
          title="No tasks found in the hub"
          detail="Tasks dispatched through Orchestrate or Chat will appear here."
          tone="idle"
        />
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: "0.65rem" }}>
          {tasks.map((t) => {
            const totalSteps = Math.max(1, t.steps?.length || 1);
            const stageNum = Math.min(totalSteps, t.step_index + 1);
            const stagePct = t.status === "completed" ? 100 : Math.round((stageNum / totalSteps) * 100);
            const tone = t.status === "completed" ? "ok" : t.status === "failed" ? "crit" : t.status === "input_required" || t.status === "auth_required" ? "warn" : "ok";
            return (
              <div
                key={t.id}
                role="button"
                tabIndex={0}
                className="noc-focusable"
                style={{ ...nocCardDenseStyle, cursor: "pointer", transition: "all 0.16s cubic-bezier(0.2, 0, 0, 1)", display: "flex", flexDirection: "column", gap: "0.45rem" }}
                onClick={() => selectTask(t)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") selectTask(t);
                }}
              >
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: "0.5rem" }}>
                  <strong style={{ fontSize: "1rem", fontWeight: 700, color: NOC_COLORS.cyan }}>{t.title}</strong>
                  <span
                    style={{
                      fontSize: "0.72rem",
                      fontWeight: 700,
                      color: t.status === "completed" ? NOC_COLORS.ok : NOC_COLORS.cyan,
                      background: t.status === "completed" ? "rgba(16, 185, 129, 0.15)" : "rgba(56, 189, 248, 0.15)",
                      border: `1px solid ${t.status === "completed" ? "rgba(16, 185, 129, 0.3)" : "rgba(56, 189, 248, 0.3)"}`,
                      padding: "0.15rem 0.5rem",
                      borderRadius: "6px",
                    }}
                  >
                    {t.status.toUpperCase()}
                  </span>
                </div>
                <div style={{ display: "flex", gap: "1rem", fontSize: "0.78rem", color: NOC_SURFACES.textMuted, flexWrap: "wrap" }}>
                  <span>ID: <code style={{ color: NOC_SURFACES.textPrimary }}>{t.id.slice(0, 8)}</code></span>
                  {t.workspace_path && <span>Workspace: {t.workspace_path.split("/").pop()}</span>}
                  <span className="tabular-nums">Stage: {stageNum} / {totalSteps}</span>
                  <span className="tabular-nums">Created: {new Date(t.created_at).toLocaleString()}</span>
                </div>
                <Meter label="Pipeline Progress" percent={stagePct} detail={`${stagePct}%`} tone={tone} />
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
