import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import ActivityView from "../ActivityView";
import type { ActivityItem } from "../types";

const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const mockAgents = [
  { id: "claude", display_name: "Claude" },
  { id: "gemini", display_name: "Gemini" },
  { id: "grok", display_name: "Grok" },
];

const sampleItems: ActivityItem[] = [
  {
    id: "task-1",
    kind: "task",
    title: "Implement D4 Tool Activity",
    status: "running",
    workspace_path: "/home/user/project",
    started_at: "2026-09-18T10:00:00Z",
    updated_at: "2026-09-18T11:00:00Z",
    agents: ["claude", "gemini"],
    commands: [
      {
        raw: "cargo test -p hub",
        cmdline: ["cargo", "test", "-p", "hub"],
        exe: "/usr/bin/cargo",
        observed_at: "2026-09-18T10:15:00Z",
        attribution: "gemini",
        source: "audit",
      },
    ],
    files: [
      {
        path: "src/components/DashboardPanel.tsx",
        operation: "modified",
        observed_at: "2026-09-18T10:14:00Z",
        content_hash: "abc1234",
        status: "approved",
      },
    ],
    message_count: 3,
    capture_count: 0,
  },
  {
    id: "session-1",
    kind: "work_session",
    title: "Release 1.0 Planning Session",
    status: "active",
    workspace_path: null,
    started_at: "2026-09-18T09:00:00Z",
    updated_at: "2026-09-18T09:30:00Z",
    agents: ["claude", "grok"],
    commands: [
      {
        raw: "git status",
        cmdline: ["git", "status"],
        exe: null,
        observed_at: "2026-09-18T09:10:00Z",
        attribution: null,
        source: "capture",
      },
    ],
    files: [],
    message_count: 12,
    capture_count: 2,
  },
];

describe("ActivityView (D4 / #324)", () => {
  beforeEach(() => {
    (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue(sampleItems);
  });

  it("renders activity items with agents, files and command counters", async () => {
    render(<ActivityView agents={mockAgents} />);

    await waitFor(() => {
      expect(screen.getByText("Implement D4 Tool Activity")).toBeTruthy();
    });

    expect(screen.getByText("Release 1.0 Planning Session")).toBeTruthy();
    expect(screen.getByText("running")).toBeTruthy();
    expect(screen.getByText("active")).toBeTruthy();

    // Summary counters
    expect(screen.getByText("Tasks & Sessions")).toBeTruthy();
    expect(screen.getByText("Files Touched")).toBeTruthy();
    expect(screen.getByText("Commands Run")).toBeTruthy();
  });

  it("expands and collapses command and file details on click", async () => {
    render(<ActivityView agents={mockAgents} />);

    await waitFor(() => {
      expect(screen.getByText("Implement D4 Tool Activity")).toBeTruthy();
    });

    const expandButtons = screen.getAllByRole("button", { name: /View Files & Commands/i });
    expect(expandButtons.length).toBeGreaterThan(0);

    // Expand details for task-1
    fireEvent.click(expandButtons[0]);

    await waitFor(() => {
      expect(screen.getByText("$ cargo test -p hub")).toBeTruthy();
      expect(screen.getByText("src/components/DashboardPanel.tsx")).toBeTruthy();
      expect(screen.getByText("modified")).toBeTruthy();
    });

    // Collapse
    const collapseButton = screen.getByRole("button", { name: /Collapse Details/i });
    fireEvent.click(collapseButton);

    await waitFor(() => {
      expect(screen.queryByText("$ cargo test -p hub")).toBeNull();
    });
  });

  it("triggers query with agent filter when agent select changes", async () => {
    render(<ActivityView agents={mockAgents} />);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("hub_get_activity_view", expect.anything());
    });

    const select = screen.getByLabelText("Agent Filter");
    fireEvent.change(select, { target: { value: "gemini" } });

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("hub_get_activity_view", {
        filter: expect.objectContaining({ agent: "gemini" }),
      });
    });
  });

  it("triggers query with since filter when time range changes", async () => {
    render(<ActivityView agents={mockAgents} />);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalled();
    });

    const select = screen.getByLabelText("Time Range");
    fireEvent.change(select, { target: { value: "24h" } });

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("hub_get_activity_view", {
        filter: expect.objectContaining({
          since: expect.any(String),
        }),
      });
    });
  });

  it("triggers query with kind filter when scope changes", async () => {
    render(<ActivityView agents={mockAgents} />);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalled();
    });

    const select = screen.getByLabelText("Scope");
    fireEvent.change(select, { target: { value: "task" } });

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("hub_get_activity_view", {
        filter: expect.objectContaining({
          kind: "task",
        }),
      });
    });
  });

  it("filters items by freeform search query locally", async () => {
    render(<ActivityView agents={mockAgents} />);

    await waitFor(() => {
      expect(screen.getByText("Implement D4 Tool Activity")).toBeTruthy();
    });

    const searchInput = screen.getByPlaceholderText(/Search title, file, command, agent/i);
    fireEvent.change(searchInput, { target: { value: "Release 1.0" } });

    expect(screen.getByText("Release 1.0 Planning Session")).toBeTruthy();
    expect(screen.queryByText("Implement D4 Tool Activity")).toBeNull();
  });

  it("displays empty state message when no items match", async () => {
    mockInvoke.mockResolvedValue([]);
    render(<ActivityView agents={mockAgents} />);

    await waitFor(() => {
      expect(
        screen.getByText("No tool or workspace activities found matching the selected criteria."),
      ).toBeTruthy();
    });
  });

  it("displays error message on backend invoke failure", async () => {
    mockInvoke.mockRejectedValue("Failed to connect to Hub SQLite");
    render(<ActivityView agents={mockAgents} />);

    await waitFor(() => {
      expect(
        screen.getByText(/Error loading activity: Failed to connect to Hub SQLite/i),
      ).toBeTruthy();
    });
  });
});
