import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import TaskTab from "../TaskTab";
import * as tauri from "../../lib/tauri";

vi.mock("../../lib/tauri", () => ({
  invoke: vi.fn(),
}));

describe("TaskTab", () => {
  const mockTasks = [
    {
      id: "task-12345678",
      title: "Run automated migrations",
      workspace_path: "/workspace/project",
      status: "in_progress",
      step_index: 1,
      steps: [
        { name: "Lint and Validate", agent: "gemini" },
        { name: "Apply Schema Changes", agent: "codex" },
        { name: "Run Integration Suite", agent: "claude" },
      ],
      created_at: "2026-10-23T12:00:00Z",
      updated_at: "2026-10-23T12:05:00Z",
      last_message_id: null,
      attempts: {},
      open_agents: ["codex"],
      pending_agents: [],
      max_parallel: 2,
    },
  ];

  const mockMessages = [
    {
      id: "msg-1",
      from_agent: "gemini",
      to_agent: "codex",
      workspace_path: "/workspace/project",
      task_id: "task-12345678",
      kind: "task",
      status: "delivered",
      subject: "Schema migration dispatched",
      body: "Please run migration 004.",
      created_at: "2026-10-23T12:02:00Z",
      acked_at: "2026-10-23T12:02:05Z",
    },
  ];

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders empty state when no tasks are returned", async () => {
    vi.mocked(tauri.invoke).mockResolvedValue([]);
    render(<TaskTab />);

    expect(screen.getByText("Task Browser")).toBeInTheDocument();
    await waitFor(() => {
      expect(screen.getByText("No tasks found in the hub")).toBeInTheDocument();
    });
  });

  it("renders tasks and allows opening detail pipeline stepper", async () => {
    vi.mocked(tauri.invoke).mockImplementation(async (cmd: string) => {
      if (cmd === "hub_list_tasks") return mockTasks;
      if (cmd === "hub_list_messages") return mockMessages;
      return [];
    });

    render(<TaskTab />);

    await waitFor(() => {
      expect(screen.getByText("Run automated migrations")).toBeInTheDocument();
      expect(screen.getByText("IN_PROGRESS")).toBeInTheDocument();
    });

    // Click task to open pipeline view
    fireEvent.click(screen.getByText("Run automated migrations"));

    await waitFor(() => {
      expect(screen.getByText("Workflow Stage Pipeline")).toBeInTheDocument();
      expect(screen.getByText("Lint and Validate")).toBeInTheDocument();
      expect(screen.getByText("Apply Schema Changes")).toBeInTheDocument();
      expect(screen.getByText("Run Integration Suite")).toBeInTheDocument();
      expect(screen.getByText("Schema migration dispatched")).toBeInTheDocument();
      expect(screen.getByText("Please run migration 004.")).toBeInTheDocument();
    });

    // Return back to browser
    fireEvent.click(screen.getByRole("button", { name: "← Back to Tasks" }));
    await waitFor(() => {
      expect(screen.getByText("Task Browser")).toBeInTheDocument();
    });
  });
});
