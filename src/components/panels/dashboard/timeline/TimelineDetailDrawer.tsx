import React from "react";
import type { TaskTimelineEvent } from "./types";
import { nocPanelStyle, nocCardDenseStyle } from "../../../common/telemetry/panel";
import { NOC_COLORS, NOC_SURFACES } from "../../../common/telemetry/tokens";

interface TimelineDetailDrawerProps {
  event: TaskTimelineEvent | null;
  onClose: () => void;
}

const statusBadgeColor: Record<string, { bg: string; text: string; border: string }> = {
  completed: {
    bg: "rgba(16, 185, 129, 0.15)",
    text: NOC_COLORS.ok,
    border: "rgba(16, 185, 129, 0.35)",
  },
  in_progress: {
    bg: "rgba(56, 189, 248, 0.15)",
    text: NOC_COLORS.cyan,
    border: "rgba(56, 189, 248, 0.35)",
  },
  paused: {
    bg: "rgba(245, 158, 11, 0.18)",
    text: NOC_COLORS.amber,
    border: "rgba(245, 158, 11, 0.45)",
  },
  failed: {
    bg: "rgba(239, 68, 68, 0.18)",
    text: NOC_COLORS.crit,
    border: "rgba(239, 68, 68, 0.45)",
  },
  pending: {
    bg: "rgba(148, 163, 184, 0.12)",
    text: NOC_COLORS.idle,
    border: "rgba(148, 163, 184, 0.25)",
  },
};

const eventTypeColor: Record<string, { bg: string; text: string }> = {
  task_created: { bg: "rgba(148, 163, 184, 0.16)", text: NOC_SURFACES.textPrimary },
  workflow_step: { bg: "rgba(56, 189, 248, 0.16)", text: NOC_COLORS.cyan },
  transition: { bg: "rgba(99, 102, 241, 0.18)", text: NOC_COLORS.indigo },
  handoff: { bg: "rgba(168, 85, 247, 0.18)", text: NOC_COLORS.purple },
  message: { bg: "rgba(6, 182, 212, 0.16)", text: NOC_COLORS.teal },
  wake: { bg: "rgba(249, 115, 22, 0.18)", text: NOC_COLORS.orange },
  tool: { bg: "rgba(34, 197, 94, 0.18)", text: NOC_COLORS.emerald },
  file: { bg: "rgba(236, 72, 153, 0.18)", text: NOC_COLORS.pink },
  provider_call: { bg: "rgba(56, 189, 248, 0.16)", text: NOC_COLORS.cyan },
  gate: { bg: "rgba(234, 179, 8, 0.18)", text: NOC_COLORS.warn },
};

export const TimelineDetailDrawer: React.FC<TimelineDetailDrawerProps> = ({ event, onClose }) => {
  if (!event) return null;

  const statusStyle = statusBadgeColor[event.status] || statusBadgeColor.pending;
  const typeStyle = eventTypeColor[event.event_type] || {
    bg: "rgba(148, 163, 184, 0.15)",
    text: NOC_SURFACES.textPrimary,
  };

  const formattedTime = (() => {
    try {
      const d = new Date(event.timestamp);
      return isNaN(d.getTime()) ? event.timestamp : d.toLocaleString();
    } catch {
      return event.timestamp;
    }
  })();

  return (
    <div
      role="region"
      aria-label="Event detail drawer"
      style={{
        ...nocPanelStyle,
        position: "relative",
        marginTop: "1rem",
        border: `1px solid ${
          event.status === "failed"
            ? "rgba(239, 68, 68, 0.45)"
            : event.status === "paused"
              ? "rgba(245, 158, 11, 0.45)"
              : NOC_SURFACES.borderStrong
        }`,
        background: "linear-gradient(180deg, rgba(15, 23, 42, 0.98) 0%, rgba(9, 14, 26, 0.99) 100%)",
      }}
    >
      {/* Header */}
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "flex-start",
          gap: "1rem",
          marginBottom: "0.85rem",
        }}
      >
        <div style={{ display: "flex", flexDirection: "column", gap: "0.35rem" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", flexWrap: "wrap" }}>
            <span
              style={{
                fontSize: "0.72rem",
                padding: "0.15rem 0.55rem",
                borderRadius: "4px",
                background: typeStyle.bg,
                color: typeStyle.text,
                fontWeight: 700,
                textTransform: "uppercase",
                letterSpacing: "0.04em",
              }}
            >
              {event.event_type.replace(/_/g, " ")}
            </span>
            <span
              style={{
                fontSize: "0.72rem",
                padding: "0.15rem 0.55rem",
                borderRadius: "4px",
                background: statusStyle.bg,
                color: statusStyle.text,
                border: `1px solid ${statusStyle.border}`,
                fontWeight: 700,
                textTransform: "uppercase",
              }}
            >
              {event.status.replace(/_/g, " ")}
            </span>
            {event.step_index !== undefined && event.step_index !== null && (
              <span
                className="tabular-nums"
                style={{
                  fontSize: "0.72rem",
                  padding: "0.15rem 0.45rem",
                  borderRadius: "4px",
                  background: "rgba(148, 163, 184, 0.12)",
                  color: NOC_SURFACES.textMuted,
                  fontWeight: 600,
                }}
              >
                Step #{event.step_index}
              </span>
            )}
          </div>
          <h4
            style={{
              margin: "0.25rem 0 0",
              fontSize: "1.05rem",
              fontWeight: 700,
              color: NOC_SURFACES.textPrimary,
            }}
          >
            {event.summary}
          </h4>
        </div>

        <button
          type="button"
          aria-label="Close event details"
          onClick={onClose}
          className="btn-secondary noc-focusable"
          style={{
            padding: "0.25rem 0.6rem",
            fontSize: "0.8rem",
            lineHeight: 1,
            cursor: "pointer",
          }}
        >
          ✕
        </button>
      </div>

      {/* Failure Callout */}
      {event.status === "failed" && (
        <div
          role="alert"
          style={{
            background: "rgba(239, 68, 68, 0.12)",
            border: "1px solid rgba(239, 68, 68, 0.35)",
            borderRadius: "6px",
            padding: "0.6rem 0.85rem",
            marginBottom: "0.85rem",
            color: NOC_COLORS.crit,
            fontSize: "0.85rem",
            display: "flex",
            alignItems: "center",
            gap: "0.5rem",
          }}
        >
          <span style={{ fontSize: "1.1rem" }}>⚠</span>
          <div>
            <strong>Step execution failed:</strong>{" "}
            {event.detail || "Error encountered during operation."}
          </div>
        </div>
      )}

      {/* Paused Callout */}
      {event.status === "paused" && (
        <div
          role="alert"
          style={{
            background: "rgba(245, 158, 11, 0.12)",
            border: "1px solid rgba(245, 158, 11, 0.35)",
            borderRadius: "6px",
            padding: "0.6rem 0.85rem",
            marginBottom: "0.85rem",
            color: NOC_COLORS.amber,
            fontSize: "0.85rem",
            display: "flex",
            alignItems: "center",
            gap: "0.5rem",
          }}
        >
          <span style={{ fontSize: "1.1rem" }}>⏸</span>
          <div>
            <strong>Workflow is currently paused:</strong>{" "}
            {event.detail || "Awaiting human review, gate approval, or operator input."}
          </div>
        </div>
      )}

      {/* Metadata Grid */}
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))",
          gap: "0.65rem",
          marginBottom: "0.85rem",
        }}
      >
        <div style={{ ...nocCardDenseStyle, padding: "0.5rem 0.75rem" }}>
          <div style={{ fontSize: "0.72rem", color: NOC_SURFACES.textMuted }}>Actor</div>
          <div style={{ fontWeight: 600, color: NOC_COLORS.cyan, marginTop: "0.15rem" }}>
            {event.actor}
          </div>
        </div>

        {event.target && (
          <div style={{ ...nocCardDenseStyle, padding: "0.5rem 0.75rem" }}>
            <div style={{ fontSize: "0.72rem", color: NOC_SURFACES.textMuted }}>Target</div>
            <div style={{ fontWeight: 600, color: NOC_COLORS.purple, marginTop: "0.15rem" }}>
              {event.target}
            </div>
          </div>
        )}

        <div style={{ ...nocCardDenseStyle, padding: "0.5rem 0.75rem" }}>
          <div style={{ fontSize: "0.72rem", color: NOC_SURFACES.textMuted }}>Timestamp</div>
          <div
            className="tabular-nums"
            style={{ fontSize: "0.82rem", color: NOC_SURFACES.textSecondary, marginTop: "0.15rem" }}
          >
            {formattedTime}
          </div>
        </div>

        <div style={{ ...nocCardDenseStyle, padding: "0.5rem 0.75rem" }}>
          <div style={{ fontSize: "0.72rem", color: NOC_SURFACES.textMuted }}>Event ID</div>
          <div
            className="tabular-nums"
            style={{
              fontSize: "0.76rem",
              fontFamily: "monospace",
              color: NOC_SURFACES.textMuted,
              marginTop: "0.15rem",
              wordBreak: "break-all",
            }}
          >
            {event.id}
          </div>
        </div>
      </div>

      {/* Detail description */}
      {event.detail && (
        <div style={{ marginBottom: "0.85rem" }}>
          <div style={{ fontSize: "0.75rem", color: NOC_SURFACES.textMuted, marginBottom: "0.25rem" }}>
            Detail
          </div>
          <div
            style={{
              background: "rgba(9, 14, 26, 0.75)",
              border: `1px solid ${NOC_SURFACES.border}`,
              borderRadius: "6px",
              padding: "0.6rem 0.85rem",
              fontSize: "0.85rem",
              color: NOC_SURFACES.textSecondary,
              lineHeight: 1.45,
              whiteSpace: "pre-wrap",
              wordBreak: "break-word",
            }}
          >
            {event.detail}
          </div>
        </div>
      )}

      {/* Structured Payload / Metadata */}
      {event.metadata && Object.keys(event.metadata).length > 0 && (
        <div>
          <div style={{ fontSize: "0.75rem", color: NOC_SURFACES.textMuted, marginBottom: "0.25rem" }}>
            Event Payload Metadata
          </div>
          <pre
            className="tabular-nums"
            style={{
              margin: 0,
              padding: "0.65rem 0.85rem",
              background: "rgba(9, 14, 26, 0.9)",
              border: `1px solid ${NOC_SURFACES.border}`,
              borderRadius: "6px",
              fontSize: "0.76rem",
              fontFamily: "monospace",
              color: NOC_SURFACES.textSecondary,
              overflowX: "auto",
              maxHeight: "220px",
            }}
          >
            {JSON.stringify(event.metadata, null, 2)}
          </pre>
        </div>
      )}
    </div>
  );
};
