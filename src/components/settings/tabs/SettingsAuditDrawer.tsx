import { useState } from "react";
import type { SettingsAuditEvent } from "../types";
import { NOC_COLORS, NOC_SURFACES } from "../../common/telemetry/tokens";
import { nocSubsurfaceStyle } from "../../common/telemetry/panel";

export interface SettingsAuditDrawerProps {
  auditEvents: SettingsAuditEvent[];
}

export default function SettingsAuditDrawer({ auditEvents }: SettingsAuditDrawerProps) {
  const [showAudit, setShowAudit] = useState(false);

  return (
    <div style={{ marginTop: "0.25rem" }}>
      <button
        type="button"
        className="btn-secondary noc-focusable"
        style={{ marginTop: 0, padding: "0.4rem 0.85rem", fontSize: "0.78rem" }}
        onClick={() => setShowAudit((value) => !value)}
      >
        {showAudit ? "Hide" : "Show"} recent settings changes ({auditEvents.length})
      </button>
      {showAudit && (
        <div
          style={{
            ...nocSubsurfaceStyle,
            marginTop: "0.6rem",
            maxHeight: "160px",
            overflowY: "auto",
            padding: "0.5rem 0.75rem",
          }}
        >
          {auditEvents.length === 0 && (
            <p style={{ color: NOC_SURFACES.textMuted, fontSize: "0.8rem", margin: 0 }}>No settings changes recorded yet.</p>
          )}
          {auditEvents
            .slice()
            .reverse()
            .map((event) => (
              <div
                key={event.id}
                style={{
                  fontSize: "0.78rem",
                  color: NOC_SURFACES.textMuted,
                  padding: "0.35rem 0",
                  borderBottom: `1px solid ${NOC_SURFACES.border}`,
                  display: "flex",
                  justifyContent: "space-between",
                  gap: "0.75rem",
                  flexWrap: "wrap",
                }}
              >
                <span>
                  <strong style={{ color: NOC_COLORS.cyan, marginRight: "0.4rem" }}>{event.operation}</strong>
                  <code style={{ color: NOC_SURFACES.textPrimary, fontSize: "0.75rem" }}>{event.path}</code>
                </span>
                <span className="tabular-nums" style={{ fontSize: "0.72rem", color: NOC_SURFACES.textMuted }}>
                  {event.observed_at}
                </span>
              </div>
            ))}
        </div>
      )}
    </div>
  );
}
