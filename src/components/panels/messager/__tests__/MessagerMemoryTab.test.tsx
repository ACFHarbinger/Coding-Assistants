import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import MessagerMemoryTab from "../MessagerMemoryTab";

const baseMemory = {
  id: "mem0000-aaaa-bbbb-cccc-dddddddddddd",
  scope: "workspace",
  tier: "short_term",
  agent_id: "claude",
  title: "Existing memory",
  body: "Some body text",
  tags_json: "[]",
  created_at: "2026-01-01T00:00:00Z",
  stale: false,
};

const mockInvoke = vi.fn(async (cmd: string, args?: Record<string, unknown>) => {
  if (cmd === "hub_list_agents") {
    return [
      { id: "human", display_name: "Human", team_member: true },
      { id: "grok", display_name: "Grok", team_member: true },
    ];
  }
  if (cmd === "hub_list_memories") return [baseMemory];
  if (cmd === "hub_write_memory") return null;
  if (cmd === "hub_update_memory") return null;
  if (cmd === "hub_search_memories") return [baseMemory];
  if (cmd === "hub_search_memories_hybrid") return [{ ...baseMemory, score: 0.9 }];
  if (cmd === "hub_reindex_memory_vectors") return 3;
  void args;
  return [];
});

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => (mockInvoke as unknown as (...a: unknown[]) => unknown)(...args),
}));

Object.assign(navigator, {
  clipboard: { writeText: vi.fn().mockResolvedValue(undefined) },
});

describe("MessagerMemoryTab (Chat & Memory subtab)", () => {
  beforeEach(() => {
    (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
    mockInvoke.mockClear();
  });

  it("loads memories on mount", async () => {
    render(<MessagerMemoryTab workspacePath="/ws" />);
    await waitFor(() => expect(screen.getByText("Existing memory")).toBeTruthy());
  });

  it("writes a new memory", async () => {
    render(<MessagerMemoryTab workspacePath="/ws" />);
    await waitFor(() => expect(screen.getByText("Existing memory")).toBeTruthy());

    fireEvent.change(screen.getByPlaceholderText("Memory body…"), { target: { value: "New memory body" } });
    fireEvent.click(screen.getByRole("button", { name: "Save to hub" }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("hub_write_memory", {
        args: {
          tier: "short_term",
          scope: "workspace",
          agent: "grok",
          workspace: null,
          title: null,
          body: "New memory body",
          tags: [],
        },
      });
    });
  });

  it("runs an exact search", async () => {
    render(<MessagerMemoryTab workspacePath="/ws" />);
    await waitFor(() => expect(screen.getByText("Existing memory")).toBeTruthy());

    fireEvent.click(screen.getByRole("button", { name: "🔤 Exact" }));
    fireEvent.change(screen.getByPlaceholderText("Exact text search…"), { target: { value: "body" } });
    fireEvent.click(screen.getByRole("button", { name: "Search" }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("hub_search_memories", { query: "body" });
    });
  });

  it("edits an existing memory", async () => {
    render(<MessagerMemoryTab workspacePath="/ws" />);
    await waitFor(() => expect(screen.getByText("Existing memory")).toBeTruthy());

    fireEvent.click(screen.getByRole("button", { name: "Edit" }));
    const editBody = screen.getByDisplayValue("Some body text");
    fireEvent.change(editBody, { target: { value: "Updated body" } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("hub_update_memory", {
        args: { id: baseMemory.id, title: "Existing memory", body: "Updated body", tags: null },
      });
    });
  });

  it("copies a memory reference to the clipboard instead of inserting into a composer", async () => {
    render(<MessagerMemoryTab workspacePath="/ws" />);
    await waitFor(() => expect(screen.getByText("Existing memory")).toBeTruthy());

    fireEvent.click(screen.getByRole("button", { name: "Copy reference" }));

    await waitFor(() => {
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith(`[Memory #${baseMemory.id.slice(0, 8)}]`);
    });
  });

  it("applies an initial search query passed in from a chat jump-to-memory link", async () => {
    render(<MessagerMemoryTab workspacePath="/ws" initialSearchQuery="mem0000" />);
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        "hub_search_memories_hybrid",
        expect.objectContaining({ query: "mem0000" }),
      );
    });
  });
});
