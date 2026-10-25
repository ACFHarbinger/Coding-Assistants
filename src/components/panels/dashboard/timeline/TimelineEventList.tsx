import React from "react";
import type { TaskTimelineEvent } from "./types";
import { nocCardDenseStyle } from "../../../common/telemetry/panel";
import { NOC_COLORS, NOC_SURFACES } from "../../../common/telemetry/tokens";

interface TimelineEventListProps {
  events: TaskTimelineEvent[];
  selectedEventId: string | null;
  onSelectEvent: (event: TaskTimelineEvent) => void;
}

const statusColor: Record<string, { bg: string; text: string; icon: string }> = {
  completed: { bg: "rgba(16, 185, 129, 0.15)", text: NOC_COLORS.ok, icon: "✓" },
  in_progress: { bg: "rgba(56, 189, 248, 0.15)", text: NOC_COLORS.cyan, icon: "●" },
  paused: { bg: "rgba(245, 158, 11, 0.18)", text: NOC_COLORS.amber, icon: "⏸" },
  failed: { bg: "rgba(239, 68, 68, 0.18)", text: NOC_COLORS.crit, icon: "✕" },
  pending: { bg: "rgba(148, 163, 184, 0.12)", text: NOC_COLORS.idle, icon: "○" },
};

const eventTypeBadge: Record<string, { bg: string; text: string; label: string }> = {
  task_created: { bg: "rgba(148, 163, 184, 0.16)", text: NOC_SURFACES.textPrimary, label: "Task" },
  workflow_step: { bg: "rgba(56, 189, 248, 0.16)", text: NOC_COLORS.cyan, label: "Step" },
  transition: { bg: "rgba(99, 102, 241, 0.18)", text: NOC_COLORS.indigo, label: "Transition" },
  handoff: { bg: "rgba(168, 85, 247, 0.18)", text: NOC_COLORS.purple, label: "Handoff" },
  message: { bg: "rgba(6, 182, 212, 0.16)", text: NOC_COLORS.teal, label: "Message" },
  wake: { bg: "rgba(249, 115, 22, 0.18)", text: NOC_COLORS.orange, label: "Wake" },
  tool: { bg: "rgba(34, 197, 94, 0.18)", text: NOC_COLORS.emerald, label: "Tool" },
  file: { bg: "rgba(236, 72, 153, 0.18)", text: NOC_COLORS.pink, label: "File" },
  provider_call: { bg: "rgba(56, 189, 248, 0.16)", text: NOC_COLORS.cyan, label: "Provider" },
  gate: { bg: "rgba(234, 179, 8, 0.18)", text: NOC_COLORS.warn, label: "Gate" },
};

function formatEventTime(timestamp: string): string {
  try {
    const d = new Date(timestamp);
    return isNaN(d.getTime())
      ? timestamp
      : d.toLocaleTimeString([], { hour12: false, hour: "2-digit", minute: "2-digit", second: "2-digit" });
  } catch {
    return timestamp;
  }
}

export const TimelineEventList: React.FC<TimelineEventListProps> = ({
  events,
  selectedEventId,
  onSelectEvent,
}) => {
  if (events.length === 0) {
    return (
      <div
        style={{
          ...nocCardDenseStyle,
          padding: "2rem",
          textAlign: "center",
          color: NOC_SURFACES.textMuted,
        }}
      >
        <p style={{ margin: 0, fontSize: "0.9rem" }}>No events match the selected criteria.</p>
      </div>
    );
  }

  return (
    <div
      role="feed"
      aria-label="Chronological task event feed"
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "0.5rem",
      }}
    >
      {events.map((event, idx) => {
        const isSelected = selectedEventId === event.id;
        const status = statusColor[event.status] || statusColor.pending;
        const typeBadge = eventTypeBadge[event.event_type] || {
          bg: "rgba(148, 163, 184, 0.15)",
          text: NOC_SURFACES.textPrimary,
          label: event.event_type,
        };

        const isProblematic = event.status === "failed" || event.status === "paused";

        return (
          <div
            key={event.id}
            role="article"
            tabIndex={0}
            aria-selected={isSelected}
            onClick={() => onSelectEvent(event)}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                onSelectEvent(event);
              }
            }}
            className="noc-focusable"
            style={{
              ...nocCardDenseStyle,
              padding: "0.65rem 0.85rem",
              cursor: "pointer",
              transition: "border-color 0.15s ease, background-color 0.15s ease",
              background: isSelected
                ? "rgba(30, 41, 59, 0.85)"
                : isProblematic && event.status === "failed"
                  ? "rgba(239, 68, 68, 0.08)"
                  : isProblematic && event.status === "paused"
                    ? "rgba(245, 158, 11, 0.08)"
                    : "rgba(15, 23, 42, 0.65)",
              border: `1px solid ${
                isSelected
                  ? NOC_COLORS.cyan
                  : event.status === "failed"
                    ? "rgba(239, 68, 68, 0.45)"
                    : event.status === "paused"
                      ? "rgba(245, 158, 11, 0.45)"
                      : NOC_SURFACES.border
              }`,
              display: "flex",
              flexDirection: "column",
              gap: "0.35rem",
            }}
          >
            {/* Top metadata strip */}
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                gap: "0.75rem",
                flexWrap: "wrap",
              }}
            >
              <div style={{ display: "flex", alignItems: "center", gap: "0.45rem", flexWrap: "wrap" }}>
                <span
                  className="tabular-nums"
                  style={{
                    fontSize: "0.72rem",
                    fontWeight: 700,
                    color: NOC_SURFACES.textMuted,
                  }}
                >
                  #{idx + 1}
                </span>

                <span
                  style={{
                    fontSize: "0.7rem",
                    padding: "0.1rem 0.4rem",
                    borderRadius: "4px",
                    background: typeBadge.bg,
                    color: typeBadge.text,
                    fontWeight: 700,
                    textTransform: "uppercase",
                  }}
                >
                  {typeBadge.label}
                </span>

                <span
                  style={{
                    fontSize: "0.76rem",
                    fontWeight: 600,
                    color: NOC_COLORS.cyan,
                  }}
                >
                  {event.actor}
                </span>

                {event.target && (
                  <>
                    <span style={{ fontSize: "0.7rem", color: NOC_SURFACES.textMuted }}>➔</span>
                    <span
                      style={{
                        fontSize: "0.76rem",
                        fontWeight: 600,
                        color: NOC_COLORS.purple,
                      }}
                    >
                      {event.target}
                    </span>
                  </>
                )}
              </div>

              <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
                <span
                  className="tabular-nums"
                  style={{
                    fontSize: "0.74rem",
                    color: NOC_SURFACES.textMuted,
                  }}
                >
                  {formatEventTime(event.timestamp)}
                </span>

                <span
                  style={{
                    fontSize: "0.7rem",
                    padding: "0.1rem 0.45rem",
                    borderRadius: "4px",
                    background: status.bg,
                    color: status.text,
                    fontWeight: 700,
                    display: "inline-flex",
                    alignItems: "center",
                    gap: "0.25rem",
                  }}
                >
                  <span>{status.icon}</span>
                  <span>{event.status}</span>
                </span>
              </div>
            </div>

            {/* Event Summary */}
            <div
              style={{
                fontSize: "0.85rem",
                color: NOC_SURFACES.textPrimary,
                fontWeight: 500,
              }}
            >
              {event.summary}
            </div>

            {/* Detail snippet if present */}
            {event.detail && (
              <div
                style={{
                  fontSize: "0.76rem",
                  color: NOC_SURFACES.textMuted,
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  whiteSpace: "nowrap",
                }}
              >
                {event.detail}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
};
