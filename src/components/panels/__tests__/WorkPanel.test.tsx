import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import WorkPanel from "../WorkPanel";

const mockInvoke = vi.fn(async (cmd: string) => {
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
  if (cmd === "hub_list_project_board") {
    return {
      owner: "ACFHarbinger",
      project_number: 21,
      notice: null,
      columns: [
        {
          name: "Backlog",
          cards: [
            {
              id: "card-1",
              kind: "internal",
              title: "Persistent Board Card",
              status: "Backlog",
              issue_number: 42,
              url: null,
              labels: [],
              github_assignees: [],
              roster_assignees: ["kimi"],
              deadline: null,
              linked_branches: [],
              size: null,
            },
          ],
        },
      ],
    };
  }
  return [];
});

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => (mockInvoke as unknown as (...a: unknown[]) => unknown)(...args),
}));

describe("WorkPanel persistence synchronization (U22 / #225)", () => {
  beforeEach(() => {
    (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
    mockInvoke.mockClear();
  });

  it("awaits onAddAgent and surfaces visible error on failure", async () => {
    const onAddAgent = vi.fn().mockRejectedValue(new Error("Store write failed: database locked"));
    render(
      <WorkPanel
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

    // Visible error should be displayed in WorkPanel
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
      <WorkPanel
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

  it("keeps BoardTab mounted and avoids refetching when switching subtabs and back", async () => {
    render(
      <WorkPanel
        teamMemberIds={["human", "claude"]}
        onAddAgent={vi.fn()}
        onRemoveAgent={vi.fn()}
      />,
    );

    // Initially on Dashboard; Board has not been mounted/fetched yet
    expect(mockInvoke.mock.calls.filter((c) => c[0] === "hub_list_project_board").length).toBe(0);

    // Switch to Board subtab
    fireEvent.click(screen.getByRole("button", { name: "Board" }));
    expect(await screen.findByText("Persistent Board Card")).toBeTruthy();
    expect(mockInvoke.mock.calls.filter((c) => c[0] === "hub_list_project_board").length).toBe(1);

    // Type a draft title in the Board subtab
    const draftInput = screen.getByPlaceholderText("Internal task title…") as HTMLInputElement;
    fireEvent.change(draftInput, { target: { value: "Keep this draft" } });

    // Switch away to Tasks and Dashboard, then back to Board
    fireEvent.click(screen.getByRole("button", { name: "Tasks" }));
    fireEvent.click(screen.getByRole("button", { name: "Dashboard" }));
    fireEvent.click(screen.getByRole("button", { name: "Board" }));

    // Card and draft remain in the DOM without a second hub_list_project_board call
    expect(screen.getByText("Persistent Board Card")).toBeTruthy();
    expect((screen.getByPlaceholderText("Internal task title…") as HTMLInputElement).value).toBe("Keep this draft");
    expect(mockInvoke.mock.calls.filter((c) => c[0] === "hub_list_project_board").length).toBe(1);
  });
});
