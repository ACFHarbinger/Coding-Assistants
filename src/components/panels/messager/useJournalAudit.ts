import { useEffect, useState } from "react";
import { invoke } from "../../../lib/tauri";
import type { AuditEvent } from "../hub/types";

/** Journal-subtab audit list for the messenger panel. */
export function useJournalAudit() {
  const [auditEvents, setAuditEvents] = useState<AuditEvent[]>([]);
  const [auditShowAll, setAuditShowAll] = useState(false);
  const [auditError, setAuditError] = useState<string>("");
  const [auditLoading, setAuditLoading] = useState(false);
  const [mutatingAuditId, setMutatingAuditId] = useState<string | null>(null);

  const refreshAuditEvents = async () => {
    setAuditLoading(true);
    try {
      const list = await invoke<AuditEvent[]>("hub_list_audit_events", { pendingOnly: !auditShowAll });
      setAuditEvents(list);
      setAuditError("");
    } catch (e) {
      setAuditError(String(e));
    } finally {
      setAuditLoading(false);
    }
  };

  const approveAudit = async (eventId: string) => {
    setMutatingAuditId(eventId);
    try {
      await invoke("hub_approve_audit_event", { eventId, approvedBy: "human" });
      setAuditError("");
    } catch (e) {
      setAuditError(String(e));
    } finally {
      setMutatingAuditId(null);
    }
    await refreshAuditEvents();
  };

  const quarantineAudit = async (eventId: string) => {
    setMutatingAuditId(eventId);
    try {
      await invoke("hub_quarantine_audit_event", { eventId, quarantinedBy: "human", reason: "Rejected via desktop journal" });
      setAuditError("");
    } catch (e) {
      setAuditError(String(e));
    } finally {
      setMutatingAuditId(null);
    }
    await refreshAuditEvents();
  };

  useEffect(() => {
    setAuditLoading(true);
    invoke<AuditEvent[]>("hub_list_audit_events", { pendingOnly: true })
      .then(setAuditEvents)
      .catch((e) => console.error("Failed to load pending audit events:", e))
      .finally(() => setAuditLoading(false));
  }, []);

  return {
    auditEvents,
    auditShowAll,
    setAuditShowAll,
    auditError,
    auditLoading,
    mutatingAuditId,
    refreshAuditEvents,
    approveAudit,
    quarantineAudit,
  };
}
