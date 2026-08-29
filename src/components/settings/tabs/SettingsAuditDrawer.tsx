import { useState } from "react";
import type { SettingsAuditEvent } from "../types";

export interface SettingsAuditDrawerProps {
  auditEvents: SettingsAuditEvent[];
}

export default function SettingsAuditDrawer({ auditEvents }: SettingsAuditDrawerProps) {
  const [showAudit, setShowAudit] = useState(false);

  return (
    <div>
      <button
        type="button"
        className="btn-secondary"
        style={{ marginTop: 0, padding: "0.4rem 0.8rem", fontSize: "0.78rem" }}
        onClick={() => setShowAudit((value) => !value)}
      >
        {showAudit ? "Hide" : "Show"} recent settings changes ({auditEvents.length})
      </button>
      {showAudit && (
        <div
          style={{
            marginTop: "0.6rem",
            maxHeight: "160px",
            overflowY: "auto",
            border: "1px solid var(--border-color)",
            borderRadius: "8px",
            padding: "0.5rem 0.75rem",
          }}
        >
          {auditEvents.length === 0 && (
            <p style={{ color: "var(--text-muted)", fontSize: "0.8rem" }}>No settings changes recorded yet.</p>
          )}
          {auditEvents
            .slice()
            .reverse()
            .map((event) => (
              <div
                key={event.id}
                style={{
                  fontSize: "0.78rem",
                  color: "var(--text-muted)",
                  padding: "0.25rem 0",
                  borderBottom: "1px solid var(--border-color)",
                }}
              >
                <span style={{ color: "var(--text-main)" }}>{event.operation}</span> {event.path} — {event.observed_at}
              </div>
            ))}
        </div>
      )}
    </div>
  );
}
