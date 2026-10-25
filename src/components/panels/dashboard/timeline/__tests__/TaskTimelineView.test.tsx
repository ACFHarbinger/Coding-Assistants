import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import TaskTimelineView from "../TaskTimelineView";
import type { TaskRecord, TaskTimelineView as ITaskTimelineView } from "../types";

const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const sampleTasks: TaskRecord[] = [
  {
    id: "task-42",
    title: "Implement D1 Timeline Visualization",
    status: "in_progress",
    updated_at: "2026-10-25T14:30:00Z",
  },
  {
    id: "task-99",
    title: "Bugfix Memory Leak",
    status: "completed",
    updated_at: "2026-10-25T15:00:00Z",
  },
];

const mockTimelineData: ITaskTimelineView = {
  task: sampleTasks[0],
  duration_seconds: 185.5,
  status: "in_progress",
  failure_reason: null,
  is_paused: false,
  involved_agents: ["orchestrator", "gemini", "claude"],
  events: [
    {
      id: "evt-1",
      task_id: "task-42",
      timestamp: "2026-10-25T14:00:00Z",
      event_type: "task_created",
      actor: "orchestrator",
      target: null,
      summary: "Task created: Implement D1 Timeline Visualization",
      detail: "Initialized workflow with 3 steps",
      status: "completed",
      step_index: 0,
      metadata: null,
    },
    {
      id: "evt-2",
      task_id: "task-42",
      timestamp: "2026-10-25T14:01:00Z",
      event_type: "handoff",
      actor: "orchestrator",
      target: "gemini",
      summary: "Handoff to gemini: design timeline components",
      detail: "Transferred context and specs",
      status: "completed",
      step_index: 1,
      metadata: { context_tokens: 1540 },
    },
    {
      id: "evt-3",
      task_id: "task-42",
      timestamp: "2026-10-25T14:02:00Z",
      event_type: "tool",
      actor: "gemini",
      target: null,
      summary: "Execute write_file for TimelineLaneChart.tsx",
      detail: "Generated 390 lines of React/TypeScript code",
      status: "completed",
      step_index: 2,
      metadata: { tool: "write_file", path: "src/TimelineLaneChart.tsx" },
    },
    {
      id: "evt-4",
      task_id: "task-42",
      timestamp: "2026-10-25T14:03:00Z",
      event_type: "gate",
      actor: "gemini",
      target: "claude",
      summary: "Gate review requested before test execution",
      detail: "Awaiting human or lead review sign-off",
      status: "paused",
      step_index: 3,
      metadata: { requires_human: true },
    },
  ],
};

describe("TaskTimelineView (Roadmap D1 / #87)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "hub_get_task_timeline") {
        return mockTimelineData;
      }
      return null;
    });
  });

  it("renders empty state message when no tasks are provided", () => {
    render(<TaskTimelineView tasks={[]} />);
    expect(
      screen.getByText(/No tasks exist in the shared hub yet/i),
    ).toBeTruthy();
  });

  it("loads timeline data on mount for the initial task", async () => {
    render(<TaskTimelineView tasks={sampleTasks} />);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("hub_get_task_timeline", {
        taskId: "task-42",
      });
    });

    // Check KPI metrics rendered
    expect(screen.getAllByText(/in_progress/i).length).toBeGreaterThan(0);
    expect(screen.getByText("3m 6s")).toBeTruthy(); // 185.5s formatted
    expect(screen.getByText("4")).toBeTruthy(); // 4 total events
    expect(screen.getByText("3")).toBeTruthy(); // 3 agents
  });

  it("renders 2D swimlanes with agent rows and event chips", async () => {
    render(<TaskTimelineView tasks={sampleTasks} />);

    await waitFor(() => {
      expect(screen.getByRole("region", { name: /2D Task Timeline Swimlane Visualization/i })).toBeTruthy();
    });

    // Agents should have swimlane headers
    expect(screen.getByTitle("orchestrator")).toBeTruthy();
    expect(screen.getByTitle("gemini")).toBeTruthy();
    expect(screen.getByTitle("claude")).toBeTruthy();

    // Event chips
    const swimlane = screen.getByRole("region", { name: /2D Task Timeline Swimlane Visualization/i });
    expect(swimlane.textContent).toMatch(/handoff/i);
    expect(swimlane.textContent).toMatch(/tool/i);
    expect(swimlane.textContent).toMatch(/gate/i);
  });

  it("switches to chronological feed view and lists event articles", async () => {
    render(<TaskTimelineView tasks={sampleTasks} />);

    await waitFor(() => {
      expect(screen.getByText("2D Swimlanes")).toBeTruthy();
    });

    const feedBtn = screen.getByRole("button", { name: /Chronological Feed/i });
    fireEvent.click(feedBtn);

    expect(screen.getByRole("feed", { name: /Chronological task event feed/i })).toBeTruthy();
    const articles = screen.getAllByRole("article");
    expect(articles.length).toBe(4);
    expect(screen.getByText("Execute write_file for TimelineLaneChart.tsx")).toBeTruthy();
  });

  it("filters events by event type and search query", async () => {
    render(<TaskTimelineView tasks={sampleTasks} />);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Chronological Feed/i })).toBeTruthy();
    });

    fireEvent.click(screen.getByRole("button", { name: /Chronological Feed/i }));

    // Filter by event type "tool"
    const typeSelect = screen.getByLabelText(/Filter by event type/i);
    fireEvent.change(typeSelect, { target: { value: "tool" } });

    let articles = screen.getAllByRole("article");
    expect(articles.length).toBe(1);
    expect(screen.getByText("Execute write_file for TimelineLaneChart.tsx")).toBeTruthy();

    // Reset type filter and use search query
    fireEvent.change(typeSelect, { target: { value: "all" } });
    const searchInput = screen.getByLabelText(/Search events/i);
    fireEvent.change(searchInput, { target: { value: "Gate review" } });

    articles = screen.getAllByRole("article");
    expect(articles.length).toBe(1);
    expect(screen.getByText("Gate review requested before test execution")).toBeTruthy();
  });

  it("opens detail drawer when an event is clicked and displays metadata", async () => {
    render(<TaskTimelineView tasks={sampleTasks} />);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Chronological Feed/i })).toBeTruthy();
    });

    fireEvent.click(screen.getByRole("button", { name: /Chronological Feed/i }));

    const articles = screen.getAllByRole("article");
    fireEvent.click(articles[1]); // The handoff event

    await waitFor(() => {
      expect(screen.getByRole("region", { name: /Event detail drawer/i })).toBeTruthy();
    });

    const drawer = screen.getByRole("region", { name: /Event detail drawer/i });
    expect(drawer).toBeTruthy();
    expect(drawer.textContent).toContain("Handoff to gemini: design timeline components");
    expect(drawer.textContent).toContain("Transferred context and specs");
    expect(drawer.textContent).toContain("context_tokens");

    // Close drawer
    const closeBtn = screen.getByRole("button", { name: /Close event details/i });
    fireEvent.click(closeBtn);
    expect(screen.queryByRole("region", { name: /Event detail drawer/i })).toBeNull();
  });

  it("highlights paused and failed status callouts", async () => {
    const pausedOrFailedTimeline: ITaskTimelineView = {
      ...mockTimelineData,
      status: "failed",
      is_paused: true,
      failure_reason: "Step 3 timed out waiting for human gate approval",
      events: [
        ...mockTimelineData.events,
        {
          id: "evt-failed",
          task_id: "task-42",
          timestamp: "2026-10-25T14:04:00Z",
          event_type: "transition",
          actor: "claude",
          target: null,
          summary: "Execution timed out",
          detail: "Gate timeout exceeded 600s",
          status: "failed",
          step_index: 4,
          metadata: null,
        },
      ],
    };

    mockInvoke.mockResolvedValueOnce(pausedOrFailedTimeline);

    render(<TaskTimelineView tasks={sampleTasks} />);

    await waitFor(() => {
      expect(screen.getByText(/Task Execution Paused/i)).toBeTruthy();
      expect(screen.getByText(/Step 3 timed out waiting for human gate approval/i)).toBeTruthy();
    });
  });
});
