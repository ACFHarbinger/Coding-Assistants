import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "../../../../lib/tauri";
import { SchedulePanel } from "../SchedulePanel";
import type { ScheduledTask, TickSummary } from "../types";

vi.mock("../../../../lib/tauri", () => ({
  invoke: vi.fn(),
  isTauriRuntime: vi.fn(() => true),
}));

const mockTask: ScheduledTask = {
  id: "task-1",
  name: "Nightly Integration Pass",
  instruction: "Run full unit tests and hygiene linting",
  target: { kind: "team" },
  workspace: "/home/user/repo",
  fire_at: "2026-10-20T04:00:00Z",
  repeat_interval_secs: 3600,
  response_timeout_secs: 3600,
  gate_criteria: [
    {
      kind: "command",
      program: "cargo",
      args: ["test"],
      check: "exit_code_zero",
    },
  ],
  gate_combinator: "all",
  budget: {
    credit_allowance: 100,
    credit_spent: 0,
    stop_on_quota_exhausted: true,
  },
  max_attempts: 5,
  deadline: null,
  status: "scheduled",
  created_by: "human",
  created_at: "2026-10-20T00:00:00Z",
  updated_at: "2026-10-20T00:00:00Z",
  next_fire_at: "2026-10-20T04:00:00Z",
  last_fired_at: null,
  attempts: 0,
};

describe("SchedulePanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("loads and renders scheduled tasks", async () => {
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === "hub_schedule_list") {
        return [mockTask];
      }
      return null;
    });

    render(<SchedulePanel workspace="/home/user/repo" />);

    expect(screen.getByText("⏱️ Scheduled Goals")).toBeInTheDocument();
    expect(await screen.findByText("Nightly Integration Pass")).toBeInTheDocument();
    expect(screen.getByText("⏱️ scheduled")).toBeInTheDocument();
    expect(screen.getByText(/Full Team/)).toBeInTheDocument();
    expect(screen.getByText("Cancel Task")).toBeInTheDocument();
  });

  it("shows empty state when task list is empty", async () => {
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === "hub_schedule_list") {
        return [];
      }
      return null;
    });

    render(<SchedulePanel workspace="/home/user/repo" />);

    expect(await screen.findByText("No scheduled goals found")).toBeInTheDocument();
  });

  it("toggles the creation builder and submits a task", async () => {
    let currentTasks: ScheduledTask[] = [];

    vi.mocked(invoke).mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "hub_schedule_list") {
        return currentTasks;
      }
      if (cmd === "hub_schedule_create") {
        const createArgs = args?.args as Record<string, unknown>;
        const created: ScheduledTask = {
          ...mockTask,
          id: "task-2",
          name: createArgs.name as string,
          instruction: createArgs.instruction as string,
        };
        currentTasks = [created];
        return created;
      }
      return null;
    });

    render(<SchedulePanel workspace="/home/user/repo" availableAgents={["claude", "gemini"]} />);

    // Open form
    const newBtn = screen.getByText("➕ New Scheduled Task");
    fireEvent.click(newBtn);

    expect(screen.getByText("Create Scheduled Goal")).toBeInTheDocument();

    // Fill in task name & instruction
    const nameInput = screen.getByPlaceholderText("e.g. Nightly test & hygiene pass");
    fireEvent.change(nameInput, { target: { value: "Refactor Audit" } });

    const instructionInput = screen.getByPlaceholderText("Detailed instruction dispatched to target agent or standing team...");
    fireEvent.change(instructionInput, { target: { value: "Verify files <= 500 lines" } });

    // Submit
    const submitBtn = screen.getByText("Schedule Goal");
    fireEvent.click(submitBtn);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("hub_schedule_create", expect.any(Object));
    });

    expect(await screen.findByText("Refactor Audit")).toBeInTheDocument();
  });

  it("cancels a scheduled task", async () => {
    const activeTask: ScheduledTask = {
      ...mockTask,
      status: "scheduled",
    };

    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === "hub_schedule_list") {
        return [activeTask];
      }
      if (cmd === "hub_schedule_cancel") {
        return {
          ...activeTask,
          status: "cancelled",
        };
      }
      return null;
    });

    render(<SchedulePanel workspace="/home/user/repo" />);

    const cancelBtn = await screen.findByText("Cancel Task");
    fireEvent.click(cancelBtn);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("hub_schedule_cancel", { id: "task-1" });
    });
  });

  it("triggers manual tick and displays summary toast", async () => {
    const summary: TickSummary = {
      outcomes: [
        { id: "task-1", action: "dispatched", detail: "first fire" },
        { id: "task-2", action: "completed", detail: "gate passed" },
      ],
    };

    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === "hub_schedule_list") {
        return [mockTask];
      }
      if (cmd === "hub_schedule_tick") {
        return summary;
      }
      return null;
    });

    render(<SchedulePanel workspace="/home/user/repo" />);

    const tickBtn = screen.getByText("⚡ Run Check Now");
    fireEvent.click(tickBtn);

    expect(await screen.findByText(/Checked 2 tasks/)).toBeInTheDocument();
    expect(screen.getByText(/1 dispatched/)).toBeInTheDocument();
    expect(screen.getByText(/1 completed/)).toBeInTheDocument();
  });
});
