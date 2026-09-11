import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import HubPanel from "../HubPanel";

const mockInvoke = vi.fn(async (cmd: string) => {
  if (cmd === "get_hub_data_dir") return "/test/hub";
  if (cmd === "hub_list_agents") {
    return [
      { id: "human", display_name: "Human", team_member: true },
      { id: "kimi", display_name: "Kimi", team_member: false },
      { id: "claude", display_name: "Claude Code", team_member: true },
    ];
  }
  if (cmd === "hub_get_budget") return null;
  if (cmd === "hub_list_agent_metrics") return [];
  if (cmd === "hub_list_tasks") return [];
  if (cmd === "hub_list_messages") return [];
  if (cmd === "hub_list_wakes") return [];
  if (cmd === "hub_list_budgets") return [];
  if (cmd === "hub_list_provider_quotas") return [];
  if (cmd === "hub_list_channel_workspaces") return [];
  if (cmd === "hub_list_audit_events") return [];
  return [];
});

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => (mockInvoke as unknown as (...a: unknown[]) => unknown)(...args),
}));

describe("HubPanel persistence synchronization (U22 / #225)", () => {
  beforeEach(() => {
    (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
    mockInvoke.mockClear();
  });

  it("awaits onAddAgent and surfaces visible error on failure", async () => {
    const onAddAgent = vi.fn().mockRejectedValue(new Error("Store write failed: database locked"));
    render(
      <HubPanel
        teamMemberIds={["human", "claude"]}
        onAddAgent={onAddAgent}
        onRemoveAgent={vi.fn()}
      />
    );

    await waitFor(() => expect(screen.getByText("Kimi")).toBeTruthy());

    // Click enroll for kimi
    const enrollBtn = screen.getAllByRole("button", { name: "Enroll" })[0];
    fireEvent.click(enrollBtn);

    // Verify onAddAgent was called
    expect(onAddAgent).toHaveBeenCalledWith(
      expect.objectContaining({ id: "kimi", target_id: "kimi" }),
    );

    // Visible error should be displayed in HubPanel
    expect(await screen.findByText(/Store write failed: database locked/i)).toBeTruthy();

    // HubStore agents list should be refreshed after the failure
    await waitFor(() => {
      const calls = mockInvoke.mock.calls.map(c => c[0]);
      expect(calls.filter(cmd => cmd === "hub_list_agents").length).toBeGreaterThanOrEqual(2);
    });
  });

  it("awaits onRemoveAgent and surfaces visible error on failure", async () => {
    const onRemoveAgent = vi.fn().mockRejectedValue(new Error("Store delete failed: foreign key"));
    render(
      <HubPanel
        teamMemberIds={["claude"]}
        onAddAgent={vi.fn()}
        onRemoveAgent={onRemoveAgent}
      />,
    );

    await waitFor(() => expect(screen.getByText("Claude Code")).toBeTruthy());

    // Click unenroll for claude
    const unenrollBtn = screen.getByRole("button", { name: "Unenroll" });
    fireEvent.click(unenrollBtn);

    // Verify onRemoveAgent was called
    expect(onRemoveAgent).toHaveBeenCalledWith(
      expect.objectContaining({ id: "claude", target_id: "claude" }),
    );

    // Visible error should be displayed
    expect(await screen.findByText(/Store delete failed: foreign key/i)).toBeTruthy();

    // HubStore agents list should be refreshed after the failure
    await waitFor(() => {
      const calls = mockInvoke.mock.calls.map(c => c[0]);
      expect(calls.filter(cmd => cmd === "hub_list_agents").length).toBeGreaterThanOrEqual(2);
    });
  });
});
