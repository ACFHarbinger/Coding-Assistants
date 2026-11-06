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
import { nocPanelStyle, nocCardDenseStyle } from "../../common/telemetry/panel";
import { NOC_COLORS, NOC_SURFACES } from "../../common/telemetry/tokens";

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
    <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1.25rem" }}>
      {/* Top Header Bar */}
      <div style={{ ...nocPanelStyle, display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: "1rem" }}>
        <div>
          <h3 style={{ margin: 0, fontSize: "1.15rem", fontWeight: 700, color: NOC_SURFACES.textPrimary }}>
            ⏱️ Scheduled Goals
          </h3>
          <p style={{ margin: "0.2rem 0 0", fontSize: "0.82rem", color: NOC_SURFACES.textMuted }}>
            Automated recurring dispatches and gate-evaluated tasks targeting standing agents or the team.
          </p>
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: "0.65rem", flexWrap: "wrap" }}>
          <button
            type="button"
            onClick={() => void handleRunTickNow()}
            disabled={ticking}
            className="btn-secondary noc-focusable"
            style={{
              marginTop: 0,
              padding: "0.35rem 0.75rem",
              fontSize: "0.82rem",
              display: "inline-flex",
              alignItems: "center",
              gap: "0.35rem",
            }}
          >
            {ticking ? "⚡ Checking…" : "⚡ Run Check Now"}
          </button>

          <button
            type="button"
            onClick={() => void loadTasks()}
            disabled={loading}
            className="btn-secondary noc-focusable"
            style={{ marginTop: 0, padding: "0.35rem 0.75rem", fontSize: "0.82rem" }}
          >
            {loading ? "Refreshing…" : "🔄 Refresh"}
          </button>

          <button
            type="button"
            onClick={() => setShowBuilder((prev) => !prev)}
            className="btn-primary noc-focusable"
            style={{ marginTop: 0, padding: "0.35rem 0.9rem", fontSize: "0.82rem" }}
          >
            {showBuilder ? "✕ Close Form" : "➕ New Scheduled Task"}
          </button>
        </div>
      </div>

      {/* Tick Feedback Toast */}
      {lastTickSummary && (
        <div
          style={{
            ...nocCardDenseStyle,
            color: NOC_COLORS.cyan,
            fontSize: "0.82rem",
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            padding: "0.5rem 0.85rem",
          }}
        >
          <span className="tabular-nums">
            ⚡ Checked {new Set(lastTickSummary.outcomes.map((outcome) => outcome.id)).size} tasks — {lastTickSummary.outcomes.filter((outcome) => outcome.action === "dispatched").length} dispatched, {lastTickSummary.outcomes.filter((outcome) => outcome.action === "completed").length} completed, {lastTickSummary.outcomes.filter((outcome) => outcome.action === "paused").length} paused, {lastTickSummary.outcomes.filter((outcome) => outcome.action === "failed").length} failed.
          </span>
          <button
            type="button"
            onClick={() => setLastTickSummary(null)}
            style={{ background: "transparent", border: "none", color: NOC_COLORS.cyan, cursor: "pointer" }}
          >
            ✕
          </button>
        </div>
      )}

      {/* Error Banner */}
      {error && (
        <div
          style={{
            padding: "0.6rem 0.9rem",
            borderRadius: "8px",
            background: "rgba(239, 68, 68, 0.15)",
            border: "1px solid rgba(239, 68, 68, 0.3)",
            color: NOC_COLORS.crit,
            fontSize: "0.84rem",
          }}
        >
          {error}
        </div>
      )}

      {/* Wake Policy Info Banner */}
      <div
        style={{
          padding: "0.55rem 0.85rem",
          borderRadius: "8px",
          background: "rgba(99, 102, 241, 0.1)",
          border: "1px solid rgba(99, 102, 241, 0.25)",
          color: "#c7d2fe",
          fontSize: "0.8rem",
          lineHeight: 1.4,
        }}
      >
        ℹ️ <strong>Wake Policy &amp; Safety:</strong> Dispatches advance under the active <code>WakePolicy</code>. Continuous execution without human approval on each step requires <code>allow_auto_wake</code> enabled in Settings.
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
        <div style={{ display: "flex", gap: "0.35rem", background: "rgba(2, 6, 23, 0.6)", padding: "0.25rem", borderRadius: "8px", border: "1px solid rgba(148, 163, 184, 0.18)" }}>
          <button
            type="button"
            className="noc-focusable"
            onClick={() => setFilter("all")}
            style={{
              padding: "0.3rem 0.75rem",
              borderRadius: "6px",
              border: "none",
              fontSize: "0.8rem",
              cursor: "pointer",
              fontWeight: filter === "all" ? 700 : 500,
              background: filter === "all" ? "rgba(56, 189, 248, 0.2)" : "transparent",
              color: filter === "all" ? NOC_COLORS.cyan : NOC_SURFACES.textMuted,
            }}
          >
            All <span className="tabular-nums">({tasks.length})</span>
          </button>
          <button
            type="button"
            className="noc-focusable"
            onClick={() => setFilter("active")}
            style={{
              padding: "0.3rem 0.75rem",
              borderRadius: "6px",
              border: "none",
              fontSize: "0.8rem",
              cursor: "pointer",
              fontWeight: filter === "active" ? 700 : 500,
              background: filter === "active" ? "rgba(56, 189, 248, 0.2)" : "transparent",
              color: filter === "active" ? NOC_COLORS.cyan : NOC_SURFACES.textMuted,
            }}
          >
            Active &amp; Scheduled <span className="tabular-nums">({tasks.filter((t) => t.status === "scheduled" || t.status === "active" || t.status === "paused").length})</span>
          </button>
          <button
            type="button"
            className="noc-focusable"
            onClick={() => setFilter("terminal")}
            style={{
              padding: "0.3rem 0.75rem",
              borderRadius: "6px",
              border: "none",
              fontSize: "0.8rem",
              cursor: "pointer",
              fontWeight: filter === "terminal" ? 700 : 500,
              background: filter === "terminal" ? "rgba(56, 189, 248, 0.2)" : "transparent",
              color: filter === "terminal" ? NOC_COLORS.cyan : NOC_SURFACES.textMuted,
            }}
          >
            Completed &amp; Cancelled <span className="tabular-nums">({tasks.filter((t) => t.status === "completed" || t.status === "cancelled" || t.status === "failed").length})</span>
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
