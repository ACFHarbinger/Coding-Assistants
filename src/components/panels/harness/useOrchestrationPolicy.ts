import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "../../../lib/tauri";
import type { EffectiveSettings } from "../../settings/types";

export interface OrchestrationPolicySnapshot {
  allow_metered_quota_probes: boolean;
  quota_auto_refresh_enabled: boolean;
  quota_auto_refresh_interval_secs: number;
}

const DEFAULT_POLICY: OrchestrationPolicySnapshot = {
  allow_metered_quota_probes: true,
  quota_auto_refresh_enabled: false,
  quota_auto_refresh_interval_secs: 300,
};

export function useOrchestrationPolicy() {
  const [policy, setPolicy] = useState<OrchestrationPolicySnapshot>(DEFAULT_POLICY);
  const [loading, setLoading] = useState(false);
  const refreshInFlight = useRef(false);

  const refreshPolicy = useCallback(async () => {
    if (refreshInFlight.current) return;
    refreshInFlight.current = true;
    try {
      setLoading(true);
      const settings = await invoke<EffectiveSettings>("settings_get_effective", { workspace: null });
      if (settings?.orchestration) {
        setPolicy({
          allow_metered_quota_probes: settings.orchestration.allow_metered_quota_probes,
          quota_auto_refresh_enabled: settings.orchestration.quota_auto_refresh_enabled,
          quota_auto_refresh_interval_secs: settings.orchestration.quota_auto_refresh_interval_secs ?? 300,
        });
      }
    } catch {
      // Degrades silently to defaults on failure
    } finally {
      refreshInFlight.current = false;
      setLoading(false);
    }
  }, []);

  const setAllowMeteredProbes = useCallback(async (enabled: boolean) => {
    // Optimistic update
    setPolicy((prev) => ({ ...prev, allow_metered_quota_probes: enabled }));
    try {
      const updated = await invoke<EffectiveSettings>("settings_update_orchestration", {
        workspace: null,
        patch: { allow_metered_quota_probes: enabled },
      });
      if (updated?.orchestration) {
        setPolicy({
          allow_metered_quota_probes: updated.orchestration.allow_metered_quota_probes,
          quota_auto_refresh_enabled: updated.orchestration.quota_auto_refresh_enabled,
          quota_auto_refresh_interval_secs: updated.orchestration.quota_auto_refresh_interval_secs ?? 300,
        });
      }
    } catch {
      // Revert on failure
      void refreshPolicy();
    }
  }, [refreshPolicy]);

  useEffect(() => {
    void refreshPolicy();
  }, [refreshPolicy]);

  return { policy, setAllowMeteredProbes, refreshPolicy, loading };
}
