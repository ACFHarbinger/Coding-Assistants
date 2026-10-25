import { render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import JournalPanel from "../JournalPanel";
import type { AuditEvent } from "../../hub/types";

describe("JournalPanel", () => {
  const sampleEvents: AuditEvent[] = [
    {
      id: "ev-1",
      root_path: "/workspace",
      path: "src/index.css",
      operation: "os_tool.file_write",
      observed_at: "2026-10-23T12:00:00Z",
      process_json: "{}",
      status: "pending",
      event_hash: "abcdef1234567890",
    },
    {
      id: "ev-2",
      root_path: "/workspace",
      path: "quota/groq",
      operation: "budget.quota_check",
      observed_at: "2026-10-23T12:05:00Z",
      process_json: "{}",
      status: "approved",
      event_hash: "1234567890abcdef",
    },
    {
      id: "ev-3",
      root_path: "/workspace",
      path: "tasks/42",
      operation: "task.dispatch",
      observed_at: "2026-10-23T12:10:00Z",
      process_json: "{}",
      status: "pending",
      event_hash: "fedcba0987654321",
    },
    {
      id: "ev-4",
      root_path: "/workspace",
      path: "harness/1",
      operation: "oversight.quarantine",
      observed_at: "2026-10-23T12:15:00Z",
      process_json: "{}",
      status: "quarantined",
      event_hash: "0987654321fedcba",
    },
  ];

  it("renders empty state chip when no audit events exist", () => {
    render(
      <JournalPanel
        auditEvents={[]}
        auditShowAll={false}
        setAuditShowAll={vi.fn()}
        refreshAuditEvents={vi.fn()}
        approveAudit={vi.fn()}
        quarantineAudit={vi.fn()}
      />
    );
    expect(screen.getByText("No pending audit events")).toBeInTheDocument();
  });

  it("renders category pills and hashes for audit stream", () => {
    const approve = vi.fn();
    const quarantine = vi.fn();
    render(
      <JournalPanel
        auditEvents={sampleEvents}
        auditShowAll={true}
        setAuditShowAll={vi.fn()}
        refreshAuditEvents={vi.fn()}
        approveAudit={approve}
        quarantineAudit={quarantine}
      />
    );

    expect(screen.getByText("os_tool")).toBeInTheDocument();
    expect(screen.getByText("budget")).toBeInTheDocument();
    expect(screen.getByText("task")).toBeInTheDocument();
    expect(screen.getByText("oversight")).toBeInTheDocument();
    expect(screen.getByText("abcdef1234")).toBeInTheDocument();

    const approveButtons = screen.getAllByRole("button", { name: "Approve" });
    fireEvent.click(approveButtons[0]);
    expect(approve).toHaveBeenCalledWith("ev-1");
  });
});
