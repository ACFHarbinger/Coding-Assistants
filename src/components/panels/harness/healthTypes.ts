/**
 * Provider health types and evaluation logic (#291, #292).
 * Mirrors Rust `ProviderHealth` in `commands/health/health.rs`.
 */

export interface ProviderHealth {
  agentId: string;
  provider: string;
  harnessTitle: string;
  installed: boolean;
  authenticated?: boolean | null;
  authExpiresAt?: string | null;
  endpointReachable?: boolean | null;
  detail: string;
  checkedAt: string;
}

export type HealthTone = "ok" | "warn" | "bad" | "unknown";

export interface HealthStatusInfo {
  tone: HealthTone;
  label: string;
  dotColor: string;
  detail: string;
}

export function evaluateProviderHealth(health: ProviderHealth | null | undefined): HealthStatusInfo {
  if (!health) {
    return {
      tone: "unknown",
      label: "unknown",
      dotColor: "#64748b",
      detail: "Health probe pending or unavailable",
    };
  }

  if (!health.installed) {
    return {
      tone: "bad",
      label: "not installed",
      dotColor: "#ef4444",
      detail: health.detail || "CLI not found on PATH",
    };
  }

  if (health.authenticated === false) {
    return {
      tone: "warn",
      label: "needs login",
      dotColor: "#f59e0b",
      detail: health.detail || "Not authenticated",
    };
  }

  if (health.authExpiresAt) {
    const expireMs = new Date(health.authExpiresAt).getTime();
    if (!isNaN(expireMs)) {
      const remainingMs = expireMs - Date.now();
      if (remainingMs <= 0) {
        return {
          tone: "warn",
          label: "auth expired",
          dotColor: "#f59e0b",
          detail: `Credential expired: ${health.detail}`,
        };
      }
      // If expiring within 48 hours
      if (remainingMs < 48 * 3600 * 1000) {
        const hoursLeft = Math.max(1, Math.round(remainingMs / (3600 * 1000)));
        return {
          tone: "warn",
          label: `expires in ${hoursLeft}h`,
          dotColor: "#f59e0b",
          detail: `Credential expires soon (${health.authExpiresAt}): ${health.detail}`,
        };
      }
    }
  }

  if (health.authenticated === true) {
    return {
      tone: "ok",
      label: "ready",
      dotColor: "#22c55e",
      detail: health.detail || "Ready and authenticated",
    };
  }

  return {
    tone: "ok",
    label: "installed",
    dotColor: "#22c55e",
    detail: health.detail || "Installed",
  };
}
