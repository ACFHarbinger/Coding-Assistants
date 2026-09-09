import { describe, expect, it } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { PaygQuotaMeter } from "../PaygQuotaMeter";
import type { ProviderQuota } from "../types";

describe("PaygQuotaMeter", () => {
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

  it("allows switching metrics and dimensions in the DeepSeek Platform dashboard", () => {
    render(<PaygQuotaMeter quota={deepseekQuotaWithBreakdown} />);

    expect(screen.getByText("Platform Usage Activity")).toBeInTheDocument();
    expect(screen.getByText("deepseek-chat")).toBeInTheDocument();
    expect(screen.getByText("deepseek-reasoner (R1)")).toBeInTheDocument();

    // Switch metric to Tokens
    fireEvent.click(screen.getByRole("button", { name: "Tokens" }));
    expect(screen.getByRole("button", { name: "Tokens" })).toHaveStyle({
      background: "var(--primary)",
    });

    // Switch dimension to By API Key
    fireEvent.click(screen.getByRole("button", { name: "By API Key" }));
    expect(screen.getByText("Default Key (sk-...)")).toBeInTheDocument();
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
});
