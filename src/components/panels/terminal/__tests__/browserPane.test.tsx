import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import HarnessTerminalGrid from "../HarnessTerminalGrid";
import { deserializeLayout, moveLeaf, type LayoutNode } from "../layoutTree";
import { browserPaneUrlError, DEEPSEEK_GUI_HOME } from "../browserUrl";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(null),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock("../../harness/EmbeddedTerminal", () => ({
  default: ({ sessionId }: { sessionId: string }) => <div data-testid={`embedded-terminal-${sessionId}`} />,
}));

class MockResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}

const storage: Record<string, string> = {};

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockResolvedValue(null);
  (window as unknown as { ResizeObserver: typeof MockResizeObserver }).ResizeObserver = MockResizeObserver;
  (window as unknown as { __TAURI_INTERNALS__: object }).__TAURI_INTERNALS__ = {};
  for (const key of Object.keys(storage)) delete storage[key];
  Object.defineProperty(window, "localStorage", {
    configurable: true,
    value: {
      getItem: (key: string) => storage[key] ?? null,
      setItem: (key: string, value: string) => {
        storage[key] = String(value);
      },
      removeItem: (key: string) => {
        delete storage[key];
      },
      clear: () => undefined,
      key: () => null,
      length: 0,
    },
  });
});

describe("DeepSeek GUI browser pane", () => {
  it("accepts the DeepSeek GUI and other public https pages", () => {
    expect(browserPaneUrlError(DEEPSEEK_GUI_HOME)).toBeNull();
    expect(browserPaneUrlError("https://example.com/docs")).toBeNull();
    expect(browserPaneUrlError("http://chat.deepseek.com/")).not.toBeNull();
    expect(browserPaneUrlError("https://user:secret@chat.deepseek.com/")).not.toBeNull();
    expect(browserPaneUrlError("https://127.0.0.1/")).not.toBeNull();
    expect(browserPaneUrlError("https://localhost/")).not.toBeNull();
    expect(browserPaneUrlError("file:///tmp/x")).not.toBeNull();
  });

  it("keeps a browser leaf's address when the pane is moved", () => {
    const root: LayoutNode = {
      type: "split",
      id: "split-1",
      direction: "row",
      ratio: 0.5,
      first: {
        type: "leaf",
        id: "page",
        harness: "browser",
        kind: "browser",
        url: "https://example.com/guide",
        cwd: "/tmp/guide",
      },
      second: { type: "leaf", id: "term", harness: "grok", cwd: "/work" },
    };
    const moved = moveLeaf(root, "page", "term", "bottom");
    const raw = JSON.stringify(moved);
    const restored = deserializeLayout(raw);
    expect(raw).toContain("https://example.com/guide");
    expect(restored).toEqual(moved);
    expect(deserializeLayout(raw.replace('"kind":"browser"', '"kind":"portal"'))).toBeNull();
  });

  it("opens a browser pane on the DeepSeek GUI without launching a harness", async () => {
    render(<HarnessTerminalGrid workspace="/test/workspace" />);
    fireEvent.click(screen.getByRole("button", { name: "+ DeepSeek GUI" }));
    const pane = await screen.findByLabelText("DeepSeek GUI address");
    expect(pane).toHaveValue(DEEPSEEK_GUI_HOME);
    expect(invoke).toHaveBeenCalledWith(
      "browser_pane_open",
      expect.objectContaining({ url: DEEPSEEK_GUI_HOME }),
    );
    expect(invoke).not.toHaveBeenCalledWith("hub_relaunch_harness_embedded", expect.anything());
  });

  it("refuses a private address from the pane", async () => {
    render(<HarnessTerminalGrid workspace="/test/workspace" />);
    fireEvent.click(screen.getByRole("button", { name: "+ DeepSeek GUI" }));
    const input = await screen.findByLabelText("DeepSeek GUI address");
    fireEvent.change(input, { target: { value: "https://192.168.1.9/" } });
    fireEvent.click(screen.getByRole("button", { name: "Open" }));
    expect(await screen.findByText("browser pane only opens public https pages")).toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalledWith(
      "browser_pane_open",
      expect.objectContaining({ url: "https://192.168.1.9/" }),
    );
  });

  it("restores a browser pane when its neighbor terminal is gone", async () => {
    storage["ca.terminalGrid.layout./test/workspace"] = JSON.stringify({
      type: "split",
      id: "split-1",
      direction: "row",
      ratio: 0.5,
      first: { type: "leaf", id: "stale", harness: "grok" },
      second: {
        type: "leaf",
        id: "page",
        harness: "browser",
        kind: "browser",
        url: DEEPSEEK_GUI_HOME,
      },
    });
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === "pty_session_status") {
        return { found: false, running: false, exited: true, exitDetail: null, outputTailB64: "" };
      }
      return null;
    });
    render(<HarnessTerminalGrid workspace="/test/workspace" />);
    expect(await screen.findByTestId("browser-pane-page")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Close grok pane" })).not.toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith(
      "browser_pane_open",
      expect.objectContaining({ leafId: "page", url: DEEPSEEK_GUI_HOME }),
    );
  });
});
