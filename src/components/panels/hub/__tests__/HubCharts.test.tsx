import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { BalanceBreakdownBar, formatCurrency, QuotaChart } from "../HubCharts";
import type { ProviderQuota, ProviderQuotaBalance } from "../types";

vi.mock("../../../lib/tauri", () => ({
  invoke: vi.fn().mockImplementation((cmd: string) => {
    if (cmd === "settings_get_effective") {
      return Promise.resolve({
        orchestration: {
          allow_metered_quota_probes: true,
          quota_auto_refresh_enabled: false,
          quota_auto_refresh_interval_secs: 300,
        },
      });
    }
    return Promise.resolve(null);
  }),
  isTauriRuntime: vi.fn().mockReturnValue(true),
}));

describe("formatCurrency", () => {
  it("formats positive USD amounts correctly", () => {
    expect(formatCurrency(20.0, "USD")).toBe("$20.00");
    expect(formatCurrency(4.1, "USD")).toBe("$4.10");
    expect(formatCurrency(0, "USD")).toBe("$0.00");
    expect(formatCurrency(0.99, "$")).toBe("$0.99");
  });

  it("formats negative amounts with leading minus", () => {
    expect(formatCurrency(-1.5, "USD")).toBe("-$1.50");
  });

  it("formats standard international currency symbols", () => {
    expect(formatCurrency(15.5, "EUR")).toBe("€15.50");
    expect(formatCurrency(25.0, "GBP")).toBe("£25.00");
    expect(formatCurrency(88.0, "CNY")).toBe("¥88.00");
  });

  it("formats unmapped currency codes with trailing code", () => {
    expect(formatCurrency(12.0, "CAD")).toBe("12.00 CAD");
    expect(formatCurrency(5.0, "AUD")).toBe("5.00 AUD");
  });
});

describe("BalanceBreakdownBar", () => {
  const balance: ProviderQuotaBalance = {
    currency: "USD",
    total: 20.0,
    kind: "spend",
    spent: 20.0,
    gift: 51.22,
  };

  it("renders stacked bar segments and legend items", () => {
    render(
      <BalanceBreakdownBar
        balance={balance}
        balanceText="$20.00 included this cycle"
      />
    );

    expect(screen.getByTestId("balance-breakdown-bar")).toBeInTheDocument();
    expect(screen.getByText("Account balance")).toBeInTheDocument();
    expect(screen.getByText("$20.00 included this cycle")).toBeInTheDocument();

    // Check bar segments
    const spentSegment = screen.getByTestId("breakdown-spent");
    expect(spentSegment).toBeInTheDocument();
    expect(spentSegment).toHaveAttribute("title", "Spent: $20.00");

    const freeSegment = screen.getByTestId("breakdown-free");
    expect(freeSegment).toBeInTheDocument();
    expect(freeSegment).toHaveAttribute("title", "Free / Bonus: $51.22");

    // Check legend items
    expect(screen.getByText("Spent $20.00")).toBeInTheDocument();
    expect(screen.getByText("Budget $0.00")).toBeInTheDocument();
    expect(screen.getByText("Free $51.22")).toBeInTheDocument();
  });

  it("computes remaining budget when partially spent", () => {
    const partial: ProviderQuotaBalance = {
      currency: "USD",
      total: 20.0,
      kind: "spend",
      spent: 5.0,
      gift: 0,
    };
    render(<BalanceBreakdownBar balance={partial} />);

    expect(screen.getByTestId("breakdown-spent")).toBeInTheDocument();
    const budgetSegment = screen.getByTestId("breakdown-budget");
    expect(budgetSegment).toBeInTheDocument();
    expect(budgetSegment).toHaveAttribute("title", "Remaining Budget: $15.00");

    expect(screen.getByText("Spent $5.00")).toBeInTheDocument();
    expect(screen.getByText("Budget $15.00")).toBeInTheDocument();
    expect(screen.queryByText(/Free/)).not.toBeInTheDocument();
  });

  it("handles over-budget spend with warning styling", () => {
    const overBudget: ProviderQuotaBalance = {
      currency: "USD",
      total: 20.0,
      kind: "spend",
      spent: 24.1,
      gift: 10.0,
    };
    render(<BalanceBreakdownBar balance={overBudget} />);

    const spentSegment = screen.getByTestId("breakdown-spent");
    expect(spentSegment).toHaveStyle({ background: "#ef4444" });
    expect(screen.getByText("Spent $24.10")).toBeInTheDocument();
  });

  it("handles zero-spent balance (e.g. granted + topped-up)", () => {
    const zeroSpent: ProviderQuotaBalance = {
      currency: "USD",
      total: 7.34,
      kind: "spend",
      spent: 0,
      gift: 5.0,
    };
    render(<BalanceBreakdownBar balance={zeroSpent} />);

    expect(screen.queryByTestId("breakdown-spent")).not.toBeInTheDocument();
    expect(screen.getByTestId("breakdown-budget")).toBeInTheDocument();
    expect(screen.getByTestId("breakdown-free")).toBeInTheDocument();
    expect(screen.getByText("Budget $7.34")).toBeInTheDocument();
    expect(screen.getByText("Free $5.00")).toBeInTheDocument();
  });
});

describe("QuotaChart with balance_info spend", () => {
  beforeEach(() => {
    (window as any).__TAURI_INTERNALS__ = {};
  });

  it("renders BalanceBreakdownBar when balance_info with kind=spend is present on provider with windows", () => {
    const cursorQuota: ProviderQuota = {
      agent_id: "cursor",
      provider: "cursor",
      harness_title: "Cursor Agent",
      status: "ok",
      fetched_at: 1_725_800_000,
      balance: "$20.00 included this cycle",
      balance_info: {
        currency: "USD",
        total: 20.0,
        kind: "spend",
        spent: 20.0,
        gift: 51.22,
      },
      windows: [
        {
          label: "Included allowance",
          used_percent: 14,
          remaining_percent: 86,
        },
      ],
    };

    render(
      <QuotaChart
        quotas={[cursorQuota]}
        refreshingIds={new Set()}
        onRefreshOne={vi.fn()}
      />
    );

    expect(screen.getByTestId("balance-breakdown-bar")).toBeInTheDocument();
    expect(screen.getByText(/Included allowance/)).toBeInTheDocument();
    expect(screen.getByText(/86% remaining/)).toBeInTheDocument();
    expect(screen.getByText("Spent $20.00")).toBeInTheDocument();
    expect(screen.getByText("Free $51.22")).toBeInTheDocument();
  });

  it("falls back to plain balance text when balance_info is absent", () => {
    const plainQuota: ProviderQuota = {
      agent_id: "cursor",
      provider: "cursor",
      harness_title: "Cursor Agent",
      status: "ok",
      fetched_at: 1_725_800_000,
      balance: "$20.00 included this cycle",
      windows: [
        {
          label: "Included allowance",
          used_percent: 14,
          remaining_percent: 86,
        },
      ],
    };

    render(
      <QuotaChart
        quotas={[plainQuota]}
        refreshingIds={new Set()}
        onRefreshOne={vi.fn()}
      />
    );

    expect(screen.queryByTestId("balance-breakdown-bar")).not.toBeInTheDocument();
    expect(screen.getByText("$20.00 included this cycle")).toBeInTheDocument();
    expect(screen.getByText("Account balance")).toBeInTheDocument();
  });

  it("suppresses 'no provider quota windows returned' when balance_info is present", () => {
    const noWindowQuota: ProviderQuota = {
      agent_id: "custom",
      provider: "custom",
      harness_title: "Custom Agent",
      status: "ok",
      fetched_at: 1_725_800_000,
      balance_info: {
        currency: "USD",
        total: 10.0,
        kind: "spend",
        spent: 0,
        gift: 2.0,
      },
      windows: [],
    };

    render(
      <QuotaChart
        quotas={[noWindowQuota]}
        refreshingIds={new Set()}
        onRefreshOne={vi.fn()}
      />
    );

    expect(screen.getByTestId("balance-breakdown-bar")).toBeInTheDocument();
    expect(screen.queryByText("No provider quota windows returned.")).not.toBeInTheDocument();
  });
});
