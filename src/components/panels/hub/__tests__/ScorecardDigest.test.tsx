import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ScorecardDigest } from "../ScorecardDigest";
import type { OwnerDigest } from "../types";

const digest: OwnerDigest = {
  window_days: 7,
  summary: {
    pending_count: 2,
    oldest_pending_at: "2026-10-19T09:00:00Z",
    unresolved_findings: 1,
    resolved_window: { desktop: 2, telegram: 1, tcp: 0, total: 3 },
  },
  groups: [
    {
      work_session_id: "session-1",
      work_session_name: "release-cut",
      workspace_id: "ws-1",
      unresolved_findings: 1,
      resolved_window: { desktop: 1, telegram: 0, tcp: 0, total: 1 },
      pending: [
        {
          id: "evt-1",
          kind: "os_tool",
          subject: "git push",
          agent: "grok",
          created_at: "2026-10-19T09:00:00Z",
          age_seconds: 3600,
          task_id: "task-9",
          github_issue: "#344",
        },
      ],
    },
    {
      work_session_id: null,
      work_session_name: null,
      workspace_id: null,
      unresolved_findings: 0,
      resolved_window: { desktop: 1, telegram: 1, tcp: 0, total: 2 },
      pending: [
        {
          id: "wake-1",
          kind: "wake",
          subject: "continue slice",
          agent: "grok",
          created_at: "2026-10-19T10:00:00Z",
          age_seconds: 60,
        },
      ],
    },
  ],
};

describe("ScorecardDigest", () => {
  it("renders nothing without an owner digest", () => {
    const { container } = render(<ScorecardDigest digest={null} />);
    expect(container).toBeEmptyDOMElement();
  });

  it("shows summary stats, session groups, and channel splits", () => {
    render(<ScorecardDigest digest={digest} />);
    expect(screen.getByText("Owner Review Digest")).toBeInTheDocument();
    expect(screen.getByText("2")).toBeInTheDocument();
    expect(screen.getByText("release-cut")).toBeInTheDocument();
    expect(screen.getByText("Ungrouped")).toBeInTheDocument();
    expect(screen.getByText("#344")).toBeInTheDocument();
    expect(screen.getByText("task-9")).toBeInTheDocument();
  });

  it("splits resolutions by channel without a composite score", () => {
    render(<ScorecardDigest digest={digest} />);
    expect(screen.getByText("1 desktop · 1 telegram")).toBeInTheDocument();
    expect(screen.getByText("2 desktop · 1 telegram")).toBeInTheDocument();
  });

  it("is advisory: no approve, reject, or bulk-action controls", () => {
    render(<ScorecardDigest digest={digest} />);
    expect(screen.queryByRole("button")).toBeNull();
  });
});
