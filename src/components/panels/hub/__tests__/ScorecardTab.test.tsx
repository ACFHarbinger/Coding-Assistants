import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { ScorecardTab } from "../ScorecardTab";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

describe("Scorecard usage coverage", () => {
  beforeEach(() => vi.clearAllMocks());

  it("labels partial currency totals while retaining measured zero", async () => {
    vi.mocked(invoke).mockResolvedValue({
      agents: [], harnesses: [], aggregation_version: 1,
      generated_at: "2026-10-19T00:00:00Z",
      providers: [{
        provider: "example", via_gateway: null,
        tokens: { value: null, provenance: "unknown", unit: "tokens" },
        cost: {
          value: 0, provenance: "measured", unit: "currency",
          coverage: { measured_requests: 1, total_requests: 2 },
        },
        request_count: { value: 2, provenance: "measured", unit: "requests" },
      }],
    });
    render(<ScorecardTab />);
    expect(await screen.findByText("Partial: 1/2 requests fully measured")).toBeInTheDocument();
    expect(screen.getByText("$0.0000")).toBeInTheDocument();
  });
});
