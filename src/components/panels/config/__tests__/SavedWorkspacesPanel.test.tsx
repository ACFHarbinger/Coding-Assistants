import { describe, it, expect, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import SavedWorkspacesPanel from "../SavedWorkspacesPanel";
import * as tauriLib from "../../../../lib/tauri";

const sample = {
  id: "ws-1",
  name: "Coding Assistants",
  path: "/tmp/ca-u24",
  created_at: "2026-09-15T00:00:00Z",
  linked_session_id: "sess-1",
  linked_session_name: "Team chat",
};

describe("SavedWorkspacesPanel (U24 / #317)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.spyOn(tauriLib, "isTauriRuntime").mockReturnValue(true);
  });

  it("lists saved workspaces and selecting one applies the path and linked session", async () => {
    vi.spyOn(tauriLib, "invoke").mockResolvedValue([sample]);
    const onSelectPath = vi.fn();
    const onSelectLinkedSession = vi.fn();
    render(
      <SavedWorkspacesPanel
        workDir="/other"
        onSelectPath={onSelectPath}
        onSelectLinkedSession={onSelectLinkedSession}
      />,
    );
    expect(await screen.findByText("Coding Assistants")).toBeInTheDocument();
    expect(screen.getByText("/tmp/ca-u24")).toBeInTheDocument();
    expect(screen.getByText("Team chat: Team chat")).toBeInTheDocument();
    fireEvent.click(screen.getByText("Coding Assistants"));
    expect(onSelectPath).toHaveBeenCalledWith("/tmp/ca-u24");
    expect(onSelectLinkedSession).toHaveBeenCalledWith("sess-1");
  });

  it("saves the current path with an optional name and active session link", async () => {
    const invokeSpy = vi.spyOn(tauriLib, "invoke").mockImplementation(async (cmd) => {
      if (cmd === "hub_list_workspaces") return [];
      if (cmd === "hub_save_workspace") return { ...sample, name: "Mine" };
      return null;
    });
    render(
      <SavedWorkspacesPanel
        workDir="/tmp/ca-u24"
        activeWorkSessionId="sess-1"
        onSelectPath={vi.fn()}
      />,
    );
    fireEvent.change(screen.getByLabelText("New saved workspace name"), {
      target: { value: "Mine" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save current as…" }));
    await waitFor(() => {
      expect(invokeSpy).toHaveBeenCalledWith("hub_save_workspace", {
        id: null,
        name: "Mine",
        path: "/tmp/ca-u24",
        linkSessionId: "sess-1",
      });
    });
  });
});
