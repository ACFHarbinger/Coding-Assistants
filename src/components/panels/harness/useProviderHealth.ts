import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "../../../lib/tauri";
import type { ProviderHealth } from "./healthTypes";

export function indexHealthList(list: ProviderHealth[]): Record<string, ProviderHealth> {
  const map: Record<string, ProviderHealth> = {};
  for (const item of list) {
    const rawId = item.agentId || (item as unknown as { agent_id?: string }).agent_id;
    if (rawId) {
      const id = rawId.toLowerCase();
      map[id] = item;
      if (id === "chat") map["codex"] = item;
      if (id === "codex") map["chat"] = item;
      if (id === "gemini") map["agy"] = item;
      if (id === "agy") map["gemini"] = item;
    }
  }
  return map;
}

export function useProviderHealth(pollIntervalMs = 30_000) {
  const [healthMap, setHealthMap] = useState<Record<string, ProviderHealth>>({});
  const [loading, setLoading] = useState(false);
  const refreshInFlight = useRef(false);

  const refresh = useCallback(async () => {
    if (refreshInFlight.current) return;
    refreshInFlight.current = true;
    try {
      setLoading(true);
      const list = await invoke<ProviderHealth[]>("hub_get_provider_health");
      if (Array.isArray(list)) {
        setHealthMap(indexHealthList(list));
      }
    } catch {
      // Degrades silently to previous or empty state; never throws or crashes UI.
    } finally {
      refreshInFlight.current = false;
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    let disposed = false;
    void refresh();
    const interval = window.setInterval(() => {
      if (!disposed) {
        void refresh();
      }
    }, pollIntervalMs);

    return () => {
      disposed = true;
      window.clearInterval(interval);
    };
  }, [refresh, pollIntervalMs]);

  return { healthMap, refresh, loading };
}
