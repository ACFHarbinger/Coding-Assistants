import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { TeamProfilesSection } from "../agents/TeamProfilesSection";
import * as api from "../../api";
import type { HubAgent } from "../../../../app/hubState";

vi.mock("../../api", () => ({
  listHubAgents: vi.fn(),
  setAgentDisplayName: vi.fn(),
}));

const mockAgents: HubAgent[] = [
  {
    id: "human",
    display_name: "Human",
    team_member: true,
    avatar_attachment_id: null,
  },
  {
    id: "claude",
    display_name: "Claude Code",
    team_member: true,
    avatar_attachment_id: "att-claude",
  },
  {
    id: "gemini",
    display_name: "Gemini / Antigravity",
    team_member: false,
    avatar_attachment_id: null,
  },
];

describe("TeamProfilesSection (U20 / #314)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(api.listHubAgents).mockResolvedValue(mockAgents);
  });

  it("renders all roster identities with avatars and display names", async () => {
    render(<TeamProfilesSection />);
    await waitFor(() => expect(api.listHubAgents).toHaveBeenCalled());

    expect(screen.getByText("Team & Identity Profiles")).toBeTruthy();
    expect(screen.getByText("Human")).toBeTruthy();
    expect(screen.getByText("Claude Code")).toBeTruthy();
    expect(screen.getByText("Gemini / Antigravity")).toBeTruthy();

    expect(screen.getByText("@human")).toBeTruthy();
    expect(screen.getByText("@claude")).toBeTruthy();
    expect(screen.getByText("@gemini")).toBeTruthy();

    expect(screen.getByText("You (Developer)")).toBeTruthy();
    expect(screen.getAllByText("Enrolled Member").length).toBe(2);
  });

  it("allows renaming an identity, rejecting collisions and empty names", async () => {
    const onChanged = vi.fn();
    render(<TeamProfilesSection onChanged={onChanged} />);
    await waitFor(() => expect(api.listHubAgents).toHaveBeenCalled());

    // Click rename on human
    const renameButtons = screen.getAllByRole("button", { name: /rename/i });
    fireEvent.click(renameButtons[0]);

    const input = screen.getByLabelText("Display name for human") as HTMLInputElement;
    expect(input.value).toBe("Human");

    // Try empty name
    fireEvent.change(input, { target: { value: "   " } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(screen.getByText("Display name cannot be empty.")).toBeTruthy();
    expect(api.setAgentDisplayName).not.toHaveBeenCalled();

    // Try collision with Claude Code
    fireEvent.change(input, { target: { value: "claude code" } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(
      screen.getByText('Display name "claude code" is already in use by @claude.'),
    ).toBeTruthy();
    expect(api.setAgentDisplayName).not.toHaveBeenCalled();

    // Valid rename
    const updatedHuman: HubAgent = {
      ...mockAgents[0],
      display_name: "Alice Developer",
    };
    vi.mocked(api.setAgentDisplayName).mockResolvedValue(updatedHuman);

    fireEvent.change(input, { target: { value: "Alice Developer" } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(api.setAgentDisplayName).toHaveBeenCalledWith("human", "Alice Developer");
    });
    expect(onChanged).toHaveBeenCalled();
  });

  it("can cancel editing without saving", async () => {
    render(<TeamProfilesSection />);
    await waitFor(() => expect(api.listHubAgents).toHaveBeenCalled());

    const renameButtons = screen.getAllByRole("button", { name: /rename/i });
    fireEvent.click(renameButtons[1]); // Claude

    const input = screen.getByLabelText("Display name for claude") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "Something Else" } });

    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));

    // Input is dismissed, original name persists
    expect(screen.queryByLabelText("Display name for claude")).toBeNull();
    expect(screen.getByText("Claude Code")).toBeTruthy();
    expect(api.setAgentDisplayName).not.toHaveBeenCalled();
  });
});
