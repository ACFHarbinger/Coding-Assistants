import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import OperationsPanel from "../OperationsPanel";

const mockInvoke = vi.fn(async (cmd: string, args?: Record<string, unknown>) => {
  if (cmd === "hub_list_agents") {
    return [
      { id: "human", display_name: "Human", team_member: true },
      { id: "claude", display_name: "Claude Code", team_member: true },
    ];
  }
  if (cmd === "system_snapshot") {
    return {
      fetched_at: 0,
      cpu_total_percent: 0,
      cpus: [],
      memory: { total_bytes: 0, used_bytes: 0, available_bytes: 0 },
      swap: { total_bytes: 0, used_bytes: 0 },
      disks: [],
      gpu: { available: false, detail: null, gpus: [] },
      processes: [],
    };
  }
  if (cmd === "hub_mcp_client_servers") return [];
  if (cmd === "hub_get_budget") return null;
  if (cmd === "hub_get_provider_quotas") return [];
  if (cmd === "hub_set_agent_budget") return null;
  if (cmd === "hub_record_budget_usage") return null;
  if (cmd === "hub_resume_agent") return null;
  if (cmd === "hub_list_wakes") return [];
  if (cmd === "hub_request_wake") return null;
  void args;
  return [];
});

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => (mockInvoke as unknown as (...a: unknown[]) => unknown)(...args),
}));

describe("OperationsPanel (System, Usage, Tools, Wakes subtabs)", () => {
  beforeEach(() => {
    (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
    mockInvoke.mockClear();
  });

  it("defaults to the System subtab and loads agents on mount", async () => {
    render(<OperationsPanel />);
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("hub_list_agents", undefined);
    });
  });

  it("sets a budget from the Usage subtab", async () => {
    render(<OperationsPanel />);
    await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith("hub_list_agents", undefined));

    fireEvent.click(screen.getByRole("button", { name: "Usage" }));
    await waitFor(() => expect(screen.getByText("Claude Code")).toBeTruthy());

    fireEvent.change(screen.getByDisplayValue("Select agent"), { target: { value: "claude" } });
    fireEvent.click(screen.getByRole("button", { name: "Set / reset budget" }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("hub_set_agent_budget", { agent: "claude", limit: 100 });
    });
  });

  it("requests a wake from the Wakes subtab", async () => {
    render(<OperationsPanel />);
    await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith("hub_list_agents", undefined));

    fireEvent.click(screen.getByRole("button", { name: "Wakes" }));
    await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith("hub_list_wakes", undefined));

    fireEvent.click(screen.getByRole("button", { name: "Wake (human gate)" }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("hub_request_wake", {
        target: "claude",
        reason: null,
        messageId: null,
        humanGate: true,
      });
    });
  });

  it("switches to the Tools subtab", async () => {
    render(<OperationsPanel />);
    fireEvent.click(screen.getByRole("button", { name: "Tools" }));
    await waitFor(() => {
      expect(screen.getByText("MCP tools")).toBeTruthy();
    });
  });
});
