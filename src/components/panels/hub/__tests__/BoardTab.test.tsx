import { describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
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
  it("renders cached-or-hub columns and the gh-fail notice", async () => {
    render(<BoardTab />);
    await waitFor(() => {
      expect(screen.getByText("Spike Kanban")).toBeTruthy();
    });
    expect(screen.getByText("Ready")).toBeTruthy();
    expect(screen.getByText(/GitHub project unavailable/)).toBeTruthy();
  });
});
