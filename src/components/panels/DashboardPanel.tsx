import { useCallback, useEffect, useState } from "react";
import { invoke } from "../../lib/tauri";
import { AgentAvatar } from "./messager/AgentAvatar";
import { rosterAgentToTeamMember } from "../../app/team";
import ActivityView from "./dashboard/ActivityView";
import { DashboardOrionTopology } from "./dashboard/DashboardOrionTopology";
import type { TeamMember } from "./config/types";
import { nocPanelStyle, nocCardDenseStyle } from "../common/telemetry/panel";
import { KpiCardGrid, type KpiCardItem, EmptyStateChip } from "../common/telemetry/status";
import { Meter } from "../common/telemetry/gauges";
import { NOC_COLORS, NOC_SURFACES } from "../common/telemetry/tokens";
import { SubtabBar } from "../common/telemetry/SubtabBar";

interface AgentRecord {
  id: string;
  display_name: string;
  avatar_attachment_id?: string | null;
}
interface BudgetStatus {
  agent_id: string;
  limit_units: number;
  spent_units: number;
  paused: boolean;
  updated_at: string;
}
interface AgentMetrics {
  agent_id: string;
  lines_written: number;
  tokens_used: number;
  tokens_cached: number;
  provider_calls: number;
  output_chars: number;
  updated_at: string;
}
interface TaskRecord {
  id: string;
  title: string;
  status: string;
  updated_at: string;
}
interface MessageRecord {
  id: string;
  from_agent: string;
  to_agent: string;
  kind: string;
  status: string;
  created_at: string;
}
interface WakeRecord {
  id: string;
  target_agent: string;
  status: string;
  created_at: string;
}

const DASHBOARD_SUBTABS = [
  { id: "overview", label: "Telemetry & Overview" },
  { id: "topology", label: "Orion Visual Topology" },
  { id: "activity", label: "Tool & Workspace Activity" },
];

const number = (value: number) => new Intl.NumberFormat().format(value);

export default function DashboardPanel({
  agents,
  teamMemberIds,
  onAddAgent,
  onRemoveAgent,
}: {
  agents: AgentRecord[];
  teamMemberIds: string[];
  onAddAgent: (agent: TeamMember) => Promise<void> | void;
  onRemoveAgent: (agent: TeamMember) => Promise<void> | void;
}) {
  const [subTab, setSubTab] = useState<"overview" | "topology" | "activity">("overview");
  const [budgets, setBudgets] = useState<BudgetStatus[]>([]);
  const [metrics, setMetrics] = useState<AgentMetrics[]>([]);
  const [tasks, setTasks] = useState<TaskRecord[]>([]);
  const [messages, setMessages] = useState<MessageRecord[]>([]);
  const [wakes, setWakes] = useState<WakeRecord[]>([]);
  const [error, setError] = useState("");
  const [updated, setUpdated] = useState("");

  const refresh = useCallback(async () => {
    setError("");
    try {
      const [nextMetrics, nextBudgets, nextTasks, nextMessages, nextWakes] =
        await Promise.all([
          invoke<AgentMetrics[]>("hub_list_agent_metrics"),
          Promise.all(
            agents.map((agent) =>
              invoke<BudgetStatus | null>("hub_get_budget", { agent: agent.id }),
            ),
          ),
          invoke<TaskRecord[]>("hub_list_tasks", { status: null }),
          invoke<MessageRecord[]>("hub_list_messages", { to: null, status: null }),
          invoke<WakeRecord[]>("hub_list_wakes", { target: null, pendingOnly: false }),
        ]);
      setMetrics(nextMetrics);
      setBudgets(nextBudgets.filter((budget): budget is BudgetStatus => budget !== null));
      setTasks(nextTasks);
      setMessages(nextMessages);
      setWakes(nextWakes);
      setUpdated(new Date().toLocaleTimeString());
    } catch (e) {
      setError(String(e));
    }
  }, [agents]);

  useEffect(() => {
    if (agents.length > 0) void refresh();
  }, [agents, refresh]);

  const metricFor = (id: string) => metrics.find((metric) => metric.agent_id === id);
  const budgetFor = (id: string) => budgets.find((budget) => budget.agent_id === id);
  const total = (field: keyof AgentMetrics) =>
    metrics.reduce((sum, metric) => sum + Number(metric[field] || 0), 0);

  const kpiItems: KpiCardItem[] = [
    {
      title: "Provider Calls",
      subtitle: "Lifetime completions",
      value: number(total("provider_calls")),
      footer: "Model inference calls",
      provenance: "measured",
    },
    {
      title: "Lines Written",
      subtitle: "Code lines delivered",
      value: number(total("lines_written")),
      footer: "Cumulative source lines",
      provenance: "measured",
    },
    {
      title: "Tokens Used",
      subtitle: "Whitespace estimates",
      value: number(total("tokens_used")),
      footer: "Turn token estimates",
      provenance: "estimated",
    },
    {
      title: "Cached Tokens",
      subtitle: "Reported cached reuse",
      value: number(total("tokens_cached")),
      footer: "Provider cached prompt tokens",
      provenance: "measured",
    },
  ];

  return (
    <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1.25rem" }}>
      {/* Navigation Subtabs & Refresh Bar */}
      <div
        style={{
          ...nocPanelStyle,
          display: "flex",
          justifyContent: "space-between",
          gap: "1rem",
          alignItems: "center",
          flexWrap: "wrap",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "1.25rem", flexWrap: "wrap" }}>
          <div>
            <h3 style={{ margin: 0, fontSize: "1.1rem", fontWeight: 700, color: NOC_SURFACES.textPrimary }}>Dashboard</h3>
            <p style={{ margin: "0.2rem 0 0", color: NOC_SURFACES.textMuted, fontSize: "0.82rem" }}>
              {subTab === "overview"
                ? "Local cumulative usage from the Shared Hub. Token counts are whitespace estimates."
                : subTab === "topology"
                  ? "Interactive constellation network, radar capability profile, and honeycomb situational matrix."
                  : "Activity traceability across tasks, commands, files, and agents."}
            </p>
          </div>
          <SubtabBar
            ariaLabel="Dashboard view modes"
            tabs={DASHBOARD_SUBTABS}
            activeId={subTab}
            onSelect={(id: string) => setSubTab(id as "overview" | "topology" | "activity")}
          />
        </div>

        {subTab === "overview" && (
          <button className="btn-secondary noc-focusable" onClick={() => void refresh()}>
            Refresh{updated ? ` · ${updated}` : ""}
          </button>
        )}
      </div>

      {error && (
        <div style={{ ...nocPanelStyle, color: NOC_COLORS.crit, background: "rgba(239, 68, 68, 0.1)" }}>
          {error}
        </div>
      )}

      {subTab === "topology" ? (
        <DashboardOrionTopology
          agents={agents}
          metrics={metrics}
          budgets={budgets}
          tasks={tasks}
          messages={messages}
          wakes={wakes}
        />
      ) : subTab === "activity" ? (
        <ActivityView agents={agents} />
      ) : (
        <>
          {/* NOC KPI Summary Grid */}
          <KpiCardGrid cards={kpiItems} />

          {/* Collaboration Overview */}
          <div style={nocPanelStyle}>
            <h3 style={{ margin: "0 0 0.85rem", fontSize: "0.95rem", fontWeight: 700, color: NOC_SURFACES.textPrimary }}>
              Collaboration Overview
            </h3>
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(auto-fit, minmax(140px, 1fr))",
                gap: "0.75rem",
                marginBottom: tasks.length > 0 ? "1rem" : 0,
              }}
            >
              {[
                { label: "Tasks", count: tasks.length, color: NOC_COLORS.cyan },
                { label: "Messages", count: messages.length, color: NOC_COLORS.indigo },
                { label: "Wakes", count: wakes.length, color: NOC_COLORS.purple },
                { label: "Pending Wakes", count: wakes.filter((w) => w.status === "pending").length, color: NOC_COLORS.warn },
              ].map((c) => (
                <div key={c.label} style={{ ...nocCardDenseStyle, padding: "0.6rem 0.85rem" }}>
                  <div style={{ fontSize: "0.74rem", color: NOC_SURFACES.textMuted, textTransform: "uppercase" }}>{c.label}</div>
                  <div className="tabular-nums" style={{ fontSize: "1.35rem", fontWeight: 800, color: c.color, marginTop: "0.2rem" }}>
                    {number(c.count)}
                  </div>
                </div>
              ))}
            </div>

            {tasks.length > 0 && (
              <div style={{ display: "flex", flexDirection: "column", gap: "0.45rem", borderTop: "1px solid rgba(148, 163, 184, 0.14)", paddingTop: "0.75rem" }}>
                {tasks.slice(0, 5).map((task) => (
                  <div
                    key={task.id}
                    style={{
                      display: "flex",
                      justifyContent: "space-between",
                      alignItems: "center",
                      gap: "1rem",
                      fontSize: "0.85rem",
                      padding: "0.25rem 0",
                    }}
                  >
                    <span style={{ color: NOC_SURFACES.textPrimary, fontWeight: 500 }}>{task.title}</span>
                    <span
                      style={{
                        fontSize: "0.74rem",
                        fontWeight: 600,
                        color: task.status === "failed" ? NOC_COLORS.crit : task.status === "completed" ? NOC_COLORS.ok : NOC_COLORS.cyan,
                      }}
                    >
                      {task.status}
                    </span>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* Per-Agent Roster Telemetry Cards */}
          <div style={{ display: "flex", flexDirection: "column", gap: "0.85rem" }}>
            {agents.map((agent) => {
              const metric = metricFor(agent.id);
              const budget = budgetFor(agent.id);
              const percent = budget
                ? Math.min(100, (budget.spent_units / budget.limit_units) * 100)
                : 0;
              const enrolled = teamMemberIds.includes(agent.id);
              const member = rosterAgentToTeamMember(agent);
              return (
                <div key={agent.id} style={nocPanelStyle}>
                  <div
                    style={{
                      display: "flex",
                      justifyContent: "space-between",
                      gap: "1rem",
                      flexWrap: "wrap",
                      alignItems: "center",
                      marginBottom: "0.6rem",
                    }}
                  >
                    <div style={{ display: "inline-flex", alignItems: "center", gap: "0.6rem" }}>
                      <AgentAvatar
                        agentId={agent.id}
                        displayName={agent.display_name}
                        avatarAttachmentId={agent.avatar_attachment_id}
                        size={28}
                      />
                      <div>
                        <strong style={{ color: NOC_COLORS.cyan, fontSize: "1rem" }}>
                          {agent.display_name}
                        </strong>
                        <span
                          style={{
                            marginLeft: "0.5rem",
                            fontSize: "0.72rem",
                            padding: "0.1rem 0.45rem",
                            borderRadius: "4px",
                            background: enrolled ? "rgba(16, 185, 129, 0.15)" : "rgba(148, 163, 184, 0.15)",
                            color: enrolled ? NOC_COLORS.ok : NOC_COLORS.idle,
                            border: `1px solid ${enrolled ? "rgba(16, 185, 129, 0.3)" : "rgba(148, 163, 184, 0.3)"}`,
                            fontWeight: 700,
                          }}
                        >
                          {enrolled ? "Enrolled" : "Roster Only"}
                        </span>
                      </div>
                    </div>
                    <button
                      type="button"
                      className={`${enrolled ? "btn-secondary" : "btn-primary"} noc-focusable`}
                      style={{ marginTop: 0, padding: "0.3rem 0.75rem", fontSize: "0.78rem" }}
                      onClick={async () => {
                        setError("");
                        try {
                          if (enrolled) {
                            await onRemoveAgent(member);
                          } else {
                            await onAddAgent(member);
                          }
                        } catch (e) {
                          setError(String(e));
                        }
                      }}
                    >
                      {enrolled ? "Unenroll" : "Enroll"}
                    </button>
                  </div>

                  {budget && (
                    <div style={{ margin: "0.75rem 0" }}>
                      <Meter
                        label="Budget Utilization"
                        percent={percent}
                        detail={`${budget.spent_units} / ${budget.limit_units} units${budget.paused ? " · paused" : ""}`}
                        tone={budget.paused ? "crit" : "cyan"}
                        provenance="measured"
                      />
                    </div>
                  )}

                  <div
                    style={{
                      display: "grid",
                      gridTemplateColumns: "repeat(auto-fit, minmax(130px, 1fr))",
                      gap: "0.6rem",
                      marginTop: "0.6rem",
                    }}
                  >
                    {[
                      { label: "Lines written", val: number(metric?.lines_written || 0), suffix: "" },
                      { label: "Tokens used", val: number(metric?.tokens_used || 0), suffix: " est." },
                      { label: "Cached tokens", val: number(metric?.tokens_cached || 0), suffix: " rep." },
                      { label: "Provider calls", val: number(metric?.provider_calls || 0), suffix: "" },
                    ].map((m) => (
                      <div key={m.label} style={{ ...nocCardDenseStyle, padding: "0.45rem 0.65rem" }}>
                        <div style={{ fontSize: "0.72rem", color: NOC_SURFACES.textMuted }}>{m.label}</div>
                        <div className="tabular-nums" style={{ color: NOC_SURFACES.textPrimary, fontWeight: 700, fontSize: "0.95rem", marginTop: "0.15rem" }}>
                          {m.val}{m.suffix}
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              );
            })}
            {agents.length === 0 && (
              <EmptyStateChip
                title="No agents registered yet"
                detail="Agents configured in the workspace will appear in the dashboard roster."
                tone="idle"
              />
            )}
          </div>
        </>
      )}
    </div>
  );
}
