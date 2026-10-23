import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import ChannelsTab from "../ChannelsTab";

const mockInvoke = vi.fn(async (cmd: string, args?: Record<string, unknown>) => {
  if (cmd === "claude_channel_list_workspaces") {
    return [{ workspace: "/ws/one", display_name: "Workspace One" }];
  }
  if (cmd === "hub_is_channel_workspace_connected") return false;
  if (cmd === "hub_connect_channel_workspace") return { ok: true, pid: 1234, detail: "started" };
  if (cmd === "hub_rename_channel_workspace") return null;
  if (cmd === "hub_delete_channel_workspace") return null;
  if (cmd === "hub_grok_leader_status") return { connected: false, detail: "no leader" };
  if (cmd === "hub_list_grok_sessions") return [];
  void args;
  return [];
});

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => (mockInvoke as unknown as (...a: unknown[]) => unknown)(...args),
}));

describe("ChannelsTab (standalone Channels panel)", () => {
  beforeEach(() => {
    (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
    mockInvoke.mockClear();
  });

  it("loads and renders workspaces on mount", async () => {
    render(<ChannelsTab />);
    await waitFor(() => expect(screen.getByText("/ws/one")).toBeTruthy());
    expect(screen.getByDisplayValue("Workspace One")).toBeTruthy();
    expect(mockInvoke).toHaveBeenCalledWith("claude_channel_list_workspaces", undefined);
  });

  it("renames a workspace via the rename button", async () => {
    render(<ChannelsTab />);
    await waitFor(() => expect(screen.getByText("/ws/one")).toBeTruthy());

    const input = screen.getByDisplayValue("Workspace One");
    fireEvent.change(input, { target: { value: "Renamed" } });
    fireEvent.click(screen.getByRole("button", { name: "Rename" }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("hub_rename_channel_workspace", {
        workspace: "/ws/one",
        displayName: "Renamed",
      });
    });
  });

  it("deletes a workspace via the remove button", async () => {
    render(<ChannelsTab />);
    await waitFor(() => expect(screen.getByText("/ws/one")).toBeTruthy());

    fireEvent.click(screen.getByRole("button", { name: "Remove" }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("hub_delete_channel_workspace", { workspace: "/ws/one" });
    });
  });

  it("connects a workspace via the connect button", async () => {
    render(<ChannelsTab />);
    await waitFor(() => expect(screen.getByText("/ws/one")).toBeTruthy());

    fireEvent.click(screen.getByRole("button", { name: "Connect" }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("hub_connect_channel_workspace", { workspace: "/ws/one" });
    });
    expect(await screen.findByText(/Session connected/i)).toBeTruthy();
  });
});
