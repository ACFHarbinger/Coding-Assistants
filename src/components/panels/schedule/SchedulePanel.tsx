import { useEffect, useState, useCallback } from "react";
import {
  cancelScheduledTask,
  createScheduledTask,
  listScheduledTasks,
  tickScheduledTasks,
} from "./scheduleApi";
import { ScheduleTaskBuilder } from "./ScheduleTaskBuilder";
import { ScheduleTaskList } from "./ScheduleTaskList";
import type { ScheduleCreateArgs, ScheduledTask, TickSummary } from "./types";

interface SchedulePanelProps {
  workspace: string;
  availableAgents?: string[];
  teamMemberIds?: string[];
}

export function SchedulePanel({
  workspace,
  availableAgents = [],
  teamMemberIds = [],
}: SchedulePanelProps) {
  const [tasks, setTasks] = useState<ScheduledTask[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [showBuilder, setShowBuilder] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [cancellingId, setCancellingId] = useState<string | null>(null);

  const [ticking, setTicking] = useState(false);
  const [lastTickSummary, setLastTickSummary] = useState<TickSummary | null>(null);

  const [filter, setFilter] = useState<"all" | "active" | "terminal">("all");

  const loadTasks = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await listScheduledTasks();
      setTasks(data);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadTasks();
  }, [loadTasks]);

  const handleCreateTask = async (args: ScheduleCreateArgs) => {
    setSubmitting(true);
    try {
      await createScheduledTask(args);
      setShowBuilder(false);
      await loadTasks();
    } finally {
      setSubmitting(false);
    }
  };

  const handleCancelTask = async (id: string) => {
    setCancellingId(id);
    try {
      await cancelScheduledTask(id);
      await loadTasks();
    } catch (err) {
      setError(`Failed to cancel task: ${String(err)}`);
    } finally {
      setCancellingId(null);
    }
  };

  const handleRunTickNow = async () => {
    setTicking(true);
    setError(null);
    try {
      const summary = await tickScheduledTasks();
      setLastTickSummary(summary);
      await loadTasks();
    } catch (err) {
      setError(`Manual tick check failed: ${String(err)}`);
    } finally {
      setTicking(false);
    }
  };

  const candidateAgents = Array.from(
    new Set([...teamMemberIds, ...availableAgents].filter(Boolean)),
  );

  const filteredTasks = tasks.filter((t) => {
    if (filter === "active") {
      return t.status === "scheduled" || t.status === "active" || t.status === "paused";
    }
    if (filter === "terminal") {
      return t.status === "completed" || t.status === "cancelled" || t.status === "failed";
    }
    return true;
  });

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "1.5rem" }}>
      {/* Top Header Bar */}
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: "1rem" }}>
        <div>
          <h3 style={{ margin: 0, fontSize: "1.25rem", color: "var(--text-color, #fff)" }}>
            ⏱️ Scheduled Goals
          </h3>
          <p style={{ margin: "0.25rem 0 0 0", fontSize: "0.85rem", color: "var(--text-muted, #94a3b8)" }}>
            Automated recurring dispatches and gate-evaluated tasks targeting standing agents or the whole team.
          </p>
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: "0.75rem", flexWrap: "wrap" }}>
          <button
            type="button"
            onClick={() => void handleRunTickNow()}
            disabled={ticking}
            className="btn-secondary"
            style={{
              marginTop: 0,
              padding: "0.4rem 0.85rem",
              fontSize: "0.85rem",
              display: "inline-flex",
              alignItems: "center",
              gap: "0.35rem",
              borderRadius: "8px",
            }}
          >
            {ticking ? "⚡ Checking…" : "⚡ Run Check Now"}
          </button>

          <button
            type="button"
            onClick={() => void loadTasks()}
            disabled={loading}
            className="btn-secondary"
            style={{ marginTop: 0, padding: "0.4rem 0.85rem", fontSize: "0.85rem", borderRadius: "8px" }}
          >
            {loading ? "Refreshing…" : "🔄 Refresh"}
          </button>

          <button
            type="button"
            onClick={() => setShowBuilder((prev) => !prev)}
            className="btn-primary"
            style={{ marginTop: 0, padding: "0.4rem 1rem", fontSize: "0.85rem", borderRadius: "8px" }}
          >
            {showBuilder ? "✕ Close Form" : "➕ New Scheduled Task"}
          </button>
        </div>
      </div>

      {/* Tick Feedback Toast */}
      {lastTickSummary && (
        <div
          style={{
            padding: "0.6rem 1rem",
            borderRadius: "8px",
            background: "rgba(6, 182, 212, 0.12)",
            border: "1px solid rgba(6, 182, 212, 0.3)",
            color: "#67e8f9",
            fontSize: "0.8rem",
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
          }}
        >
          <span>
            ⚡ <strong>Evaluated at {new Date(lastTickSummary.evaluated_at).toLocaleTimeString()}:</strong> Checked {lastTickSummary.tasks_checked} tasks — {lastTickSummary.dispatched} dispatched, {lastTickSummary.re_prompted} re-prompted, {lastTickSummary.completed} completed, {lastTickSummary.paused} paused, {lastTickSummary.failed} failed.
          </span>
          <button
            type="button"
            onClick={() => setLastTickSummary(null)}
            style={{ background: "transparent", border: "none", color: "#67e8f9", cursor: "pointer" }}
          >
            ✕
          </button>
        </div>
      )}

      {/* Error Banner */}
      {error && (
        <div
          style={{
            padding: "0.75rem 1rem",
            borderRadius: "8px",
            background: "rgba(239, 68, 68, 0.15)",
            border: "1px solid rgba(239, 68, 68, 0.3)",
            color: "#ef4444",
            fontSize: "0.85rem",
          }}
        >
          {error}
        </div>
      )}

      {/* Wake Policy Info Banner */}
      <div
        style={{
          padding: "0.6rem 0.9rem",
          borderRadius: "8px",
          background: "rgba(99, 102, 241, 0.08)",
          border: "1px solid rgba(99, 102, 241, 0.2)",
          color: "#c7d2fe",
          fontSize: "0.8rem",
          lineHeight: 1.4,
        }}
      >
        ℹ️ <strong>Wake Policy &amp; Safety:</strong> Dispatches request task advancement under the active <code>WakePolicy</code>. Continuous execution without human approval on each step requires <code>allow_auto_wake</code> enabled in Settings.
      </div>

      {/* Task Creation Form (conditional) */}
      {showBuilder && (
        <ScheduleTaskBuilder
          workspace={workspace}
          availableAgents={candidateAgents}
          onSubmit={handleCreateTask}
          onCancel={() => setShowBuilder(false)}
          submitting={submitting}
        />
      )}

      {/* Filter Tabs */}
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: "0.5rem" }}>
        <div style={{ display: "flex", gap: "0.3rem", background: "rgba(0,0,0,0.2)", padding: "0.2rem", borderRadius: "8px" }}>
          <button
            type="button"
            onClick={() => setFilter("all")}
            style={{
              padding: "0.3rem 0.75rem",
              borderRadius: "6px",
              border: "none",
              fontSize: "0.8rem",
              cursor: "pointer",
              background: filter === "all" ? "rgba(255, 255, 255, 0.15)" : "transparent",
              color: filter === "all" ? "#fff" : "var(--text-muted, #94a3b8)",
            }}
          >
            All ({tasks.length})
          </button>
          <button
            type="button"
            onClick={() => setFilter("active")}
            style={{
              padding: "0.3rem 0.75rem",
              borderRadius: "6px",
              border: "none",
              fontSize: "0.8rem",
              cursor: "pointer",
              background: filter === "active" ? "rgba(255, 255, 255, 0.15)" : "transparent",
              color: filter === "active" ? "#fff" : "var(--text-muted, #94a3b8)",
            }}
          >
            Active &amp; Scheduled ({tasks.filter((t) => t.status === "scheduled" || t.status === "active" || t.status === "paused").length})
          </button>
          <button
            type="button"
            onClick={() => setFilter("terminal")}
            style={{
              padding: "0.3rem 0.75rem",
              borderRadius: "6px",
              border: "none",
              fontSize: "0.8rem",
              cursor: "pointer",
              background: filter === "terminal" ? "rgba(255, 255, 255, 0.15)" : "transparent",
              color: filter === "terminal" ? "#fff" : "var(--text-muted, #94a3b8)",
            }}
          >
            Completed &amp; Cancelled ({tasks.filter((t) => t.status === "completed" || t.status === "cancelled" || t.status === "failed").length})
          </button>
        </div>
      </div>

      {/* Task List */}
      <ScheduleTaskList
        tasks={filteredTasks}
        onCancelTask={handleCancelTask}
        cancellingId={cancellingId}
      />
    </div>
  );
}
