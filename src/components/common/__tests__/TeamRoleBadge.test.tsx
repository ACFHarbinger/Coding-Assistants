import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { TeamRoleBadge, getRoleVisual } from "../TeamRoleBadge";

describe("TeamRoleBadge (U21 / #315)", () => {
  it("renders nothing when role is undefined, null, or empty whitespace", () => {
    const { container: c1 } = render(<TeamRoleBadge role={undefined} />);
    expect(c1.firstChild).toBeNull();

    const { container: c2 } = render(<TeamRoleBadge role={null} />);
    expect(c2.firstChild).toBeNull();

    const { container: c3 } = render(<TeamRoleBadge role="   " />);
    expect(c3.firstChild).toBeNull();
  });

  it("renders preset visual styles for lead, reviewer, implementer, and observer", () => {
    const lead = getRoleVisual("lead");
    expect(lead.label).toBe("Team Lead");
    expect(lead.icon).toBe("★");

    const reviewer = getRoleVisual("reviewer");
    expect(reviewer.label).toBe("Reviewer");
    expect(reviewer.icon).toBe("✓");

    const implementer = getRoleVisual("implementer");
    expect(implementer.label).toBe("Implementer");
    expect(implementer.icon).toBe("⚡");

    const observer = getRoleVisual("observer");
    expect(observer.label).toBe("Observer");
    expect(observer.icon).toBe("◉");

    const custom = getRoleVisual("Security Auditor");
    expect(custom.label).toBe("Security Auditor");
    expect(custom.icon).toBe("◈");
  });

  it("renders badge in the DOM with correct text and optional clear button", () => {
    const onClear = vi.fn();
    render(<TeamRoleBadge role="lead" onClear={onClear} />);

    expect(screen.getByText("Team Lead")).toBeTruthy();
    expect(screen.getByText("★")).toBeTruthy();

    const clearButton = screen.getByRole("button", { name: /clear team lead role/i });
    fireEvent.click(clearButton);
    expect(onClear).toHaveBeenCalledTimes(1);
  });

  it("renders custom role label as given", () => {
    render(<TeamRoleBadge role="DevOps Engineer" />);
    expect(screen.getByText("DevOps Engineer")).toBeTruthy();
    expect(screen.getByText("◈")).toBeTruthy();
  });
});
