import React, { useMemo } from "react";
import type { TaskTimelineEvent } from "./types";
import { nocPanelStyle, nocCardDenseStyle } from "../../../common/telemetry/panel";
import { NOC_COLORS, NOC_SURFACES } from "../../../common/telemetry/tokens";

interface TimelineLaneChartProps {
  events: TaskTimelineEvent[];
  involvedAgents: string[];
  selectedEventId: string | null;
  onSelectEvent: (event: TaskTimelineEvent) => void;
}

const statusColor: Record<string, { bg: string; text: string; ring: string }> = {
  completed: {
    bg: "rgba(16, 185, 129, 0.15)",
    text: NOC_COLORS.ok,
    ring: "rgba(16, 185, 129, 0.4)",
  },
  in_progress: {
    bg: "rgba(56, 189, 248, 0.15)",
    text: NOC_COLORS.cyan,
    ring: "rgba(56, 189, 248, 0.4)",
  },
  paused: {
    bg: "rgba(245, 158, 11, 0.2)",
    text: NOC_COLORS.amber,
    ring: "rgba(245, 158, 11, 0.6)",
  },
  failed: {
    bg: "rgba(239, 68, 68, 0.2)",
    text: NOC_COLORS.crit,
    ring: "rgba(239, 68, 68, 0.6)",
  },
  pending: {
    bg: "rgba(148, 163, 184, 0.12)",
    text: NOC_COLORS.idle,
    ring: "rgba(148, 163, 184, 0.25)",
  },
};

const eventTypeIcons: Record<string, string> = {
  task_created: "★",
  workflow_step: "▶",
  transition: "⇄",
  handoff: "➔",
  message: "✉",
  wake: "⚡",
  tool: "⚙",
  file: "☵",
  provider_call: "◈",
  gate: "⚑",
};

export const TimelineLaneChart: React.FC<TimelineLaneChartProps> = ({
  events,
  involvedAgents,
  selectedEventId,
  onSelectEvent,
}) => {
  // Collect all unique actors/targets if involvedAgents is empty
  const laneAgents = useMemo(() => {
    const set = new Set(involvedAgents);
    for (const e of events) {
      if (e.actor) set.add(e.actor);
      if (e.target) set.add(e.target);
    }
    return Array.from(set).filter(Boolean).sort();
  }, [involvedAgents, events]);

  if (laneAgents.length === 0 || events.length === 0) {
    return (
      <div
        style={{
          ...nocCardDenseStyle,
          padding: "2rem",
          textAlign: "center",
          color: NOC_SURFACES.textMuted,
        }}
      >
        <p style={{ margin: 0, fontSize: "0.9rem" }}>No events available to render swimlanes.</p>
      </div>
    );
  }

  const columnWidth = 190;
  const laneHeight = 84;
  const headerWidth = 140;
  const totalChartWidth = headerWidth + events.length * columnWidth;

  return (
    <div
      role="region"
      aria-label="2D Task Timeline Swimlane Visualization"
      style={{
        ...nocPanelStyle,
        padding: "1rem",
        overflowX: "auto",
        position: "relative",
      }}
    >
      <div style={{ minWidth: `${totalChartWidth}px`, position: "relative" }}>
        {/* Step Sequence Header Row */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            borderBottom: `1px solid ${NOC_SURFACES.borderStrong}`,
            paddingBottom: "0.5rem",
            marginBottom: "0.5rem",
          }}
        >
          <div
            style={{
              width: `${headerWidth}px`,
              fontSize: "0.74rem",
              fontWeight: 700,
              color: NOC_SURFACES.textMuted,
              textTransform: "uppercase",
              letterSpacing: "0.04em",
            }}
          >
            Agents / Steps
          </div>

          <div style={{ display: "flex", gap: "10px" }}>
            {events.map((evt, idx) => (
              <div
                key={evt.id}
                style={{
                  width: `${columnWidth - 10}px`,
                  fontSize: "0.72rem",
                  color: NOC_SURFACES.textMuted,
                  display: "flex",
                  justifyContent: "space-between",
                  padding: "0 0.35rem",
                }}
              >
                <span className="tabular-nums" style={{ fontWeight: 700 }}>
                  Step #{idx + 1}
                </span>
                <span
                  className="tabular-nums"
                  style={{
                    fontSize: "0.68rem",
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                  }}
                >
                  {evt.timestamp.slice(11, 19)}
                </span>
              </div>
            ))}
          </div>
        </div>

        {/* Lanes */}
        <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem", position: "relative" }}>
          {laneAgents.map((agentId) => {
            const laneEvents = events.map((e) => (e.actor === agentId ? e : null));

            return (
              <div
                key={agentId}
                style={{
                  display: "flex",
                  alignItems: "center",
                  minHeight: `${laneHeight}px`,
                  background: "rgba(15, 23, 42, 0.45)",
                  borderRadius: "8px",
                  border: `1px solid ${NOC_SURFACES.border}`,
                  padding: "0.4rem 0.6rem",
                }}
              >
                {/* Lane Header */}
                <div
                  style={{
                    width: `${headerWidth - 12}px`,
                    flexShrink: 0,
                    display: "flex",
                    flexDirection: "column",
                    justifyContent: "center",
                    borderRight: `1px solid ${NOC_SURFACES.border}`,
                    paddingRight: "0.5rem",
                  }}
                >
                  <div
                    style={{
                      fontSize: "0.85rem",
                      fontWeight: 700,
                      color: NOC_COLORS.cyan,
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap",
                    }}
                    title={agentId}
                  >
                    {agentId}
                  </div>
                  <div
                    className="tabular-nums"
                    style={{
                      fontSize: "0.7rem",
                      color: NOC_SURFACES.textMuted,
                      marginTop: "0.15rem",
                    }}
                  >
                    {events.filter((e) => e.actor === agentId).length} events
                  </div>
                </div>

                {/* Event Columns in this Lane */}
                <div style={{ display: "flex", gap: "10px", marginLeft: "10px" }}>
                  {laneEvents.map((event, colIdx) => {
                    if (!event) {
                      // Check if this agent is the target of a transition/handoff at this step
                      const stepEvent = events[colIdx];
                      const isTarget = stepEvent.target === agentId;

                      return (
                        <div
                          key={`empty-${colIdx}`}
                          style={{
                            width: `${columnWidth - 10}px`,
                            height: "64px",
                            display: "flex",
                            alignItems: "center",
                            justifyContent: "center",
                            border: `1px dashed ${isTarget ? "rgba(168, 85, 247, 0.4)" : "rgba(148, 163, 184, 0.08)"}`,
                            borderRadius: "6px",
                            background: isTarget ? "rgba(168, 85, 247, 0.08)" : "transparent",
                          }}
                        >
                          {isTarget && (
                            <span
                              style={{
                                fontSize: "0.68rem",
                                color: NOC_COLORS.purple,
                                fontWeight: 700,
                                textTransform: "uppercase",
                              }}
                            >
                              ↙ Receives {stepEvent.event_type}
                            </span>
                          )}
                        </div>
                      );
                    }

                    const isSelected = selectedEventId === event.id;
                    const status = statusColor[event.status] || statusColor.pending;
                    const icon = eventTypeIcons[event.event_type] || "•";
                    const isPaused = event.status === "paused";
                    const isFailed = event.status === "failed";

                    return (
                      <div
                        key={event.id}
                        role="button"
                        tabIndex={0}
                        aria-label={`Event ${colIdx + 1}: ${event.summary} (${event.status})`}
                        onClick={() => onSelectEvent(event)}
                        onKeyDown={(e) => {
                          if (e.key === "Enter" || e.key === " ") {
                            e.preventDefault();
                            onSelectEvent(event);
                          }
                        }}
                        className="noc-focusable"
                        style={{
                          width: `${columnWidth - 10}px`,
                          height: "64px",
                          boxSizing: "border-box",
                          padding: "0.4rem 0.5rem",
                          borderRadius: "6px",
                          cursor: "pointer",
                          background: isSelected
                            ? "rgba(30, 41, 59, 0.95)"
                            : isFailed
                              ? "rgba(239, 68, 68, 0.16)"
                              : isPaused
                                ? "rgba(245, 158, 11, 0.16)"
                                : "rgba(15, 23, 42, 0.85)",
                          border: `1px solid ${
                            isSelected
                              ? NOC_COLORS.cyan
                              : isFailed
                                ? NOC_COLORS.crit
                                : isPaused
                                  ? NOC_COLORS.amber
                                  : status.ring
                          }`,
                          boxShadow: isSelected
                            ? `0 0 10px ${NOC_COLORS.cyan}`
                            : isFailed
                              ? `0 0 8px rgba(239, 68, 68, 0.5)`
                              : isPaused
                                ? `0 0 8px rgba(245, 158, 11, 0.5)`
                                : "none",
                          display: "flex",
                          flexDirection: "column",
                          justifyContent: "space-between",
                          transition: "all 0.15s ease",
                        }}
                      >
                        {/* Event Card Top Header */}
                        <div
                          style={{
                            display: "flex",
                            justifyContent: "space-between",
                            alignItems: "center",
                            gap: "0.25rem",
                          }}
                        >
                          <span
                            style={{
                              fontSize: "0.68rem",
                              fontWeight: 700,
                              color: isFailed
                                ? NOC_COLORS.crit
                                : isPaused
                                  ? NOC_COLORS.amber
                                  : NOC_SURFACES.textPrimary,
                              display: "inline-flex",
                              alignItems: "center",
                              gap: "0.25rem",
                            }}
                          >
                            <span>{icon}</span>
                            <span style={{ textTransform: "uppercase" }}>
                              {event.event_type.replace(/_/g, " ")}
                            </span>
                          </span>

                          <span
                            style={{
                              fontSize: "0.62rem",
                              fontWeight: 700,
                              padding: "0.05rem 0.3rem",
                              borderRadius: "3px",
                              background: status.bg,
                              color: status.text,
                              textTransform: "uppercase",
                            }}
                          >
                            {event.status}
                          </span>
                        </div>

                        {/* Summary */}
                        <div
                          style={{
                            fontSize: "0.74rem",
                            color: NOC_SURFACES.textSecondary,
                            overflow: "hidden",
                            textOverflow: "ellipsis",
                            whiteSpace: "nowrap",
                            fontWeight: 500,
                          }}
                          title={event.summary}
                        >
                          {event.summary}
                        </div>

                        {/* Handoff / Target indicator if applicable */}
                        {event.target && event.target !== event.actor && (
                          <div
                            style={{
                              fontSize: "0.65rem",
                              color: NOC_COLORS.purple,
                              fontWeight: 600,
                              display: "flex",
                              alignItems: "center",
                              gap: "0.2rem",
                            }}
                          >
                            <span>➔ {event.target}</span>
                          </div>
                        )}
                      </div>
                    );
                  })}
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
};
