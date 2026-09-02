import { beforeEach, describe, expect, it } from "vitest";
import {
  DEFAULT_ROLES,
  defaultMcpConfig,
  loadPersistedRoles,
  ROLES_STORAGE_KEY,
  savePersistedRoles,
} from "../rolesConfig";
import type { RoleConfig } from "../../components/panels/config/types";

describe("rolesConfig (#241 QA-4, #242 QA-5)", () => {
  const store: Record<string, string> = {};

  beforeEach(() => {
    for (const key of Object.keys(store)) {
      delete store[key];
    }
    globalThis.localStorage = {
      getItem: (key: string) => store[key] ?? null,
      setItem: (key: string, val: string) => {
        store[key] = String(val);
      },
      removeItem: (key: string) => {
        delete store[key];
      },
      clear: () => {
        for (const key of Object.keys(store)) {
          delete store[key];
        }
      },
      key: (index: number) => Object.keys(store)[index] ?? null,
      get length() {
        return Object.keys(store).length;
      },
    };
  });

  it("loads DEFAULT_ROLES when nothing is stored", () => {
    expect(loadPersistedRoles()).toEqual(DEFAULT_ROLES);
  });

  it("persists and reloads custom roles from localStorage", () => {
    const customRoles: RoleConfig[] = [
      { name: "Architect", config: { provider: "anthropic", model: "claude-3-7-sonnet" } },
      { name: "Tester", config: { provider: "openai", model: "gpt-4o" } },
    ];

    savePersistedRoles(customRoles);
    expect(localStorage.getItem(ROLES_STORAGE_KEY)).toBe(JSON.stringify(customRoles));
    expect(loadPersistedRoles()).toEqual(customRoles);
  });

  it("generates default neutral MCP config with active workspace root", () => {
    const configStr = defaultMcpConfig("/tmp/test-workspace");
    const parsed = JSON.parse(configStr);
    expect(parsed.mcpServers.filesystem.args).toEqual([
      "-y",
      "@modelcontextprotocol/server-filesystem",
      "/tmp/test-workspace",
    ]);
    expect(configStr).not.toContain("/home/pkhunter/Repositories/Coding-Assistants");
  });
});
