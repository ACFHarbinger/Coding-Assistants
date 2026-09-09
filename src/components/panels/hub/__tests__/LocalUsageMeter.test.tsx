import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { LocalUsageMeter } from "../LocalUsageMeter";
import { QuotaChart } from "../HubCharts";
import type { ProviderQuota, ProviderQuotaLocalUsage } from "../types";

// Mock invoke for QuotaChart tests
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

describe("LocalUsageMeter", () => {
  beforeEach(() => {
    (window as any).__TAURI_INTERNALS__ = {};
  });

  const sampleUsage: ProviderQuotaLocalUsage = {
    sessions: 14,
    prompt_tokens: 4967,
    completion_tokens: 40,
    cached_tokens: 2048,
    tool_calls_succeeded: 12,
    tool_calls_failed: 1,
    tool_calls_rejected: 0,
    since: 1_725_800_000,
  };

  it("renders compact, unmetered local session stats without percentage or currency", () => {
    render(
      <LocalUsageMeter
        usage={sampleUsage}
        detail="Plan budget needs an Admin API key."
      />
    );

    // Title and badge
    expect(screen.getByText("Local Session Activity")).toBeInTheDocument();
    expect(screen.getByText("Local · Unmetered")).toBeInTheDocument();
    expect(screen.getByText(/Tracking sessions since/)).toBeInTheDocument();

    // Raw counts
    expect(screen.getByText("14")).toBeInTheDocument();
    expect(screen.getByText("4,967")).toBeInTheDocument();
    expect(screen.getByText("40")).toBeInTheDocument();
    expect(screen.getByText("2,048")).toBeInTheDocument();
    expect(screen.getByText("13")).toBeInTheDocument(); // total tool calls: 12 + 1 + 0
    expect(screen.getByText(/12 ok · 1 err/)).toBeInTheDocument();

    // Detail advisory
    expect(screen.getByText("Plan budget needs an Admin API key.")).toBeInTheDocument();

    // Ensure raw numbers are NOT forced into percentages or dollar amounts
    expect(screen.queryByText(/4,967%/)).not.toBeInTheDocument();
    expect(screen.queryByText(/\$4,967/)).not.toBeInTheDocument();
    expect(screen.queryByText(/14%/)).not.toBeInTheDocument();
  });

  it("exercises a local_usage-only Mistral quota in QuotaChart", () => {
    const mistralQuotaOnlyLocalUsage: ProviderQuota = {
      agent_id: "mistral",
      provider: "mistral",
      harness_title: "Mistral Vibe",
      status: "ok",
      windows: [],
      fetched_at: 1_725_900_000,
      balance: null,
      balance_info: null,
      detail: "Admin key needed for monthly spend budget",
      local_usage: sampleUsage,
    };

    render(
      <QuotaChart
        quotas={[mistralQuotaOnlyLocalUsage]}
        refreshingIds={new Set()}
        onRefreshOne={vi.fn()}
      />
    );

    // Mistral Vibe header
    expect(screen.getByText("Mistral Vibe")).toBeInTheDocument();

    // LocalUsageMeter rendered
    expect(screen.getByTestId("local-usage-meter")).toBeInTheDocument();
    expect(screen.getByText("Local · Unmetered")).toBeInTheDocument();
    expect(screen.getByText("4,967")).toBeInTheDocument();
    expect(screen.getByText("Admin key needed for monthly spend budget")).toBeInTheDocument();

    // Fallback "No provider quota windows returned" should NOT appear
    expect(screen.queryByText("No provider quota windows returned.")).not.toBeInTheDocument();
  });

  it("renders a capped admin response plus local usage with period spend and progress window", () => {
    const cappedMistralQuota: ProviderQuota = {
      agent_id: "mistral",
      provider: "mistral",
      harness_title: "Mistral Vibe",
      status: "ok",
      windows: [
        {
          label: "Monthly spend",
          family: "Mistral Admin",
          used_percent: 35,
          remaining_percent: 65,
          resets_at: null,
          window_minutes: null,
        },
      ],
      fetched_at: 1_725_900_000,
      balance: "Spent $35.00 USD this period (Vibe $10.00) · 2026-09-01 → 2026-09-30",
      balance_info: {
        currency: "USD",
        total: 35.0,
        kind: "spend",
      },
      detail: null,
      local_usage: sampleUsage,
    };

    render(
      <QuotaChart
        quotas={[cappedMistralQuota]}
        refreshingIds={new Set()}
        onRefreshOne={vi.fn()}
      />
    );

    // Period spend label and value
    expect(screen.getByText("Period spend")).toBeInTheDocument();
    expect(
      screen.getByText("Spent $35.00 USD this period (Vibe $10.00) · 2026-09-01 → 2026-09-30")
    ).toBeInTheDocument();

    // Must NOT label spend as "Account balance" or "Available Balance"
    expect(screen.queryByText("Account balance")).not.toBeInTheDocument();
    expect(screen.queryByText("Available Balance")).not.toBeInTheDocument();

    // Monthly spend window progress
    expect(screen.getByText(/Monthly spend/)).toBeInTheDocument();
    expect(screen.getByText("65% remaining")).toBeInTheDocument();

    // Local session usage meter is rendered alongside
    expect(screen.getByTestId("local-usage-meter")).toBeInTheDocument();
    expect(screen.getByText("4,967")).toBeInTheDocument();

    // PaygQuotaMeter must not render for spend
    expect(screen.queryByTestId("payg-balance-history")).not.toBeInTheDocument();
  });
});
