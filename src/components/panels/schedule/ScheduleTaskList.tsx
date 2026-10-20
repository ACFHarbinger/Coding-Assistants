import React, { useState } from "react";
import type { ScheduledTask, ScheduleStatus } from "./types";

interface ScheduleTaskListProps {
  tasks: ScheduledTask[];
  onCancelTask: (id: string) => Promise<void>;
  cancellingId: string | null;
}

export function ScheduleTaskList({
  tasks,
  onCancelTask,
  cancellingId,
}: ScheduleTaskListProps) {
  const [expandedId, setExpandedId] = useState<string | null>(null);

  const renderStatusBadge = (status: ScheduleStatus) => {
    const baseStyle: React.CSSProperties = {
      padding: "0.2rem 0.55rem",
      borderRadius: "9999px",
      fontSize: "0.75rem",
      fontWeight: 600,
      textTransform: "capitalize",
      letterSpacing: "0.02em",
      display: "inline-block",
    };

    switch (status) {
      case "active":
        return (
          <span style={{ ...baseStyle, background: "rgba(6, 182, 212, 0.15)", color: "#06b6d4", border: "1px solid rgba(6, 182, 212, 0.3)" }}>
            ⚡ active
          </span>
        );
      case "scheduled":
        return (
          <span style={{ ...baseStyle, background: "rgba(59, 130, 246, 0.15)", color: "#60a5fa", border: "1px solid rgba(59, 130, 246, 0.3)" }}>
            ⏱️ scheduled
          </span>
        );
      case "completed":
        return (
          <span style={{ ...baseStyle, background: "rgba(34, 197, 94, 0.15)", color: "#22c55e", border: "1px solid rgba(34, 197, 94, 0.3)" }}>
            ✓ completed
          </span>
        );
      case "paused":
        return (
          <span style={{ ...baseStyle, background: "rgba(245, 158, 11, 0.15)", color: "#f59e0b", border: "1px solid rgba(245, 158, 11, 0.3)" }}>
            ⏸️ paused
          </span>
        );
      case "failed":
        return (
          <span style={{ ...baseStyle, background: "rgba(239, 68, 68, 0.15)", color: "#ef4444", border: "1px solid rgba(239, 68, 68, 0.3)" }}>
            ✕ failed
          </span>
        );
      case "cancelled":
      default:
        return (
          <span style={{ ...baseStyle, background: "rgba(148, 163, 184, 0.15)", color: "#94a3b8", border: "1px solid rgba(148, 163, 184, 0.3)" }}>
            cancelled
          </span>
        );
    }
  };

  const isTerminal = (status: ScheduleStatus) =>
    status === "completed" || status === "cancelled" || status === "failed";

  const formatDateTime = (iso?: string | null) => {
    if (!iso) return "—";
    try {
      const d = new Date(iso);
      return `${d.toLocaleDateString()} ${d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}`;
    } catch {
      return iso;
    }
  };

  if (tasks.length === 0) {
    return (
      <div
        style={{
          padding: "3rem 1.5rem",
          textAlign: "center",
          borderRadius: "12px",
          background: "rgba(255, 255, 255, 0.02)",
          border: "1px dashed rgba(255, 255, 255, 0.1)",
          color: "var(--text-muted, #94a3b8)",
        }}
      >
        <div style={{ fontSize: "2rem", marginBottom: "0.5rem" }}>⏱️</div>
        <div style={{ fontSize: "1rem", fontWeight: 600, color: "var(--text-color, #fff)" }}>
          No scheduled goals found
        </div>
        <p style={{ margin: "0.4rem 0 0 0", fontSize: "0.85rem" }}>
          Create a scheduled goal above to automate timed or continuous re-prompts across standing agents.
        </p>
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
      {tasks.map((task) => {
        const isExpanded = expandedId === task.id;
        const terminal = isTerminal(task.status);
        const isCancelling = cancellingId === task.id;

        return (
          <div
            key={task.id}
            style={{
              background: "rgba(255, 255, 255, 0.03)",
              border: "1px solid rgba(255, 255, 255, 0.08)",
              borderRadius: "10px",
              padding: "1rem 1.25rem",
              display: "flex",
              flexDirection: "column",
              gap: "0.75rem",
            }}
          >
            {/* Top row: Name, target, status & actions */}
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: "1rem", flexWrap: "wrap" }}>
              <div>
                <div style={{ display: "flex", alignItems: "center", gap: "0.6rem" }}>
                  <span style={{ fontWeight: 600, fontSize: "1rem", color: "var(--text-color, #fff)" }}>
                    {task.name}
                  </span>
                  {renderStatusBadge(task.status)}
                </div>
                <div style={{ display: "flex", alignItems: "center", gap: "0.75rem", marginTop: "0.25rem", fontSize: "0.8rem", color: "var(--text-muted, #94a3b8)" }}>
                  <span>
                    🎯 Target:{" "}
                    <strong style={{ color: "var(--text-color, #fff)" }}>
                      {task.target.kind === "team" ? "👥 Full Team" : `🤖 ${task.target.agent_id}`}
                    </strong>
                  </span>
                  <span>•</span>
                  <span>
                    📁 Workspace: <code>{task.workspace}</code>
                  </span>
                </div>
              </div>

              <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
                {!terminal && (
                  <button
                    type="button"
                    onClick={() => void onCancelTask(task.id)}
                    disabled={isCancelling}
                    style={{
                      padding: "0.3rem 0.7rem",
                      borderRadius: "6px",
                      background: "rgba(239, 68, 68, 0.15)",
                      border: "1px solid rgba(239, 68, 68, 0.3)",
                      color: "#ef4444",
                      fontSize: "0.75rem",
                      cursor: isCancelling ? "wait" : "pointer",
                    }}
                  >
                    {isCancelling ? "Cancelling…" : "Cancel Task"}
                  </button>
                )}
                <button
                  type="button"
                  onClick={() => setExpandedId(isExpanded ? null : task.id)}
                  style={{
                    padding: "0.3rem 0.6rem",
                    borderRadius: "6px",
                    background: "rgba(255, 255, 255, 0.05)",
                    border: "1px solid rgba(255, 255, 255, 0.1)",
                    color: "var(--text-muted, #94a3b8)",
                    fontSize: "0.75rem",
                    cursor: "pointer",
                  }}
                >
                  {isExpanded ? "Hide Details ▲" : "Details ▼"}
                </button>
              </div>
            </div>

            {/* Middle metrics grid */}
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(auto-fit, minmax(140px, 1fr))",
                gap: "0.75rem",
                padding: "0.6rem 0.8rem",
                borderRadius: "6px",
                background: "rgba(0, 0, 0, 0.2)",
                fontSize: "0.8rem",
              }}
            >
              <div>
                <span style={{ color: "var(--text-muted, #94a3b8)", display: "block", fontSize: "0.7rem" }}>Next Fire</span>
                <span style={{ fontWeight: 500, color: "#fff" }}>{formatDateTime(task.next_fire_at)}</span>
              </div>
              <div>
                <span style={{ color: "var(--text-muted, #94a3b8)", display: "block", fontSize: "0.7rem" }}>Last Fired</span>
                <span style={{ color: "var(--text-muted, #94a3b8)" }}>{formatDateTime(task.last_fired_at)}</span>
              </div>
              <div>
                <span style={{ color: "var(--text-muted, #94a3b8)", display: "block", fontSize: "0.7rem" }}>Attempts</span>
                <span style={{ fontWeight: 500, color: task.attempts >= task.max_attempts ? "#ef4444" : "#fff" }}>
                  {task.attempts} / {task.max_attempts}
                </span>
              </div>
              <div>
                <span style={{ color: "var(--text-muted, #94a3b8)", display: "block", fontSize: "0.7rem" }}>Cadence</span>
                <span style={{ color: "#fff" }}>
                  {task.repeat_interval_secs ? `Every ${Math.round(task.repeat_interval_secs / 60)}m` : "One-shot"}
                </span>
              </div>
              <div>
                <span style={{ color: "var(--text-muted, #94a3b8)", display: "block", fontSize: "0.7rem" }}>Gate Criteria</span>
                <span style={{ color: "#fff" }}>
                  {task.gate_criteria.length} criteria ({task.gate_combinator.toUpperCase()})
                </span>
              </div>
            </div>

            {/* Expanded details */}
            {isExpanded && (
              <div
                style={{
                  display: "flex",
                  flexDirection: "column",
                  gap: "0.6rem",
                  paddingTop: "0.5rem",
                  borderTop: "1px solid rgba(255, 255, 255, 0.06)",
                  fontSize: "0.8rem",
                }}
              >
                <div>
                  <strong style={{ color: "var(--text-muted, #94a3b8)", display: "block", fontSize: "0.75rem", marginBottom: "0.2rem" }}>
                    Instruction:
                  </strong>
                  <div style={{ whiteSpace: "pre-wrap", background: "rgba(0,0,0,0.3)", padding: "0.6rem", borderRadius: "6px", color: "#e2e8f0" }}>
                    {task.instruction}
                  </div>
                </div>

                {task.gate_criteria.length > 0 && (
                  <div>
                    <strong style={{ color: "var(--text-muted, #94a3b8)", display: "block", fontSize: "0.75rem", marginBottom: "0.2rem" }}>
                      Gate Criteria ({task.gate_combinator.toUpperCase()}):
                    </strong>
                    <div style={{ display: "flex", flexDirection: "column", gap: "0.3rem" }}>
                      {task.gate_criteria.map((c, i) => (
                        <div key={i} style={{ background: "rgba(0,0,0,0.2)", padding: "0.4rem 0.6rem", borderRadius: "4px" }}>
                          {c.kind === "command" && (
                            <span>🖥️ <code>{c.program} {c.args.join(" ")}</code> ({c.check === "exit_code_zero" ? "exit code 0" : "at most max"})</span>
                          )}
                          {c.kind === "max_lines_per_file" && (
                            <span>📏 Max lines ≤ {c.max_lines} on <code>{c.globs.join(", ")}</code></span>
                          )}
                          {c.kind === "min_tokens_written" && (
                            <span>✍️ Min tokens delta ≥ {c.min_tokens}</span>
                          )}
                        </div>
                      ))}
                    </div>
                  </div>
                )}

                <div style={{ display: "flex", gap: "1rem", flexWrap: "wrap", fontSize: "0.75rem", color: "var(--text-muted, #94a3b8)" }}>
                  <span>Created by: <code>{task.created_by}</code></span>
                  <span>Created: {formatDateTime(task.created_at)}</span>
                  {task.deadline && <span>Deadline: {formatDateTime(task.deadline)}</span>}
                  {task.budget.credit_allowance != null && (
                    <span>Credit Allowance: {task.budget.credit_allowance} (Spent: {task.budget.credit_spent})</span>
                  )}
                </div>
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
