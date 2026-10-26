import { describe, expect, it, vi } from "vitest";
import { invoke } from "../../../../lib/tauri";
import { browserPaneCommand } from "../browserPaneCommands";
vi.mock("../../../../lib/tauri", () => ({ invoke: vi.fn() }));

describe("native browser lifetime queue", () => {
  it("finishes a pending open and its close before reopening a remounted leaf", async () => {
    let finish!: () => void;
    vi.mocked(invoke).mockImplementationOnce(() => new Promise<void>((resolve) => { finish = resolve; }));
    const first = browserPaneCommand("lifetime", "browser_pane_open");
    await vi.waitFor(() => expect(finish).toBeTypeOf("function"));
    const close = browserPaneCommand("lifetime", "browser_pane_close");
    const reopen = browserPaneCommand("lifetime", "browser_pane_open");
    expect(vi.mocked(invoke).mock.calls.map(([command]) => command)).toEqual(["browser_pane_open"]);
    finish();
    await Promise.all([first, close, reopen]);
    expect(vi.mocked(invoke).mock.calls.map(([command]) => command)).toEqual([
      "browser_pane_open", "browser_pane_close", "browser_pane_open",
    ]);
  });
  it("still closes a leaf after a failed native command", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error("open failed"));
    const open = browserPaneCommand("failed", "browser_pane_open");
    const close = browserPaneCommand("failed", "browser_pane_close");
    await expect(open).rejects.toThrow("open failed");
    await close;
    expect(invoke).toHaveBeenLastCalledWith("browser_pane_close", { leafId: "failed" });
  });
});


it("evaluates placement against the latest visibility when a queued open finishes", async () => {
  let finish!: () => void;
  let visible = true;
  vi.mocked(invoke).mockImplementationOnce(() => new Promise<void>((resolve) => { finish = resolve; }));
  const open = browserPaneCommand("visibility", "browser_pane_open");
  await vi.waitFor(() => expect(finish).toBeTypeOf("function"));
  const place = browserPaneCommand("visibility", "browser_pane_place", () => ({ bounds: { visible } }));
  visible = false;
  finish();
  await Promise.all([open, place]);
  expect(invoke).toHaveBeenLastCalledWith("browser_pane_place", { leafId: "visibility", bounds: { visible: false } });
});
