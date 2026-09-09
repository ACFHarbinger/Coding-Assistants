import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import HarnessTerminalGrid from "../HarnessTerminalGrid";

const defaultInvokeHandler = (cmd: string, args?: any) => {
  if (cmd === "hub_list_harness_sessions") {
    return Promise.resolve([]);
  }
  if (cmd === "hub_relaunch_harness_embedded") {
    return Promise.resolve({
      harness: args?.harness || "claude",
      sessionId: `sid-${args?.harness || "claude"}`,
      detail: "Connected",
      killedPid: null,
    });
  }
  if (cmd === "pty_session_status") {
    return Promise.resolve({
      found: true,
      running: true,
      exited: false,
      exitDetail: null,
      outputTailB64: "",
    });
  }
  if (cmd === "pty_kill") {
    return Promise.resolve(null);
  }
  return Promise.resolve(null);
};

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string, args?: any) => defaultInvokeHandler(cmd, args)),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock("../../harness/EmbeddedTerminal", () => ({
  default: ({ sessionId }: { sessionId: string }) => (
    <div data-testid={`embedded-terminal-${sessionId}`}>Terminal {sessionId}</div>
  ),
}));

// Mock ResizeObserver for jsdom
class MockResizeObserver {
  callback: (entries: any[]) => void;
  constructor(cb: (entries: any[]) => void) {
    this.callback = cb;
  }
  observe(el: Element) {
    this.callback([{ target: el, contentRect: { width: 1000, height: 600 } }]);
  }
  unobserve() {}
  disconnect() {}
}

const storageStore: Record<string, string> = {};

beforeEach(() => {
  (window as any).ResizeObserver = MockResizeObserver;
  (window as any).__TAURI_INTERNALS__ = {};
  Element.prototype.setPointerCapture = vi.fn();
  Element.prototype.releasePointerCapture = vi.fn();
  Element.prototype.getBoundingClientRect = vi.fn().mockReturnValue({
    left: 0,
    top: 0,
    width: 1000,
    height: 600,
    right: 1000,
    bottom: 600,
    x: 0,
    y: 0,
  });
  for (const k of Object.keys(storageStore)) {
    delete storageStore[k];
  }
  Object.defineProperty(window, "localStorage", {
    value: {
      getItem: (k: string) => storageStore[k] ?? null,
      setItem: (k: string, v: string) => {
        storageStore[k] = String(v);
      },
      removeItem: (k: string) => {
        delete storageStore[k];
      },
      clear: () => {
        for (const k of Object.keys(storageStore)) delete storageStore[k];
      },
      key: (i: number) => Object.keys(storageStore)[i] ?? null,
      get length() {
        return Object.keys(storageStore).length;
      },
    },
    writable: true,
    configurable: true,
  });
  vi.clearAllMocks();
  vi.mocked(invoke).mockImplementation((cmd: string, args?: any) => defaultInvokeHandler(cmd, args));
});

describe("HarnessTerminalGrid", () => {
  it("renders empty state with 6 launch buttons when no terminals are open", () => {
    render(<HarnessTerminalGrid workspace="/test/workspace" />);

    expect(screen.getByText("No active harness terminals in grid")).toBeInTheDocument();
    expect(screen.getByText("Launch Grok")).toBeInTheDocument();
    expect(screen.getByText("Launch Codex")).toBeInTheDocument();
    expect(screen.getByText("Launch Claude")).toBeInTheDocument();
    expect(screen.getByText("Launch Gemini")).toBeInTheDocument();
    expect(screen.getByText("Launch Muse")).toBeInTheDocument();
    expect(screen.getByText("Launch Cursor")).toBeInTheDocument();
  });

  it("launches a harness when clicking add pane button", async () => {
    const tauriCore = await import("@tauri-apps/api/core");
    render(<HarnessTerminalGrid workspace="/test/workspace" />);

    const launchBtn = screen.getByText("+ Claude");
    fireEvent.click(launchBtn);

    // Pane title bar should now be visible
    expect(await screen.findByText("Claude")).toBeInTheDocument();

    expect(tauriCore.invoke).toHaveBeenCalledWith("hub_relaunch_harness_embedded", {
      harness: "claude",
      workspace: "/test/workspace",
      existingPid: null,
    });
  });

  it("closes a pane and removes it from layout when clicking close button", async () => {
    const tauriCore = await import("@tauri-apps/api/core");
    render(<HarnessTerminalGrid workspace="/test/workspace" />);

    // Launch Claude
    fireEvent.click(screen.getByText("+ Claude"));
    expect(await screen.findByText("Claude")).toBeInTheDocument();

    // Close Claude pane
    const closeBtn = screen.getByRole("button", { name: "Close claude pane" });
    fireEvent.click(closeBtn);

    // pty_kill should be invoked
    expect(tauriCore.invoke).toHaveBeenCalledWith("pty_kill", { sessionId: "sid-claude" });

    // Grid should return to empty state
    expect(await screen.findByText("No active harness terminals in grid")).toBeInTheDocument();
  });

  it("resets grid layout when clicking reset grid button", async () => {
    const tauriCore = await import("@tauri-apps/api/core");
    render(<HarnessTerminalGrid workspace="/test/workspace" />);

    // Launch Grok
    fireEvent.click(screen.getByText("+ Grok"));
    expect(await screen.findByText("Grok")).toBeInTheDocument();

    // Click Reset grid
    const resetBtn = screen.getByText("Reset grid");
    fireEvent.click(resetBtn);

    expect(tauriCore.invoke).toHaveBeenCalledWith("pty_kill", { sessionId: "sid-grok" });
    expect(await screen.findByText("No active harness terminals in grid")).toBeInTheDocument();
  });

  it("restores only persisted panes whose deterministic PTY session still exists", async () => {
    storageStore["ca.terminalGrid.layout./test/workspace"] = JSON.stringify({
      type: "split",
      id: "split-1",
      direction: "row",
      ratio: 0.5,
      first: { type: "leaf", id: "live", harness: "claude" },
      second: { type: "leaf", id: "stale", harness: "grok" },
    });
    const tauriCore = await import("@tauri-apps/api/core");
    vi.mocked(tauriCore.invoke).mockImplementation((cmd: string, args?: any) => {
      if (cmd === "pty_session_status") {
        return Promise.resolve({
          found: args?.sessionId === "harness-terminal:claude:/test/workspace",
          running: true,
          exited: false,
          exitDetail: null,
          outputTailB64: "",
        });
      }
      return Promise.resolve(null);
    });

    render(<HarnessTerminalGrid workspace="/test/workspace" />);

    expect(await screen.findByTestId("embedded-terminal-harness-terminal:claude:/test/workspace")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Close grok pane" })).not.toBeInTheDocument();
    expect(tauriCore.invoke).toHaveBeenCalledWith("pty_session_status", {
      sessionId: "harness-terminal:claude:/test/workspace",
    });
  });

  it("maximizes a pane and restores via button or Escape key", async () => {
    render(<HarnessTerminalGrid workspace="/test/workspace" />);

    // Launch Claude and Grok
    fireEvent.click(screen.getByText("+ Claude"));
    expect(await screen.findByText("Claude")).toBeInTheDocument();
    fireEvent.click(screen.getByText("+ Grok"));
    expect(await screen.findByText("Grok")).toBeInTheDocument();

    // Splitter should be visible
    expect(screen.getByRole("separator")).toBeInTheDocument();

    // Click maximize on Claude
    const maxClaudeBtn = screen.getByRole("button", { name: "Maximize claude pane" });
    fireEvent.click(maxClaudeBtn);

    // Banner should now be visible
    expect(screen.getByText(/Maximized: Claude/)).toBeInTheDocument();
    // Splitter should be hidden while maximized
    expect(screen.queryByRole("separator")).not.toBeInTheDocument();

    // Press Escape to restore
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByText(/Maximized: Claude/)).not.toBeInTheDocument();
    expect(screen.getByRole("separator")).toBeInTheDocument();

    // Maximize Grok and restore via the restore button
    const maxGrokBtn = screen.getByRole("button", { name: "Maximize grok pane" });
    fireEvent.click(maxGrokBtn);
    expect(screen.getByText(/Maximized: Grok/)).toBeInTheDocument();

    const restoreGrokBtn = screen.getByRole("button", { name: "Restore grok pane" });
    fireEvent.click(restoreGrokBtn);
    expect(screen.queryByText(/Maximized: Grok/)).not.toBeInTheDocument();
  });

  it("handles titlebar drag to center to swap panes", async () => {
    render(<HarnessTerminalGrid workspace="/test/workspace" />);

    // Launch Claude and Grok
    fireEvent.click(screen.getByText("+ Claude"));
    expect(await screen.findByText("Claude")).toBeInTheDocument();
    fireEvent.click(screen.getByText("+ Grok"));
    expect(await screen.findByText("Grok")).toBeInTheDocument();

    const claudeTitle = screen.getByText("Claude");
    const claudeTitleBar = claudeTitle.closest("div")?.parentElement as HTMLElement;
    expect(claudeTitleBar).toBeTruthy();

    // Pointer down on Claude's titlebar
    fireEvent.pointerDown(claudeTitleBar, { clientX: 250, clientY: 20, button: 0, pointerId: 1 });

    // Drag into Grok's center (x: 750, y: 300)
    fireEvent.pointerMove(claudeTitleBar, { clientX: 750, clientY: 300, pointerId: 1 });

    // Drop zone overlay should appear with swap label
    expect(await screen.findByTestId("drop-zone-overlay")).toBeInTheDocument();
    expect(screen.getByText("Swap with Grok")).toBeInTheDocument();

    // Pointer up to complete the swap
    fireEvent.pointerUp(claudeTitleBar, { clientX: 750, clientY: 300, pointerId: 1 });

    // Drop zone overlay should disappear
    expect(screen.queryByTestId("drop-zone-overlay")).not.toBeInTheDocument();
  });

  it("handles titlebar drag to bottom edge to re-split panes", async () => {
    render(<HarnessTerminalGrid workspace="/test/workspace" />);

    // Launch Claude and Grok
    fireEvent.click(screen.getByText("+ Claude"));
    expect(await screen.findByText("Claude")).toBeInTheDocument();
    fireEvent.click(screen.getByText("+ Grok"));
    expect(await screen.findByText("Grok")).toBeInTheDocument();

    const claudeTitle = screen.getByText("Claude");
    const claudeTitleBar = claudeTitle.closest("div")?.parentElement as HTMLElement;
    expect(claudeTitleBar).toBeTruthy();

    // Pointer down on Claude
    fireEvent.pointerDown(claudeTitleBar, { clientX: 250, clientY: 20, button: 0, pointerId: 1 });

    // Drag into Grok's bottom edge (x: 750, y: 580)
    fireEvent.pointerMove(claudeTitleBar, { clientX: 750, clientY: 580, pointerId: 1 });

    // Drop zone overlay should show Dock below Grok
    expect(await screen.findByTestId("drop-zone-overlay")).toBeInTheDocument();
    expect(screen.getByText("Dock below Grok")).toBeInTheDocument();

    // Pointer up to complete re-split
    fireEvent.pointerUp(claudeTitleBar, { clientX: 750, clientY: 580, pointerId: 1 });

    expect(screen.queryByTestId("drop-zone-overlay")).not.toBeInTheDocument();
    const separator = screen.getByRole("separator");
    expect(separator).toHaveAttribute("aria-orientation", "horizontal");
  });

  it("drag-resizes canvas via corner handle and persists dimensions to localStorage", async () => {
    render(<HarnessTerminalGrid workspace="/test/workspace" />);

    fireEvent.click(screen.getByText("+ Claude"));
    expect(await screen.findByText("Claude")).toBeInTheDocument();

    const cornerHandle = screen.getByTestId("canvas-resize-corner");
    expect(cornerHandle).toBeInTheDocument();

    fireEvent.pointerDown(cornerHandle, { clientX: 1000, clientY: 600, button: 0, pointerId: 1 });
    fireEvent.pointerMove(cornerHandle, { clientX: 850, clientY: 700, pointerId: 1 });
    fireEvent.pointerUp(cornerHandle, { pointerId: 1 });

    expect(screen.getByText("Grid: 850×700px")).toBeInTheDocument();
    expect(screen.getByText("Fit to window")).toBeInTheDocument();

    const saved = JSON.parse(storageStore["ca.terminalGrid.canvasSize./test/workspace"] || "{}");
    expect(saved).toEqual({ width: 850, height: 700 });
  });

  it("restores valid canvas size from localStorage on mount and clears via Fit to window button", async () => {
    storageStore["ca.terminalGrid.canvasSize./test/workspace"] = JSON.stringify({
      width: 800,
      height: 550,
    });

    render(<HarnessTerminalGrid workspace="/test/workspace" />);

    fireEvent.click(screen.getByText("+ Grok"));
    expect(await screen.findByText("Grok")).toBeInTheDocument();

    expect(screen.getByText("Grid: 800×550px")).toBeInTheDocument();
    const fitBtn = screen.getByText("Fit to window");
    expect(fitBtn).toBeInTheDocument();

    fireEvent.click(fitBtn);

    expect(storageStore["ca.terminalGrid.canvasSize./test/workspace"]).toBeUndefined();
    expect(screen.queryByText("Fit to window")).not.toBeInTheDocument();
    expect(screen.queryByText("Grid: 800×550px")).not.toBeInTheDocument();
  });

  it("resizes width and height independently using right and bottom handles, enforcing minimum boundaries", async () => {
    render(<HarnessTerminalGrid workspace="/test/workspace" />);

    fireEvent.click(screen.getByText("+ Claude"));
    expect(await screen.findByText("Claude")).toBeInTheDocument();

    const rightHandle = screen.getByTestId("canvas-resize-right");
    const bottomHandle = screen.getByTestId("canvas-resize-bottom");

    // Drag right handle past minimum (clamped to min 360)
    fireEvent.pointerDown(rightHandle, { clientX: 1000, clientY: 300, button: 0, pointerId: 1 });
    fireEvent.pointerMove(rightHandle, { clientX: 100, clientY: 300, pointerId: 1 });
    fireEvent.pointerUp(rightHandle, { pointerId: 1 });

    expect(screen.getByText("Grid: 360×600px")).toBeInTheDocument();

    // Drag bottom handle past minimum (clamped to min 280)
    fireEvent.pointerDown(bottomHandle, { clientX: 300, clientY: 600, button: 0, pointerId: 1 });
    fireEvent.pointerMove(bottomHandle, { clientX: 300, clientY: 100, pointerId: 1 });
    fireEvent.pointerUp(bottomHandle, { pointerId: 1 });

    expect(screen.getByText("Grid: 360×280px")).toBeInTheDocument();
  });
});

