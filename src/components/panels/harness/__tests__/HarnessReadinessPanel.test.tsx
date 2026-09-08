import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import HarnessReadinessPanel from "../HarnessReadinessPanel";
import {
  HARNESS_PREREQUISITES,
  LIVE_TERMINAL_HARNESSES,
  sessionAliases,
} from "../types";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string) => {
    if (cmd === "hub_list_harness_sessions") {
      return Promise.resolve([
        {
          harness: "muse",
          workspace: "/test/ws",
          disk_session_id: "12345678-1234-1234-1234-123456789abc",
          leader_socket: null,
          registered_at: "2026-09-08T12:00:00Z",
          mode: "managed",
          state: "ready",
          managed_pid: 1234,
          writer_owner: "muse",
          writer_acquired_at: "2026-09-08T12:00:00Z",
        },
        {
          harness: "cursor",
          workspace: "/test/ws",
          disk_session_id: "cursor-chat-xyz",
          leader_socket: null,
          registered_at: "2026-09-08T12:05:00Z",
          mode: "observed",
          state: "ready",
          managed_pid: null,
          writer_owner: null,
          writer_acquired_at: null,
        },
      ]);
    }
    if (cmd === "hub_grok_leader_status") {
      return Promise.resolve({
        running: false,
        pid: null,
        socketPath: null,
        activeLeaderPid: null,
        leader_live: false,
        detail: "No leader session",
      });
    }
    if (cmd === "hub_grok_list_live_sessions") {
      return Promise.resolve([]);
    }
    return Promise.resolve(null);
  }),
}));

describe("HarnessReadinessPanel with muse and cursor (#278)", () => {
  beforeEach(() => {
    (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
  });

  it("includes muse and cursor in prerequisites and live terminal harness types", () => {
    expect(LIVE_TERMINAL_HARNESSES).toContain("muse");
    expect(LIVE_TERMINAL_HARNESSES).toContain("cursor");

    expect(HARNESS_PREREQUISITES.muse).toContain("muse exec --session-id");
    expect(HARNESS_PREREQUISITES.cursor).toContain("agent -p");

    expect(sessionAliases("muse")).toEqual(["muse"]);
    expect(sessionAliases("cursor")).toEqual(["cursor"]);
  });

  it("renders muse and cursor in the harness provider dropdown and registered session rows", async () => {
    render(<HarnessReadinessPanel workspace="/test/ws" />);

    // Check dropdown options
    const select = screen.getByRole("combobox") as HTMLSelectElement;
    const optionValues = Array.from(select.options).map((opt) => opt.value);
    expect(optionValues).toContain("grok");
    expect(optionValues).toContain("chat");
    expect(optionValues).toContain("claude");
    expect(optionValues).toContain("gemini");
    expect(optionValues).toContain("muse");
    expect(optionValues).toContain("cursor");

    // Wait for async sessions to load and render session cards
    expect(await screen.findByText(/12345678-1234-1234-1234-123456789abc/)).toBeInTheDocument();
    expect(await screen.findByText(/cursor-chat-xyz/)).toBeInTheDocument();

    // Now both select option and session card headers are rendered
    const museElements = screen.getAllByText("muse");
    expect(museElements.length).toBeGreaterThanOrEqual(2);

    const cursorElements = screen.getAllByText("cursor");
    expect(cursorElements.length).toBeGreaterThanOrEqual(2);
  });

  it("does not require a chat ID for Cursor managed start, but requires UUID for Muse", async () => {
    const { fireEvent } = await import("@testing-library/react");
    render(<HarnessReadinessPanel workspace="/test/ws" />);

    // Wait for initial refresh to finish
    expect(await screen.findByText(/12345678-1234-1234-1234-123456789abc/)).toBeInTheDocument();

    const select = screen.getByRole("combobox") as HTMLSelectElement;
    const startManagedBtn = screen.getByText("Start managed");

    // Select muse without an ID -> should show error requiring UUID
    fireEvent.change(select, { target: { value: "muse" } });
    fireEvent.click(startManagedBtn);
    expect(
      await screen.findByText("Start managed needs a real Muse session UUID. Do not invent a placeholder."),
    ).toBeInTheDocument();
    // Select cursor without an ID -> should not fail with chat ID requirement
    const tauriCore = await import("@tauri-apps/api/core");
    vi.mocked(tauriCore.invoke).mockImplementation((cmd: string) => {
      if (cmd === "hub_start_managed_harness") {
        return Promise.resolve({
          start: { harness: "cursor", pid: 9999, status: "started", detail: "Cursor agent started" },
          registration: {
            harness: "cursor",
            workspace: "/test/ws",
            disk_session_id: "fresh-chat-id",
            leader_socket: null,
            registered_at: "2026-09-08T12:00:00Z",
            mode: "managed",
            state: "ready",
            managed_pid: 9999,
            writer_owner: "cursor",
            writer_acquired_at: "2026-09-08T12:00:00Z",
          },
        });
      }
      if (cmd === "hub_list_harness_sessions") {
        return Promise.resolve([]);
      }
      if (cmd === "hub_grok_leader_status") {
        return Promise.resolve({
          running: false,
          pid: null,
          socketPath: null,
          activeLeaderPid: null,
          leader_live: false,
          detail: "No leader session",
        });
      }
      if (cmd === "hub_grok_list_live_sessions") {
        return Promise.resolve([]);
      }
      return Promise.resolve(null);
    });

    fireEvent.change(select, { target: { value: "cursor" } });
    fireEvent.click(startManagedBtn);

    // It should invoke hub_start_managed_harness and display the success detail
    expect(await screen.findByText("Cursor agent started")).toBeInTheDocument();
  });
});
