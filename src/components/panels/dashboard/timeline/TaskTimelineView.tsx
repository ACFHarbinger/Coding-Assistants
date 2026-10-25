import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "../../../../lib/tauri";
import type {
  AgentRecord,
  TaskRecord,
  TaskTimelineEvent,
  TaskTimelineView as ITaskTimelineView,
  TimelineEventStatus,
  TimelineEventType,
} from "./types";
import { TimelineLaneChart } from "./TimelineLaneChart";
import { TimelineEventList } from "./TimelineEventList";
import { TimelineDetailDrawer } from "./TimelineDetailDrawer";
import { nocPanelStyle, nocCardDenseStyle } from "../../../common/telemetry/panel";
import { NOC_COLORS, NOC_SURFACES } from "../../../common/telemetry/tokens";

interface TaskTimelineViewProps {
  tasks: TaskRecord[];
  agents?: AgentRecord[];
  initialTaskId?: string;
}

const statusBadgeColor: Record<string, { bg: string; text: string }> = {
  completed: { bg: "rgba(16, 185, 129, 0.15)", text: NOC_COLORS.ok },
  done: { bg: "rgba(16, 185, 129, 0.15)", text: NOC_COLORS.ok },
  in_progress: { bg: "rgba(56, 189, 248, 0.15)", text: NOC_COLORS.cyan },
  running: { bg: "rgba(56, 189, 248, 0.15)", text: NOC_COLORS.cyan },
  active: { bg: "rgba(56, 189, 248, 0.15)", text: NOC_COLORS.cyan },
  paused: { bg: "rgba(245, 158, 11, 0.2)", text: NOC_COLORS.amber },
  input_required: { bg: "rgba(245, 158, 11, 0.2)", text: NOC_COLORS.amber },
  auth_required: { bg: "rgba(245, 158, 11, 0.2)", text: NOC_COLORS.amber },
  failed: { bg: "rgba(239, 68, 68, 0.2)", text: NOC_COLORS.crit },
  cancelled: { bg: "rgba(148, 163, 184, 0.12)", text: NOC_COLORS.idle },
  pending: { bg: "rgba(148, 163, 184, 0.12)", text: NOC_COLORS.idle },
};

const EVENT_TYPE_OPTIONS: { value: TimelineEventType | "all"; label: string }[] = [
  { value: "all", label: "All Event Types" },
  { value: "task_created", label: "Task Created" },
  { value: "workflow_step", label: "Workflow Step" },
  { value: "transition", label: "Transition" },
  { value: "handoff", label: "Handoff" },
  { value: "message", label: "Message" },
  { value: "wake", label: "Wake" },
  { value: "tool", label: "Tool Activity" },
  { value: "file", label: "File Touch" },
  { value: "provider_call", label: "Provider Call" },
  { value: "gate", label: "Gate Approval" },
];

const STATUS_OPTIONS: { value: TimelineEventStatus | "all"; label: string }[] = [
  { value: "all", label: "All Statuses" },
  { value: "completed", label: "Completed" },
  { value: "in_progress", label: "In Progress" },
  { value: "paused", label: "Paused" },
  { value: "failed", label: "Failed" },
  { value: "pending", label: "Pending" },
];

const selectStyle: React.CSSProperties = {
  background: "rgba(15, 23, 42, 0.8)",
  border: `1px solid ${NOC_SURFACES.borderStrong}`,
  borderRadius: "5px",
  color: NOC_SURFACES.textPrimary,
  padding: "0.25rem 0.5rem",
  fontSize: "0.78rem",
};

function formatDuration(seconds: number | null | undefined): string {
  if (seconds === null || seconds === undefined || isNaN(seconds)) return "--";
  if (seconds < 60) return `${Math.round(seconds)}s`;
  const m = Math.floor(seconds / 60);
  const s = Math.round(seconds % 60);
  return `${m}m ${s}s`;
}

export const TaskTimelineView: React.FC<TaskTimelineViewProps> = ({
  tasks,
  initialTaskId,
}) => {
  const [selectedTaskId, setSelectedTaskId] = useState<string>(
    initialTaskId || (tasks.length > 0 ? tasks[0].id : ""),
  );
  const [timeline, setTimeline] = useState<ITaskTimelineView | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [viewMode, setViewMode] = useState<"swimlane" | "list">("swimlane");
  const [selectedAgent, setSelectedAgent] = useState("all");
  const [selectedType, setSelectedType] = useState<TimelineEventType | "all">("all");
  const [selectedStatus, setSelectedStatus] = useState<TimelineEventStatus | "all">("all");
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedEvent, setSelectedEvent] = useState<TaskTimelineEvent | null>(null);

  const requestId = useRef(0);
  const currentTaskId = useRef(selectedTaskId);
  currentTaskId.current = selectedTaskId;

  useEffect(() => {
    if (!tasks.some((task) => task.id === selectedTaskId)) {
      setSelectedTaskId(tasks[0]?.id ?? "");
    }
  }, [tasks, selectedTaskId]);

  const loadTimeline = useCallback(async (taskId: string) => {
    const request = ++requestId.current;
    if (!taskId) {
      setTimeline(null);
      setSelectedEvent(null);
      setLoading(false);
      return;
    }
    setTimeline(null);
    setSelectedEvent(null);
    setLoading(true);
    setError("");
    try {
      const data = await invoke<ITaskTimelineView>("hub_get_task_timeline", { taskId });
      if (request !== requestId.current || taskId !== currentTaskId.current) return;
      setTimeline(data);
    } catch (e) {
      if (request !== requestId.current || taskId !== currentTaskId.current) return;
      setError(String(e));
      setTimeline(null);
    } finally {
      if (request === requestId.current && taskId === currentTaskId.current) setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadTimeline(selectedTaskId);
    return () => { requestId.current += 1; };
  }, [selectedTaskId, loadTimeline]);

  const filteredEvents = useMemo(() => {
    if (!timeline) return [];
    return timeline.events.filter((evt) => {
      if (selectedAgent !== "all" && evt.actor !== selectedAgent && evt.target !== selectedAgent) {
        return false;
      }
      if (selectedType !== "all" && evt.event_type !== selectedType) return false;
      if (selectedStatus !== "all" && evt.status !== selectedStatus) return false;
      if (searchQuery.trim()) {
        const q = searchQuery.toLowerCase();
        const mSummary = evt.summary.toLowerCase().includes(q);
        const mActor = evt.actor.toLowerCase().includes(q);
        const mTarget = evt.target?.toLowerCase().includes(q) ?? false;
        const mDetail = evt.detail?.toLowerCase().includes(q) ?? false;
        const mType = evt.event_type.toLowerCase().includes(q);
        if (!mSummary && !mActor && !mTarget && !mDetail && !mType) return false;
      }
      return true;
    });
  }, [timeline, selectedAgent, selectedType, selectedStatus, searchQuery]);

  return (
    <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
      {/* Top Selector & KPI Panel */}
      <div style={nocPanelStyle}>
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            flexWrap: "wrap",
            gap: "1rem",
            marginBottom: "0.85rem",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: "0.75rem", flexWrap: "wrap" }}>
            <label
              htmlFor="task-selector"
              style={{
                fontSize: "0.82rem",
                fontWeight: 700,
                color: NOC_SURFACES.textMuted,
                textTransform: "uppercase",
                letterSpacing: "0.04em",
              }}
            >
              Task:
            </label>
            <select
              id="task-selector"
              value={selectedTaskId}
              onChange={(e) => setSelectedTaskId(e.target.value)}
              className="noc-focusable"
              style={{
                ...selectStyle,
                padding: "0.35rem 0.75rem",
                fontSize: "0.85rem",
                minWidth: "240px",
              }}
            >
              {tasks.length === 0 && <option value="">No tasks available</option>}
              {tasks.map((t) => (
                <option key={t.id} value={t.id}>
                  {t.title} ({t.status})
                </option>
              ))}
            </select>
          </div>

          <button
            type="button"
            className="btn-secondary noc-focusable"
            style={{ fontSize: "0.8rem", padding: "0.35rem 0.85rem" }}
            onClick={() => void loadTimeline(selectedTaskId)}
            disabled={loading || !selectedTaskId}
          >
            {loading ? "Refreshing..." : "Refresh Timeline"}
          </button>
        </div>

        {/* Task Metrics & Status */}
        {timeline && (
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(auto-fit, minmax(130px, 1fr))",
              gap: "0.6rem",
            }}
          >
            <div style={{ ...nocCardDenseStyle, padding: "0.45rem 0.65rem" }}>
              <div style={{ fontSize: "0.7rem", color: NOC_SURFACES.textMuted }}>STATUS</div>
              <div
                style={{
                  fontSize: "0.95rem",
                  fontWeight: 700,
                  marginTop: "0.15rem",
                  color: statusBadgeColor[timeline.status]?.text || NOC_SURFACES.textPrimary,
                  textTransform: "uppercase",
                }}
              >
                {timeline.status}
              </div>
            </div>

            <div style={{ ...nocCardDenseStyle, padding: "0.45rem 0.65rem" }}>
              <div style={{ fontSize: "0.7rem", color: NOC_SURFACES.textMuted }}>DURATION</div>
              <div
                className="tabular-nums"
                style={{
                  fontSize: "0.95rem",
                  fontWeight: 700,
                  marginTop: "0.15rem",
                  color: NOC_COLORS.cyan,
                }}
              >
                {formatDuration(timeline.duration_seconds)}
              </div>
            </div>

            <div style={{ ...nocCardDenseStyle, padding: "0.45rem 0.65rem" }}>
              <div style={{ fontSize: "0.7rem", color: NOC_SURFACES.textMuted }}>TOTAL EVENTS</div>
              <div
                className="tabular-nums"
                style={{
                  fontSize: "0.95rem",
                  fontWeight: 700,
                  marginTop: "0.15rem",
                  color: NOC_COLORS.indigo,
                }}
              >
                {timeline.events.length}
              </div>
            </div>

            <div style={{ ...nocCardDenseStyle, padding: "0.45rem 0.65rem" }}>
              <div style={{ fontSize: "0.7rem", color: NOC_SURFACES.textMuted }}>AGENTS</div>
              <div
                className="tabular-nums"
                style={{
                  fontSize: "0.95rem",
                  fontWeight: 700,
                  marginTop: "0.15rem",
                  color: NOC_COLORS.purple,
                }}
              >
                {timeline.involved_agents.length}
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Critical Failure Banner */}
      {timeline && (timeline.status === "failed" || timeline.failure_reason) && (
        <div
          role="alert"
          style={{
            background: "rgba(239, 68, 68, 0.12)",
            border: "1px solid rgba(239, 68, 68, 0.45)",
            borderRadius: "8px",
            padding: "0.75rem 1rem",
            color: NOC_COLORS.crit,
            display: "flex",
            alignItems: "center",
            gap: "0.75rem",
          }}
        >
          <span style={{ fontSize: "1.25rem" }}>⚠</span>
          <div>
            <strong>Task Failed:</strong>{" "}
            {timeline.failure_reason || "One or more steps encountered unhandled failures."}
          </div>
        </div>
      )}

      {/* Paused Workflow Banner */}
      {timeline && timeline.is_paused && (
        <div
          role="alert"
          style={{
            background: "rgba(245, 158, 11, 0.14)",
            border: "1px solid rgba(245, 158, 11, 0.45)",
            borderRadius: "8px",
            padding: "0.75rem 1rem",
            color: NOC_COLORS.amber,
            display: "flex",
            alignItems: "center",
            gap: "0.75rem",
          }}
        >
          <span style={{ fontSize: "1.25rem" }}>⏸</span>
          <div>
            <strong>Task Execution Paused:</strong> Workflow is waiting for operator input, human
            gate sign-off, or tool wake resolution.
          </div>
        </div>
      )}

      {error && (
        <div
          role="alert"
          style={{
            ...nocPanelStyle,
            background: "rgba(239, 68, 68, 0.1)",
            color: NOC_COLORS.crit,
          }}
        >
          {error}
        </div>
      )}

      {/* Filter & View Switch Toolbar */}
      {timeline && (
        <div
          style={{
            ...nocPanelStyle,
            padding: "0.75rem 1rem",
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            flexWrap: "wrap",
            gap: "0.75rem",
          }}
        >
          <div style={{ display: "flex", gap: "0.35rem" }}>
            <button
              type="button"
              className={`noc-focusable ${viewMode === "swimlane" ? "btn-primary" : "btn-secondary"}`}
              style={{ fontSize: "0.78rem", padding: "0.25rem 0.65rem" }}
              onClick={() => setViewMode("swimlane")}
            >
              2D Swimlanes
            </button>
            <button
              type="button"
              className={`noc-focusable ${viewMode === "list" ? "btn-primary" : "btn-secondary"}`}
              style={{ fontSize: "0.78rem", padding: "0.25rem 0.65rem" }}
              onClick={() => setViewMode("list")}
            >
              Chronological Feed
            </button>
          </div>

          <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", flexWrap: "wrap" }}>
            <select
              aria-label="Filter by agent"
              value={selectedAgent}
              onChange={(e) => setSelectedAgent(e.target.value)}
              className="noc-focusable"
              style={selectStyle}
            >
              <option value="all">All Agents</option>
              {timeline.involved_agents.map((ag) => (
                <option key={ag} value={ag}>
                  {ag}
                </option>
              ))}
            </select>

            <select
              aria-label="Filter by event type"
              value={selectedType}
              onChange={(e) => setSelectedType(e.target.value as TimelineEventType | "all")}
              className="noc-focusable"
              style={selectStyle}
            >
              {EVENT_TYPE_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.label}
                </option>
              ))}
            </select>

            <select
              aria-label="Filter by event status"
              value={selectedStatus}
              onChange={(e) => setSelectedStatus(e.target.value as TimelineEventStatus | "all")}
              className="noc-focusable"
              style={selectStyle}
            >
              {STATUS_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.label}
                </option>
              ))}
            </select>

            <input
              type="text"
              aria-label="Search events"
              placeholder="Search events..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="noc-focusable"
              style={{ ...selectStyle, width: "150px" }}
            />
          </div>
        </div>
      )}

      {/* Main View Port */}
      {timeline && (
        <>
          {viewMode === "swimlane" ? (
            <TimelineLaneChart
              events={filteredEvents}
              involvedAgents={timeline.involved_agents}
              selectedEventId={selectedEvent?.id ?? null}
              onSelectEvent={setSelectedEvent}
            />
          ) : (
            <TimelineEventList
              events={filteredEvents}
              selectedEventId={selectedEvent?.id ?? null}
              onSelectEvent={setSelectedEvent}
            />
          )}

          <TimelineDetailDrawer
            event={selectedEvent}
            onClose={() => setSelectedEvent(null)}
          />
        </>
      )}

      {tasks.length === 0 && !loading && (
        <div
          style={{
            ...nocCardDenseStyle,
            padding: "2.5rem",
            textAlign: "center",
            color: NOC_SURFACES.textMuted,
          }}
        >
          <p style={{ margin: 0, fontSize: "0.95rem" }}>
            No tasks exist in the shared hub yet. Tasks created by workflows or agents will appear here.
          </p>
        </div>
      )}
    </div>
  );
};

export default TaskTimelineView;
