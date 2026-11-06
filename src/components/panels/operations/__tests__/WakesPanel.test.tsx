import { render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import WakesPanel from "../WakesPanel";
import type { AgentRecord, WakeRecord } from "../../hub/types";

describe("WakesPanel", () => {
  const sampleAgents: AgentRecord[] = [
    { id: "agent-1", display_name: "Grok" },
    { id: "agent-2", display_name: "Claude" },
  ];

  const sampleWakes: WakeRecord[] = [
    {
      id: "w-1",
      target_agent: "agent-1",
      reason: "Inspect failed build",
      status: "pending",
      requires_human_gate: true,
      created_at: "2026-10-23T12:00:00Z",
    },
    {
      id: "w-2",
      target_agent: "agent-2",
      reason: "Review pull request",
      status: "delivered",
      requires_human_gate: false,
      created_at: "2026-10-23T12:10:00Z",
    },
  ];

  it("renders empty state chip when no wakes are recorded", () => {
    render(
      <WakesPanel
        agents={sampleAgents}
        wakeTarget="agent-1"
        setWakeTarget={vi.fn()}
        wakeReason=""
        setWakeReason={vi.fn()}
        requestWake={vi.fn()}
        refreshWakes={vi.fn()}
        wakes={[]}
      />
    );

    expect(screen.getByText("Request Wake")).toBeInTheDocument();
    expect(screen.getByText("No wake requests recorded")).toBeInTheDocument();
    expect(screen.getByTestId("staleness-dot")).toBeInTheDocument();
  });

  it("renders wake cards with severity badges and human gate indicator", () => {
    const handleWake = vi.fn();
    const handleRefresh = vi.fn();

    render(
      <WakesPanel
        agents={sampleAgents}
        wakeTarget="agent-1"
        setWakeTarget={vi.fn()}
        wakeReason="Inspect failed build"
        setWakeReason={vi.fn()}
        requestWake={handleWake}
        refreshWakes={handleRefresh}
        wakes={sampleWakes}
      />
    );

    expect(screen.getByText("Inspect failed build")).toBeInTheDocument();
    expect(screen.getByText("Review pull request")).toBeInTheDocument();
    expect(screen.getByText("Pending")).toBeInTheDocument();
    expect(screen.getByText("Delivered")).toBeInTheDocument();
    expect(screen.getByText("Human Gate Required")).toBeInTheDocument();

    const wakeBtn = screen.getByRole("button", { name: "Wake (human gate)" });
    fireEvent.click(wakeBtn);
    expect(handleWake).toHaveBeenCalled();

    const refreshBtn = screen.getByRole("button", { name: "Refresh List" });
    fireEvent.click(refreshBtn);
    expect(handleRefresh).toHaveBeenCalled();
  });
});
