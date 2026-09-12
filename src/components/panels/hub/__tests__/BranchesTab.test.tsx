import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import BranchesTab, { formatCommitAge } from "../BranchesTab";

vi.mock("../../../../lib/tauri", () => ({
  invoke: vi.fn(),
}));

vi.mock("../../../../app/hubState", () => ({
  loadWorkspaceRoot: vi.fn(() => "./workspace"),
}));

describe("BranchesTab", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("formats commit age from a unix timestamp", () => {
    expect(formatCommitAge(1_700_000_000, 1_700_000_030)).toBe("just now");
    expect(formatCommitAge(1_700_000_000, 1_700_003_600)).toBe("1h ago");
    expect(formatCommitAge(1_700_000_000, 1_700_086_400)).toBe("1d ago");
  });

  it("asks for an absolute workspace when Config has none", () => {
    render(<BranchesTab />);
    expect(screen.getByText(/Set an absolute workspace root/)).toBeTruthy();
  });
});
