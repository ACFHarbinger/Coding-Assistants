import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import HarnessTerminalGrid from "../HarnessTerminalGrid";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string, args?: any) => {
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
  }),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock("../harness/EmbeddedTerminal", () => ({
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
    render(<HarnessTerminalGrid workspace="/test/workspace" />);

    // Launch Grok
    fireEvent.click(screen.getByText("+ Grok"));
    expect(await screen.findByText("Grok")).toBeInTheDocument();

    // Click Reset grid
    const resetBtn = screen.getByText("Reset grid");
    fireEvent.click(resetBtn);

    expect(await screen.findByText("No active harness terminals in grid")).toBeInTheDocument();
  });
});
