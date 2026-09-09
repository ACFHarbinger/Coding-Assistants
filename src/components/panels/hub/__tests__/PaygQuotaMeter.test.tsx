import { beforeEach, describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { PaygQuotaMeter, STORAGE_KEY_PREFIX } from "../PaygQuotaMeter";
import type { ProviderQuota } from "../types";

describe("PaygQuotaMeter", () => {
  let store: Record<string, string> = {};

  beforeEach(() => {
    store = {};
    globalThis.localStorage = {
      getItem: (key: string) => store[key] ?? null,
      setItem: (key: string, value: string) => {
        store[key] = value;
      },
      removeItem: (key: string) => {
        delete store[key];
      },
      clear: () => {
        store = {};
      },
      length: 0,
      key: () => null,
    };
  });

  const deepseekQuotaWithBreakdown: ProviderQuota = {
    agent_id: "deepseek",
    provider: "deepseek",
    harness_title: "DeepSeek",
    status: "ok",
    windows: [],
    fetched_at: 1_725_000_000,
    balance: "Balance $12.34 USD — granted $5.00 + topped-up $7.34",
    balance_info: {
      currency: "USD",
      total: 12.34,
      paid: 7.34,
      gift: 5.0,
      topped_up: 7.34,
      granted: 5.0,
    },
  };

  it("renders structured balances: available, paid, and promotional grant", () => {
    render(<PaygQuotaMeter quota={deepseekQuotaWithBreakdown} />);

    expect(screen.getByText("Available Balance")).toBeInTheDocument();
    expect(screen.getByText("$12.34")).toBeInTheDocument();
    expect(screen.getByText("Paid / Topped-up")).toBeInTheDocument();
    expect(screen.getByText("$7.34")).toBeInTheDocument();
    expect(screen.getByText("Free / Gift Grant")).toBeInTheDocument();
    expect(screen.getByText("$5.00")).toBeInTheDocument();
  });

  it("renders segmented balance bar with paid and gift labels", () => {
    render(<PaygQuotaMeter quota={deepseekQuotaWithBreakdown} />);

    expect(screen.getByText("Credit Composition")).toBeInTheDocument();
    expect(screen.getByText(/Paid Refill \(59\.5%\)/)).toBeInTheDocument();
    expect(screen.getByText(/Promotional Grant \(40\.5%\)/)).toBeInTheDocument();
  });

  it("guards the truthful empty/insufficient-history state when fewer than two snapshots exist", () => {
    // With no prior snapshots in localStorage, only 1 snapshot is recorded on mount.
    render(<PaygQuotaMeter quota={deepseekQuotaWithBreakdown} />);

    const emptyBox = screen.getByTestId("insufficient-history");
    expect(emptyBox).toBeInTheDocument();
    expect(screen.getByText("Insufficient Snapshot History")).toBeInTheDocument();
    expect(screen.getByText(/1 balance snapshot recorded/i)).toBeInTheDocument();
    expect(screen.queryByRole("img", { name: "Recorded balance over time" })).not.toBeInTheDocument();
  });

  it("renders SVG balance history chart when at least two snapshots exist", () => {
    // Seed localStorage with multiple historical snapshots
    const key = `${STORAGE_KEY_PREFIX}deepseek`;
    const pastSnapshots = [
      { timestamp: 1_724_900_000, total: 15.0, paid: 10.0, gift: 5.0 },
      { timestamp: 1_724_950_000, total: 13.5, paid: 8.5, gift: 5.0 },
    ];
    localStorage.setItem(key, JSON.stringify(pastSnapshots));

    render(<PaygQuotaMeter quota={deepseekQuotaWithBreakdown} />);

    expect(screen.queryByTestId("insufficient-history")).not.toBeInTheDocument();
    expect(screen.getByRole("img", { name: "Recorded balance over time" })).toBeInTheDocument();
    expect(screen.getByText("Balance History")).toBeInTheDocument();
    expect(screen.getByText(/3 snapshots/i)).toBeInTheDocument();
  });

  it("degrades cleanly when no breakdown is provided", () => {
    const quotaWithoutBreakdown: ProviderQuota = {
      agent_id: "deepseek",
      provider: "deepseek",
      harness_title: "DeepSeek",
      status: "ok",
      windows: [],
      fetched_at: 1_725_000_000,
      balance: "Balance $88.00 CNY",
      balance_info: {
        currency: "CNY",
        total: 88.0,
      },
    };

    render(<PaygQuotaMeter quota={quotaWithoutBreakdown} />);

    expect(screen.getByText("$88.00")).toBeInTheDocument();
    expect(screen.queryByText("Paid / Topped-up")).not.toBeInTheDocument();
    expect(screen.queryByText("Free / Gift Grant")).not.toBeInTheDocument();
  });

  it("labels spend appropriately and does not record spend as balance history", () => {
    const spendQuota: ProviderQuota = {
      agent_id: "mistral",
      provider: "mistral",
      harness_title: "Mistral Vibe",
      status: "ok",
      windows: [],
      fetched_at: 1_725_000_000,
      balance: "Spent $24.50 USD this period",
      balance_info: {
        currency: "USD",
        total: 24.5,
        kind: "spend",
      },
    };

    render(<PaygQuotaMeter quota={spendQuota} />);

    expect(screen.getByText("Period Spend")).toBeInTheDocument();
    expect(screen.getByText("$24.50")).toBeInTheDocument();
    expect(screen.getByText("Total period spend")).toBeInTheDocument();
    expect(screen.queryByText("Available Balance")).not.toBeInTheDocument();
    expect(screen.queryByText("Credit Composition")).not.toBeInTheDocument();
    expect(screen.queryByTestId("payg-balance-history")).not.toBeInTheDocument();

    // Verify localStorage was not populated with a balance snapshot
    expect(store[`${STORAGE_KEY_PREFIX}mistral`]).toBeUndefined();
  });
});
