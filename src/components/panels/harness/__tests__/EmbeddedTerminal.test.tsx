import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import EmbeddedTerminal from "../EmbeddedTerminal";

// Track mock instances
interface MockTerm {
  options: any;
  element: HTMLElement | null;
  addons: any[];
  written: (string | Uint8Array)[];
  disposed: boolean;
}

const mockTerminalInstances: MockTerm[] = [];
const mockFitAddonInstances: Array<{ fitCalls: number; disposed: boolean }> = [];
const mockWebglInstances: Array<{
  contextLossCallbacks: Array<() => void>;
  disposed: boolean;
}> = [];

let webglConstructorShouldThrow = false;
let webglActivateShouldThrow = false;

vi.mock("@xterm/xterm", () => {
  return {
    Terminal: class MockTerminal {
      options: any;
      element: HTMLElement | null = null;
      addons: any[] = [];
      written: (string | Uint8Array)[] = [];
      disposed = false;
      rows = 24;
      cols = 80;
      buffer = { active: { type: "normal" } };

      constructor(options: any) {
        this.options = options;
        mockTerminalInstances.push(this);
      }

      loadAddon(addon: any) {
        this.addons.push(addon);
        if (typeof addon.activate === "function") {
          addon.activate(this);
        }
      }

      open(el: HTMLElement) {
        this.element = el;
      }

      attachCustomWheelEventHandler() {}

      write(data: string | Uint8Array) {
        this.written.push(data);
      }

      onData() {
        return { dispose: vi.fn() };
      }

      dispose() {
        this.disposed = true;
      }
    },
  };
});

vi.mock("@xterm/addon-fit", () => {
  return {
    FitAddon: class MockFitAddon {
      fitCalls = 0;
      disposed = false;

      constructor() {
        mockFitAddonInstances.push(this);
      }

      activate() {}

      fit() {
        this.fitCalls += 1;
      }

      dispose() {
        this.disposed = true;
      }
    },
  };
});

vi.mock("@xterm/addon-webgl", () => {
  return {
    WebglAddon: class MockWebglAddon {
      contextLossCallbacks: Array<() => void> = [];
      disposed = false;

      constructor() {
        if (webglConstructorShouldThrow) {
          throw new Error("WebGL context creation failed");
        }
        mockWebglInstances.push(this);
      }

      activate() {
        if (webglActivateShouldThrow) {
          throw new Error("Failed to activate WebglAddon");
        }
      }

      onContextLoss(cb: () => void) {
        this.contextLossCallbacks.push(cb);
        return { dispose: vi.fn() };
      }

      dispose() {
        this.disposed = true;
      }
    },
  };
});

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

class MockResizeObserver {
  callback: (entries: any[]) => void;
  constructor(cb: (entries: any[]) => void) {
    this.callback = cb;
  }
  observe() {}
  unobserve() {}
  disconnect() {}
}

describe("EmbeddedTerminal", () => {
  beforeEach(() => {
    (window as any).__TAURI_INTERNALS__ = {};
    (window as any).ResizeObserver = MockResizeObserver;
    mockTerminalInstances.length = 0;
    mockFitAddonInstances.length = 0;
    mockWebglInstances.length = 0;
    webglConstructorShouldThrow = false;
    webglActivateShouldThrow = false;

    Object.defineProperty(HTMLElement.prototype, "clientWidth", {
      configurable: true,
      value: 800,
    });
    Object.defineProperty(HTMLElement.prototype, "clientHeight", {
      configurable: true,
      value: 600,
    });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("loads WebglAddon and uses Linux-first monospace font stack on normal start", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === "pty_session_status") {
        return Promise.resolve({
          found: true,
          running: true,
          exited: false,
          exitDetail: null,
          outputTailB64: "",
        });
      }
      return Promise.resolve(null);
    });

    render(<EmbeddedTerminal sessionId="test-session-1" />);

    await waitFor(() => {
      expect(mockTerminalInstances.length).toBe(1);
    });

    const term = mockTerminalInstances[0];
    expect(term.options.fontFamily).toContain('"Noto Sans Mono"');
    expect(term.options.fontFamily).toContain('"DejaVu Sans Mono"');
    expect(term.options.fontFamily).toContain('"Liberation Mono"');
    expect(term.options.fontFamily).toContain('"Ubuntu Mono"');
    expect(term.options.fontFamily).toContain('"Hack"');
    expect(term.options.fontFamily).toContain("ui-monospace");

    expect(mockWebglInstances.length).toBe(1);
    expect(mockWebglInstances[0].contextLossCallbacks.length).toBe(1);
    expect(mockFitAddonInstances.length).toBe(1);
    expect(mockFitAddonInstances[0].fitCalls).toBeGreaterThanOrEqual(1);
  });

  it("swallows WebglAddon constructor throw and falls back to DOM renderer", async () => {
    webglConstructorShouldThrow = true;
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === "pty_session_status") {
        return Promise.resolve({
          found: true,
          running: true,
          exited: false,
          exitDetail: null,
          outputTailB64: "",
        });
      }
      return Promise.resolve(null);
    });

    const onError = vi.fn();
    render(<EmbeddedTerminal sessionId="test-session-webgl-fail" onError={onError} />);

    await waitFor(() => {
      expect(mockTerminalInstances.length).toBe(1);
    });

    // WebGL instance was not created, but terminal still initialized and rendered cleanly
    expect(mockWebglInstances.length).toBe(0);
    expect(mockFitAddonInstances.length).toBe(1);
    expect(mockFitAddonInstances[0].fitCalls).toBeGreaterThanOrEqual(1);
    // onError shouldn't be called for WebGL fallback; it degrades quietly to DOM
    expect(onError).not.toHaveBeenCalled();
  });

  it("swallows WebglAddon activate throw and continues in DOM mode", async () => {
    webglActivateShouldThrow = true;
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === "pty_session_status") {
        return Promise.resolve({
          found: true,
          running: true,
          exited: false,
          exitDetail: null,
          outputTailB64: "",
        });
      }
      return Promise.resolve(null);
    });

    render(<EmbeddedTerminal sessionId="test-session-webgl-activate-fail" />);

    await waitFor(() => {
      expect(mockTerminalInstances.length).toBe(1);
    });

    expect(mockFitAddonInstances.length).toBe(1);
    expect(mockFitAddonInstances[0].fitCalls).toBeGreaterThanOrEqual(1);
    expect(mockWebglInstances[0].disposed).toBe(true);
  });

  it("disposes WebglAddon on context loss to degrade to DOM renderer", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === "pty_session_status") {
        return Promise.resolve({
          found: true,
          running: true,
          exited: false,
          exitDetail: null,
          outputTailB64: "",
        });
      }
      return Promise.resolve(null);
    });

    render(<EmbeddedTerminal sessionId="test-session-ctx-loss" />);

    await waitFor(() => {
      expect(mockWebglInstances.length).toBe(1);
    });

    const webgl = mockWebglInstances[0];
    expect(webgl.disposed).toBe(false);

    // Simulate context loss
    expect(webgl.contextLossCallbacks.length).toBe(1);
    webgl.contextLossCallbacks[0]();

    expect(webgl.disposed).toBe(true);
  });

  it("disposes WebglAddon and terminal on unmount", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === "pty_session_status") {
        return Promise.resolve({
          found: true,
          running: true,
          exited: false,
          exitDetail: null,
          outputTailB64: "",
        });
      }
      return Promise.resolve(null);
    });

    const { unmount } = render(<EmbeddedTerminal sessionId="test-session-cleanup" />);

    await waitFor(() => {
      expect(mockWebglInstances.length).toBe(1);
    });

    const webgl = mockWebglInstances[0];
    const term = mockTerminalInstances[0];
    expect(webgl.disposed).toBe(false);
    expect(term.disposed).toBe(false);

    unmount();

    expect(webgl.disposed).toBe(true);
    expect(term.disposed).toBe(true);
  });

  it("awaits document.fonts.ready before fit() when fonts API is present", async () => {
    let fontReadyResolved = false;
    const fontReadyPromise = new Promise<void>((resolve) => {
      setTimeout(() => {
        fontReadyResolved = true;
        resolve();
      }, 20);
    });

    Object.defineProperty(document, "fonts", {
      configurable: true,
      value: { ready: fontReadyPromise },
    });

    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === "pty_session_status") {
        return Promise.resolve({
          found: true,
          running: true,
          exited: false,
          exitDetail: null,
          outputTailB64: "",
        });
      }
      return Promise.resolve(null);
    });

    render(<EmbeddedTerminal sessionId="test-session-fonts" />);

    await waitFor(() => {
      expect(mockFitAddonInstances.length).toBe(1);
      expect(mockFitAddonInstances[0].fitCalls).toBeGreaterThanOrEqual(1);
    });

    expect(fontReadyResolved).toBe(true);

    // Clean up document.fonts
    delete (document as any).fonts;
  });

  it("handles missing terminal session cleanly", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === "pty_session_status") {
        return Promise.resolve({
          found: false,
          running: false,
          exited: false,
          exitDetail: null,
          outputTailB64: "",
        });
      }
      return Promise.resolve(null);
    });

    const onError = vi.fn();
    render(<EmbeddedTerminal sessionId="missing-session" onError={onError} />);

    expect(await screen.findByText("Terminal session not found")).toBeInTheDocument();
    expect(onError).toHaveBeenCalledWith(
      expect.stringContaining("Terminal session not found"),
    );
  });

  it("handles fast exit session showing exited badge and triggering onExit", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === "pty_session_status") {
        return Promise.resolve({
          found: true,
          running: false,
          exited: true,
          exitDetail: "Process exited with code 0",
          outputTailB64: btoa("hello exit"),
        });
      }
      return Promise.resolve(null);
    });

    const onExit = vi.fn();
    render(<EmbeddedTerminal sessionId="exited-session" onExit={onExit} />);

    expect(await screen.findByText("exited")).toBeInTheDocument();
    expect(onExit).toHaveBeenCalledWith("Process exited with code 0");
  });
});
