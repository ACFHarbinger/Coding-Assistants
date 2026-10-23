import { useEffect, useState } from "react";
import { invoke } from "../../../lib/tauri";
import type { AuditEvent } from "../hub/types";

/** Journal-subtab audit list for the messenger panel. */
export function useJournalAudit() {
  const [auditEvents, setAuditEvents] = useState<AuditEvent[]>([]);
  const [auditShowAll, setAuditShowAll] = useState(false);
  const [auditError, setAuditError] = useState<string>("");

  const refreshAuditEvents = async () => {
    try {
      const list = await invoke<AuditEvent[]>("hub_list_audit_events", { pendingOnly: !auditShowAll });
      setAuditEvents(list);
      setAuditError("");
    } catch (e) {
      setAuditError(String(e));
    }
  };

  const approveAudit = async (eventId: string) => {
    try {
      await invoke("hub_approve_audit_event", { eventId, approvedBy: "human" });
      setAuditError("");
    } catch (e) {
      setAuditError(String(e));
    }
    await refreshAuditEvents();
  };

  const quarantineAudit = async (eventId: string) => {
    try {
      await invoke("hub_quarantine_audit_event", { eventId, quarantinedBy: "human", reason: "Rejected via desktop journal" });
      setAuditError("");
    } catch (e) {
      setAuditError(String(e));
    }
    await refreshAuditEvents();
  };

  useEffect(() => {
    invoke<AuditEvent[]>("hub_list_audit_events", { pendingOnly: true })
      .then(setAuditEvents)
      .catch((e) => console.error("Failed to load pending audit events:", e));
  }, []);

  return {
    auditEvents,
    auditShowAll,
    setAuditShowAll,
    auditError,
    refreshAuditEvents,
    approveAudit,
    quarantineAudit,
  };
}
