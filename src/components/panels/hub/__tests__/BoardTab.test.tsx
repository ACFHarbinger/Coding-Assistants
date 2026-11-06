import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import BoardTab from "../BoardTab";

vi.mock("../../../../lib/tauri", () => ({
  invoke: vi.fn().mockImplementation((cmd: string) => {
    if (cmd === "hub_list_agents") {
      return Promise.resolve([{ id: "grok", display_name: "Grok" }]);
    }
    if (cmd === "hub_list_project_board") {
      return Promise.resolve({
        owner: "ACFHarbinger",
        project_number: 21,
        notice: "GitHub project unavailable (offline). Showing Hub-only cards.",
        columns: [
          {
            name: "Ready",
            cards: [
              {
                id: "internal-1",
                kind: "internal",
                title: "Spike Kanban",
                status: "Ready",
                issue_number: null,
                url: null,
                labels: [],
                github_assignees: [],
                roster_assignees: ["grok"],
                deadline: "2026-09-20",
                linked_branches: [],
                size: null,
              },
            ],
          },
        ],
      });
    }
    return Promise.resolve(null);
  }),
  isTauriRuntime: vi.fn().mockReturnValue(true),
}));

describe("BoardTab", () => {
  it("renders cached-or-hub columns, KPI telemetry strip, and the gh-fail notice", async () => {
    render(<BoardTab />);
    await waitFor(() => {
      expect(screen.getByText("Spike Kanban")).toBeTruthy();
    });
    expect(screen.getAllByText("Ready").length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText("Total Cards")).toBeTruthy();
    expect(screen.getByText("Column Distribution")).toBeTruthy();
    expect(screen.getByText(/GitHub project unavailable/)).toBeTruthy();

    // Filter cards by query
    const filterInput = screen.getByPlaceholderText(/Filter cards/i);
    fireEvent.change(filterInput, { target: { value: "nonexistent" } });
    expect(screen.getByText("No matching cards")).toBeTruthy();
  });
});
