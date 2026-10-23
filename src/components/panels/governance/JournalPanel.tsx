import { nocPanelStyle, nocCardDenseStyle } from "../../common/telemetry/panel";
import { NOC_COLORS, NOC_SURFACES } from "../../common/telemetry/tokens";
import { EmptyStateChip } from "../../common/telemetry/status";
import type { AuditEvent } from "../hub/types";

function getCategoryPill(operation: string) {
  const op = operation.toLowerCase();
  if (op.startsWith("os_tool") || op.startsWith("fs")) {
    return { label: "os_tool", bg: "rgba(245, 158, 11, 0.15)", color: NOC_COLORS.warn, border: "rgba(245, 158, 11, 0.35)" };
  }
  if (op.startsWith("budget")) {
    return { label: "budget", bg: "rgba(56, 189, 248, 0.15)", color: NOC_COLORS.cyan, border: "rgba(56, 189, 248, 0.35)" };
  }
  if (op.startsWith("task")) {
    return { label: "task", bg: "rgba(99, 102, 241, 0.15)", color: NOC_COLORS.indigo, border: "rgba(99, 102, 241, 0.35)" };
  }
  if (op.startsWith("oversight")) {
    return { label: "oversight", bg: "rgba(168, 85, 247, 0.15)", color: NOC_COLORS.purple, border: "rgba(168, 85, 247, 0.35)" };
  }
  return { label: "audit", bg: "rgba(148, 163, 184, 0.15)", color: NOC_COLORS.idle, border: "rgba(148, 163, 184, 0.35)" };
}

export default function JournalPanel({
  auditEvents,
  auditShowAll,
  setAuditShowAll,
  refreshAuditEvents,
  approveAudit,
  quarantineAudit,
}: {
  auditEvents: AuditEvent[];
  auditShowAll: boolean;
  setAuditShowAll: (next: boolean) => void;
  refreshAuditEvents: () => void;
  approveAudit: (eventId: string) => void;
  quarantineAudit: (eventId: string) => void;
}) {
  return (
    <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1.25rem" }}>
      {/* Header bar */}
      <div style={{ ...nocPanelStyle, display: "flex", alignItems: "center", justifyContent: "space-between", gap: "1rem", flexWrap: "wrap" }}>
        <div>
          <h3 style={{ margin: 0, fontSize: "1.05rem", fontWeight: 700, color: NOC_SURFACES.textPrimary }}>Pending Audit Events</h3>
          <p style={{ margin: "0.25rem 0 0", fontSize: "0.82rem", color: NOC_SURFACES.textMuted }}>
            Filesystem and tool operations observed by <code>ca audit watch</code>, awaiting owner review.
          </p>
        </div>
        <div style={{ display: "flex", gap: "0.75rem", alignItems: "center" }}>
          <label style={{ display: "flex", alignItems: "center", gap: "0.4rem", fontSize: "0.85rem", color: NOC_SURFACES.textMuted, cursor: "pointer" }}>
            <input
              type="checkbox"
              checked={auditShowAll}
              onChange={(e) => setAuditShowAll(e.target.checked)}
              style={{ accentColor: NOC_COLORS.cyan }}
            />
            <span>Show all (not just pending)</span>
          </label>
          <button className="btn-secondary noc-focusable" onClick={refreshAuditEvents}>
            Refresh List
          </button>
        </div>
      </div>

      {/* Stream list */}
      <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem", maxHeight: 440, overflowY: "auto", paddingRight: "0.25rem" }}>
        {auditEvents.length === 0 && (
          <div style={{ display: "flex", justifyContent: "center", padding: "2.5rem 1rem", width: "100%" }}>
            <EmptyStateChip title={auditShowAll ? "No audit events recorded" : "No pending audit events"} />
          </div>
        )}
        {auditEvents.map((event) => {
          const pill = getCategoryPill(event.operation);
          const hashSnippet = event.event_hash ? event.event_hash.slice(0, 10) : event.id.slice(0, 8);
          return (
            <div
              key={event.id}
              style={{
                ...nocCardDenseStyle,
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                gap: "1rem",
                flexWrap: "wrap",
                borderLeft: `3px solid ${event.status === "pending" ? NOC_COLORS.warn : event.status === "quarantined" ? NOC_COLORS.crit : NOC_COLORS.ok}`,
              }}
            >
              <div style={{ minWidth: 0, flex: 1 }}>
                <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", flexWrap: "wrap", marginBottom: "0.3rem" }}>
                  <span
                    style={{
                      fontSize: "0.68rem",
                      fontWeight: 700,
                      padding: "0.1rem 0.42rem",
                      borderRadius: "4px",
                      background: pill.bg,
                      color: pill.color,
                      border: `1px solid ${pill.border}`,
                      textTransform: "uppercase",
                      letterSpacing: "0.04em",
                    }}
                  >
                    {pill.label}
                  </span>
                  <span style={{ fontSize: "0.82rem", color: NOC_SURFACES.textPrimary, fontWeight: 600 }}>
                    {event.operation}
                  </span>
                  <code style={{ fontSize: "0.72rem", color: NOC_COLORS.cyan, background: "rgba(56, 189, 248, 0.1)", padding: "0.1rem 0.35rem", borderRadius: "4px" }}>
                    {hashSnippet}
                  </code>
                </div>

                <div style={{ marginBottom: "0.2rem", fontSize: "0.88rem", fontWeight: 600, color: NOC_SURFACES.textPrimary, fontFamily: "var(--font-mono)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  {event.path}
                </div>

                <div className="tabular-nums" style={{ fontSize: "0.74rem", color: NOC_SURFACES.textMuted }}>
                  root: {event.root_path} · {event.observed_at}
                </div>
              </div>

              <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", flexShrink: 0 }}>
                <span
                  style={{
                    fontSize: "0.74rem",
                    fontWeight: 600,
                    padding: "0.22rem 0.65rem",
                    borderRadius: "9999px",
                    background: event.status === "pending" ? "rgba(234, 179, 8, 0.15)" : event.status === "quarantined" ? "rgba(239, 68, 68, 0.15)" : "rgba(16, 185, 129, 0.15)",
                    color: event.status === "pending" ? NOC_COLORS.warn : event.status === "quarantined" ? NOC_COLORS.crit : NOC_COLORS.ok,
                    border: `1px solid ${event.status === "pending" ? "rgba(234, 179, 8, 0.3)" : event.status === "quarantined" ? "rgba(239, 68, 68, 0.3)" : "rgba(16, 185, 129, 0.3)"}`,
                    textTransform: "capitalize",
                  }}
                >
                  {event.status}
                </span>
                {event.status === "pending" && (
                  <>
                    <button className="btn-primary noc-focusable" onClick={() => approveAudit(event.id)}>
                      Approve
                    </button>
                    <button className="btn-secondary noc-focusable" onClick={() => quarantineAudit(event.id)}>
                      Quarantine
                    </button>
                  </>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
