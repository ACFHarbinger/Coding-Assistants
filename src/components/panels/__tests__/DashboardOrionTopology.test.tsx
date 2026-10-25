import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { DashboardOrionTopology, type DashboardTopologyProps } from "../dashboard/DashboardOrionTopology";

vi.mock("../../common/telemetry/orion", () => ({
  OrionConstellationGraph: () => <div>Live graph</div>,
  OrionRadarChart: ({ series }: { series: unknown }) => <div data-testid="radar">{JSON.stringify(series)}</div>,
  OrionHoneycombMap: ({ cells }: { cells: unknown }) => <div data-testid="cells">{JSON.stringify(cells)}</div>,
  OrionRadialRingMeter: ({ rings }: { rings: Array<{ percent: number; valueText: string }> }) => (
    <div>{rings.map((ring, i) => <div key={i} data-testid={`ring-${i}`}>{ring.percent}% {ring.valueText}</div>)}</div>
  ),
}));

const empty: DashboardTopologyProps = { agents: [], metrics: [], budgets: [], tasks: [], messages: [], wakes: [] };

describe("live Orion telemetry", () => {
  it("shows an empty roster and preserves zero utilization", () => {
    render(<DashboardOrionTopology {...empty} />);
    expect(screen.getByRole("status").textContent).toContain("No workspace agents");
    expect(screen.queryByText("Live graph")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Radial Arcs" }));
    for (let i = 0; i < 4; i++) expect(screen.getByTestId(`ring-${i}`).textContent).toMatch(/^0%/);
  });

  it("counts running workflows", () => {
    render(<DashboardOrionTopology {...empty} tasks={[
      { id: "1", title: "Executing", status: "running" },
      { id: "2", title: "Waiting", status: "pending" },
    ]} />);
    fireEvent.click(screen.getByRole("button", { name: "Radial Arcs" }));
    expect(screen.getByTestId("ring-0").textContent).toBe("50% 1 / 2 active");
  });

  it("does not invent radar or honeycomb usage when metrics are missing", () => {
    render(<DashboardOrionTopology {...empty} agents={[{ id: "a", display_name: "Agent" }]} />);
    fireEvent.click(screen.getByRole("button", { name: "Radar Comparison" }));
    expect(screen.getByRole("status").textContent).toContain("Metrics unavailable");
    expect(screen.getByTestId("radar").textContent).toBe("[]");
    fireEvent.click(screen.getByRole("button", { name: "Honeycomb Matrix" }));
    expect(screen.getByTestId("cells").textContent).toContain('"intensity":0');
    expect(screen.getByTestId("cells").textContent).toContain("Unavailable");
  });
});
