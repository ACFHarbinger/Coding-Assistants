import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import DangerTab from "../DangerTab";
import * as api from "../../api";

vi.mock("../../api", () => ({
  listSettingsProfiles: vi.fn(),
  purgeWorkspaceData: vi.fn(),
  purgeWorkspaceMemories: vi.fn(),
  purgeWorkspaceTranscript: vi.fn(),
  removeSettingsProfile: vi.fn(),
}));

const props = {
  workspaceRoot: "/ws/demo",
  busy: false,
  onResetWorkspaceOverrides: vi.fn(),
  onChanged: vi.fn(),
};

function mockProfiles() {
  vi.mocked(api.listSettingsProfiles).mockResolvedValue([
    {
      name: "work",
      provider: "openai",
      model: null,
      base_url: null,
      secret_source: "env_var",
      secret_badge: "none",
    },
  ]);
}

describe("DangerTab (#132)", () => {
  it("renders every backing action with no unavailable stub copy", async () => {
    mockProfiles();
    render(<DangerTab {...props} />);
    await waitFor(() => expect(api.listSettingsProfiles).toHaveBeenCalled());
    for (const label of [
      "Reset Overrides",
      "Purge Transcript",
      "Purge Memories",
      "Purge Workspace Data",
      "Delete Profile",
    ]) {
      expect(screen.getByRole("button", { name: label })).toBeTruthy();
    }
    expect(screen.queryByText(/remain unavailable/i)).toBeNull();
  });

  it("cancelling a purge never invokes the backend", async () => {
    mockProfiles();
    render(<DangerTab {...props} />);
    await waitFor(() => expect(api.listSettingsProfiles).toHaveBeenCalled());

    fireEvent.click(screen.getByRole("button", { name: "Purge Transcript" }));
    fireEvent.click(screen.getByRole("button", { name: "Cancel (Keep Transcript)" }));

    expect(api.purgeWorkspaceTranscript).not.toHaveBeenCalled();
    expect(api.purgeWorkspaceMemories).not.toHaveBeenCalled();
    expect(api.purgeWorkspaceData).not.toHaveBeenCalled();
    expect(props.onChanged).not.toHaveBeenCalled();
  });

  it("confirm stays disabled until the typed target matches, then purges", async () => {
    mockProfiles();
    vi.mocked(api.purgeWorkspaceTranscript).mockResolvedValue(3);
    render(<DangerTab {...props} />);
    await waitFor(() => expect(api.listSettingsProfiles).toHaveBeenCalled());

    fireEvent.click(screen.getByRole("button", { name: "Purge Transcript" }));
    const confirm = screen.getByRole("button", { name: "Confirm Purge" });
    expect((confirm as HTMLButtonElement).disabled).toBe(true);

    fireEvent.change(screen.getByPlaceholderText("demo"), { target: { value: "demo " } });
    expect((confirm as HTMLButtonElement).disabled).toBe(false);

    fireEvent.click(confirm);
    await waitFor(() =>
      expect(api.purgeWorkspaceTranscript).toHaveBeenCalledWith("/ws/demo"),
    );
    expect(await screen.findByText(/Purged 3 transcript messages for demo/)).toBeTruthy();
    expect(props.onChanged).toHaveBeenCalled();
  });

  it("profile deletion types the profile name, not the workspace", async () => {
    mockProfiles();
    vi.mocked(api.removeSettingsProfile).mockResolvedValue([]);
    render(<DangerTab {...props} />);
    await waitFor(() => expect(api.listSettingsProfiles).toHaveBeenCalled());

    fireEvent.change(screen.getByLabelText("Profile"), { target: { value: "work" } });
    fireEvent.click(screen.getByRole("button", { name: "Delete Profile" }));
    const confirm = screen.getByRole("button", { name: "Confirm Delete" });
    expect((confirm as HTMLButtonElement).disabled).toBe(true);

    fireEvent.change(screen.getByPlaceholderText("work"), { target: { value: "work" } });
    fireEvent.click(confirm);
    await waitFor(() => expect(api.removeSettingsProfile).toHaveBeenCalledWith("work"));
    expect(await screen.findByText(/Profile work deleted/)).toBeTruthy();
  });
});
