import { cardStyle } from "../hub/HubCharts";
import type { AuditEvent } from "../hub/types";

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
    <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1.5rem" }}>
      <div style={{ ...cardStyle, display: "flex", alignItems: "center", justifyContent: "space-between", gap: "1rem", flexWrap: "wrap" }}>
        <div>
          <h3 style={{ margin: 0, fontSize: "1rem", fontWeight: 600, color: "var(--text-main)" }}>Pending Audit Events</h3>
          <p style={{ margin: "0.35rem 0 0 0", fontSize: "0.8rem", color: "var(--text-muted)" }}>
            Filesystem changes observed by <code>ca audit watch</code>, awaiting owner review.
          </p>
        </div>
        <div style={{ display: "flex", gap: "0.75rem", alignItems: "center" }}>
          <label style={{ display: "flex", alignItems: "center", gap: "0.4rem", fontSize: "0.85rem", color: "var(--text-muted)", cursor: "pointer" }}>
            <input
              type="checkbox"
              checked={auditShowAll}
              onChange={(e) => setAuditShowAll(e.target.checked)}
            />
            Show all (not just pending)
          </label>
          <button className="btn-secondary" onClick={refreshAuditEvents}>Refresh List</button>
        </div>
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: "1rem", maxHeight: 400, overflowY: "auto", paddingRight: "0.5rem" }}>
        {auditEvents.length === 0 && (
          <div style={{ padding: "3rem", textAlign: "center", background: "rgba(0,0,0,0.2)", borderRadius: "12px", border: "1px dashed var(--border-color)" }}>
            <p style={{ color: "var(--text-muted)", fontSize: "0.95rem", margin: 0 }}>
              {auditShowAll ? "No audit events recorded." : "No pending audit events."}
            </p>
          </div>
        )}
        {auditEvents.map((event) => (
          <div key={event.id} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center", gap: "1rem", flexWrap: "wrap" }}>
            <div style={{ minWidth: 0 }}>
              <div style={{ marginBottom: "0.25rem", fontSize: "0.95rem", fontWeight: 600, color: "var(--primary)", fontFamily: "var(--font-mono)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                {event.path}
              </div>
              <div style={{ fontSize: "0.85rem", color: "var(--text-main)" }}>
                {event.operation} · {event.observed_at}
              </div>
              <div style={{ fontSize: "0.75rem", color: "var(--text-muted)", marginTop: "0.35rem" }}>
                root: {event.root_path}
              </div>
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", flexShrink: 0 }}>
              <span style={{
                fontSize: "0.75rem",
                padding: "0.2rem 0.6rem",
                borderRadius: "20px",
                background: event.status === "pending" ? "rgba(234, 179, 8, 0.15)" : event.status === "quarantined" ? "rgba(239, 68, 68, 0.15)" : "rgba(255,255,255,0.1)",
                color: event.status === "pending" ? "#eab308" : event.status === "quarantined" ? "#ef4444" : "var(--text-muted)"
              }}>
                {event.status}
              </span>
              {event.status === "pending" && (
                <>
                  <button className="btn-primary" onClick={() => approveAudit(event.id)}>Approve</button>
                  <button className="btn-secondary" onClick={() => quarantineAudit(event.id)}>Quarantine</button>
                </>
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
