import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import UsagePanel from "../UsagePanel";
import type { AgentRecord, BudgetStatus, ProviderQuota } from "../../hub/types";

vi.mock("../../../lib/tauri", () => ({
  invoke: vi.fn().mockResolvedValue(null),
  isTauriRuntime: vi.fn().mockReturnValue(true),
}));

describe("UsagePanel hybrid NOC telemetry (V4 / #375)", () => {
  const agents: AgentRecord[] = [
    { id: "claude", display_name: "Claude Code" },
    { id: "codex", display_name: "Codex" },
  ];

  const budgets: BudgetStatus[] = [
    {
      agent_id: "claude",
      spent_units: 45,
      limit_units: 100,
      paused: false,
      updated_at: "2026-10-23T12:00:00Z",
    },
    {
      agent_id: "codex",
      spent_units: 100,
      limit_units: 100,
      paused: true,
      updated_at: "2026-10-23T12:00:00Z",
    },
  ];

  const quotas: ProviderQuota[] = [
    {
      agent_id: "claude",
      provider: "anthropic",
      harness_title: "Claude Code",
      status: "ok",
      fetched_at: 1_700_000_000,
      windows: [],
    },
  ];

  it("renders KPI metric strip for comparable counts without composite spend sum", () => {
    render(
      <UsagePanel
        agents={agents}
        budgetAgent="claude"
        setBudgetAgent={vi.fn()}
        budgetLimit="100"
        setBudgetLimit={vi.fn()}
        setBudget={vi.fn()}
        refreshBudgets={vi.fn()}
        refreshQuotas={vi.fn()}
        refreshStaleQuotas={vi.fn()}
        budgets={budgets}
        quotas={quotas}
        refreshingQuotaIds={new Set()}
        refreshSingleQuota={vi.fn()}
        budgetSpend="10"
        setBudgetSpend={vi.fn()}
        recordSpend={vi.fn()}
        resumeBudget={vi.fn()}
      />
    );

    const kpiGrid = screen.getByTestId("usage-kpi-grid");
    expect(kpiGrid).toBeInTheDocument();
    expect(screen.getByText("Active Budgets")).toBeInTheDocument();
    expect(screen.getByText("Paused Agents")).toBeInTheDocument();
    expect(screen.getByText("Live Quota Providers")).toBeInTheDocument();
    expect(screen.getAllByText("1").length).toBeGreaterThanOrEqual(1);
  });

  it("renders individual agent rows with utilization meter and action triggers", () => {
    const recordSpend = vi.fn();
    const resumeBudget = vi.fn();

    render(
      <UsagePanel
        agents={agents}
        budgetAgent=""
        setBudgetAgent={vi.fn()}
        budgetLimit="100"
        setBudgetLimit={vi.fn()}
        setBudget={vi.fn()}
        refreshBudgets={vi.fn()}
        refreshQuotas={vi.fn()}
        refreshStaleQuotas={vi.fn()}
        budgets={budgets}
        quotas={quotas}
        refreshingQuotaIds={new Set()}
        refreshSingleQuota={vi.fn()}
        budgetSpend="10"
        setBudgetSpend={vi.fn()}
        recordSpend={recordSpend}
        resumeBudget={resumeBudget}
      />
    );

    expect(screen.getAllByText("claude").length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText("45 / 100 units · active")).toBeInTheDocument();

    expect(screen.getAllByText("codex").length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText("100 / 100 units · paused")).toBeInTheDocument();

    // The paused agent has a Resume button
    const resumeBtn = screen.getByRole("button", { name: "Resume" });
    fireEvent.click(resumeBtn);
    expect(resumeBudget).toHaveBeenCalledWith("codex");

    // Click record usage
    const recordBtns = screen.getAllByRole("button", { name: "Record usage" });
    fireEvent.click(recordBtns[0]);
    expect(recordSpend).toHaveBeenCalledWith("claude");
  });
});
