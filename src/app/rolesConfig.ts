import type { RoleConfig } from "../components/panels/config/types";
import { invoke, isTauriRuntime } from "../lib/tauri";

export const DEFAULT_ROLES: RoleConfig[] = [
  { name: "Planner", config: { provider: "openai", model: "gpt-4o" } },
  { name: "Developer", config: { provider: "openai", model: "gpt-4o-mini" } },
  { name: "Reviewer", config: { provider: "openai", model: "gpt-4o" } },
];

export const ROLES_STORAGE_KEY = "ca.orchestrateRoles";

export function loadPersistedRoles(): RoleConfig[] {
  try {
    const raw = localStorage.getItem(ROLES_STORAGE_KEY);
    if (!raw) return DEFAULT_ROLES;
    const parsed = JSON.parse(raw);
    if (Array.isArray(parsed) && parsed.length > 0) {
      return parsed;
    }
    return DEFAULT_ROLES;
  } catch {
    return DEFAULT_ROLES;
  }
}

export function savePersistedRoles(roles: RoleConfig[]) {
  try {
    localStorage.setItem(ROLES_STORAGE_KEY, JSON.stringify(roles));
  } catch {
    /* ignore storage failures */
  }

  if (isTauriRuntime()) {
    for (const role of roles) {
      const roleId = role.name.toLowerCase().replace(/[^a-z0-9_-]/g, "-");
      invoke("hub_upsert_role", {
        args: {
          id: roleId,
          displayName: role.name,
          dailyUngatedQuota: null,
          maxBroadcastRecipients: null,
          canArchiveMessages: false,
          canUpdateAgentRoles: false,
          canAllocateTasks: true,
          responsibilities: [role.config.provider, role.config.model],
        },
      }).catch(() => {});
      if (role.config.provider) {
        invoke("hub_set_role_provider_default", {
          roleId,
          provider: role.config.provider,
        }).catch(() => {});
      }
    }
  }
}

export function defaultMcpConfig(workDir: string): string {
  const ws = workDir && workDir !== "./workspace" ? workDir : ".";
  return JSON.stringify(
    {
      mcpServers: {
        "sequential-thinking": {
          command: "npx",
          args: ["-y", "@modelcontextprotocol/server-sequential-thinking"],
          env: {},
        },
        filesystem: {
          command: "npx",
          args: ["-y", "@modelcontextprotocol/server-filesystem", ws],
          disabledTools: ["read_file"],
        },
        memory: {
          command: "npx",
          args: ["-y", "@modelcontextprotocol/server-memory"],
        },
      },
    },
    null,
    2,
  );
}
