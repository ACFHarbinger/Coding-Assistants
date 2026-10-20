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

  it("renders probe states and the reviewer signature without composite scores", async () => {
    const unknownFigure = { value: null, provenance: "unknown", unit: "seconds" };
    vi.mocked(invoke).mockResolvedValue({
      agents: [{
        agent_id: "cursor", display_name: "Cursor", oversight_role: null,
        open_security_events: [], oldest_pending_human_decision: null,
        tokens: { value: null, provenance: "unknown", unit: "tokens" },
        cost: { value: null, provenance: "unknown", unit: "currency" },
        budget_paused: false, provenance: "unknown",
        task_count: { value: null, provenance: "unknown", unit: "tasks" },
        unresolved_disagreements: 2,
      }],
      harnesses: [], providers: [], aggregation_version: 1,
      generated_at: "2026-10-19T00:00:00Z",
      probes: [
        { probe: "quota_vs_cost", subject: "deepseek", state: "disagreement",
          reason: "measured usage spend 5.0000 exceeds balance decrease 0.3400",
          source_event_ids: ["a", "b", "c"], linked_agents: [] },
        { probe: "git_scope_vs_touch_set", subject: "global", state: "unknown",
          reason: "no git-diff source in the Hub yet", source_event_ids: [], linked_agents: [] },
      ],
      reviewers: [{
        reviewer_id: "chat", display_name: "Chat",
        findings_recorded: { value: 3, provenance: "measured", unit: "findings" },
        findings_corrected_by_owner: { value: 1, provenance: "measured", unit: "findings" },
        findings_per_task: { value: 1.5, provenance: "measured", unit: "findings/task" },
        owner_override_reversal_rate: { value: 0.33, provenance: "measured", unit: "corrected/recorded" },
        zero_evidence_approvals: { value: null, provenance: "unknown", unit: "approvals" },
        review_dwell: unknownFigure,
      }],
    });
    render(<ScorecardTab />);
    // Disagreement chip + reason, unknown probe with its honest reason.
    expect(await screen.findByText("disagreement")).toBeInTheDocument();
    expect(screen.getByText(/exceeds balance decrease/)).toBeInTheDocument();
    expect(screen.getAllByText("unknown").length).toBeGreaterThan(0);
    // Card badge is a count, never a score.
    expect(screen.getByText("⚡ 2 disagreements")).toBeInTheDocument();
    // Reviewer figures stay descriptive.
    expect(screen.getByText("1.5 findings/task")).toBeInTheDocument();
  });
});
