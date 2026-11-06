import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { RoleAssignmentControl } from "../agents/RoleAssignmentControl";
import * as api from "../../api";

vi.mock("../../api", () => ({
  setAgentRole: vi.fn(),
}));

describe("RoleAssignmentControl (U21 / #315)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders '+ Role' button when no role is assigned", () => {
    render(
      <RoleAssignmentControl
        agentId="human"
        displayName="Human"
        currentRole={null}
        onRoleChanged={vi.fn()}
      />
    );

    expect(screen.getByRole("button", { name: /\+ role/i })).toBeTruthy();
  });

  it("renders role badge and 'Change' button when role is assigned", () => {
    render(
      <RoleAssignmentControl
        agentId="claude"
        displayName="Claude"
        currentRole="lead"
        onRoleChanged={vi.fn()}
      />
    );

    expect(screen.getByText("Team Lead")).toBeTruthy();
    expect(screen.getByRole("button", { name: /change/i })).toBeTruthy();
  });

  it("can clear assigned role via the clear button on the badge", async () => {
    const onRoleChanged = vi.fn();
    vi.mocked(api.setAgentRole).mockResolvedValue({
      id: "claude",
      display_name: "Claude",
      role: null,
    });

    render(
      <RoleAssignmentControl
        agentId="claude"
        displayName="Claude"
        currentRole="lead"
        onRoleChanged={onRoleChanged}
      />
    );

    const clearBtn = screen.getByRole("button", { name: /clear team lead role/i });
    fireEvent.click(clearBtn);

    await waitFor(() => {
      expect(api.setAgentRole).toHaveBeenCalledWith("claude", null);
    });
    expect(onRoleChanged).toHaveBeenCalled();
  });

  it("opens inline editor, lets user pick preset, and saves", async () => {
    const onRoleChanged = vi.fn();
    vi.mocked(api.setAgentRole).mockResolvedValue({
      id: "gemini",
      display_name: "Gemini",
      role: "reviewer",
    });

    render(
      <RoleAssignmentControl
        agentId="gemini"
        displayName="Gemini"
        currentRole={null}
        onRoleChanged={onRoleChanged}
      />
    );

    // Open editor
    fireEvent.click(screen.getByRole("button", { name: /\+ role/i }));

    // Select preset "reviewer"
    const select = screen.getByRole("combobox") as HTMLSelectElement;
    fireEvent.change(select, { target: { value: "reviewer" } });

    // Save
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(api.setAgentRole).toHaveBeenCalledWith("gemini", "reviewer");
    });
    expect(onRoleChanged).toHaveBeenCalled();
  });

  it("allows entering a custom role and enforces max length of 64 chars", async () => {
    render(
      <RoleAssignmentControl
        agentId="human"
        displayName="Human"
        currentRole={null}
        onRoleChanged={vi.fn()}
      />
    );

    fireEvent.click(screen.getByRole("button", { name: /\+ role/i }));

    const select = screen.getByRole("combobox") as HTMLSelectElement;
    fireEvent.change(select, { target: { value: "custom" } });

    const input = screen.getByPlaceholderText(/e\.g\. DevOps, QA, Architect/i) as HTMLInputElement;

    // Try overlong string
    fireEvent.change(input, { target: { value: "a".repeat(65) } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    expect(screen.getByText("Role cannot exceed 64 characters")).toBeTruthy();
    expect(api.setAgentRole).not.toHaveBeenCalled();

    // Valid custom role
    vi.mocked(api.setAgentRole).mockResolvedValue({
      id: "human",
      display_name: "Human",
      role: "Lead Architect",
    });

    fireEvent.change(input, { target: { value: "Lead Architect" } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(api.setAgentRole).toHaveBeenCalledWith("human", "Lead Architect");
    });
  });

  it("cancels editing without saving", () => {
    render(
      <RoleAssignmentControl
        agentId="grok"
        displayName="Grok"
        currentRole="observer"
        onRoleChanged={vi.fn()}
      />
    );

    fireEvent.click(screen.getByRole("button", { name: /change/i }));
    expect(screen.getByRole("combobox")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(screen.queryByRole("combobox")).toBeNull();
    expect(screen.getByText("Observer")).toBeTruthy();
    expect(api.setAgentRole).not.toHaveBeenCalled();
  });
});
