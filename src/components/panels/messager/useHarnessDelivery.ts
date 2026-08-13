import { useEffect, useState } from "react";
import { invoke, isTauriRuntime } from "../../../lib/tauri";
import type { HarnessDeliveryNotice, HarnessSessionRegistration } from "../harness/types";
import { injectNotice } from "../harness/types";
import type { HarnessInjectResult } from "./types";

export function useHarnessDelivery(workspacePath: string, activeWorkSessionId: string | null) {
  const [harnessSessions, setHarnessSessions] = useState<HarnessSessionRegistration[]>([]);
  const [deliveryNotices, setDeliveryNotices] = useState<HarnessDeliveryNotice[]>([]);

  const refreshHarnessSessions = async () => {
    if (!isTauriRuntime()) return;
    try {
      setHarnessSessions(await invoke<HarnessSessionRegistration[]>("hub_list_harness_sessions"));
    } catch (error) {
      console.error("Failed to list harness sessions:", error);
    }
  };

  useEffect(() => {
    void refreshHarnessSessions();
    const interval = setInterval(() => void refreshHarnessSessions(), 5000);
    return () => clearInterval(interval);
  }, [workspacePath]);

  const retryDelivery = async (notice: HarnessDeliveryNotice) => {
    if (!notice.messageId || !notice.body) return;
    try {
      const result = await invoke<HarnessInjectResult>("hub_inject_harness", {
        harness: notice.harness,
        workspace: workspacePath,
        sessionId: activeWorkSessionId,
        messageId: notice.messageId,
        body: notice.body,
        isTask: notice.isTask ?? false,
        isWake: notice.isWake ?? false,
      });
      const next = injectNotice(result.status, result.detail);
      setDeliveryNotices((current) => current.map((item) => item.harness === notice.harness
        ? { ...item, status: result.status, detail: result.detail, retryable: next.retryable }
        : item));
      await refreshHarnessSessions();
    } catch (error) {
      setDeliveryNotices((current) => current.map((item) => item.harness === notice.harness
        ? { ...item, status: "unavailable", detail: String(error), retryable: true }
        : item));
    }
  };

  return {
    harnessSessions,
    deliveryNotices,
    setDeliveryNotices,
    refreshHarnessSessions,
    retryDelivery,
    dismissDelivery: (harness: string) => setDeliveryNotices((current) => current.filter((item) => item.harness !== harness)),
  };
}
