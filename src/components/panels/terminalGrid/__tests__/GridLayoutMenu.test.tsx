import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import GridLayoutMenu from "../GridLayoutMenu";
import type { LayoutNode } from "../layoutTree";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("GridLayoutMenu", () => {
  const mockLayout: LayoutNode = {
    type: "split",
    id: "split-1",
    direction: "row",
    ratio: 0.5,
    first: { type: "leaf", id: "leaf-1", harness: "claude" },
    second: { type: "leaf", id: "leaf-2", harness: "gemini" },
  };

  const mockCanvas = { width: 1200, height: 700 };

  beforeEach(() => {
    vi.clearAllMocks();
    (window as any).__TAURI_INTERNALS__ = {};
    vi.spyOn(window, "confirm").mockImplementation(() => true);
  });

  it("renders the trigger button and opens menu on click", async () => {
    vi.mocked(invoke).mockImplementation((cmd) => {
      if (cmd === "hub_list_terminal_grid_layouts") {
        return Promise.resolve([]);
      }
      return Promise.resolve(null);
    });

    render(
      <GridLayoutMenu
        currentLayout={mockLayout}
        currentCanvas={mockCanvas}
        onLoadLayout={vi.fn()}
      />
    );

    const button = screen.getByRole("button", { name: /Layouts/i });
    expect(button).toBeInTheDocument();

    fireEvent.click(button);

    expect(screen.getByText("Grid Layouts")).toBeInTheDocument();
    expect(screen.getByPlaceholderText(/Layout name/i)).toBeInTheDocument();
    await waitFor(() => {
      expect(screen.getByText(/No saved layouts yet/i)).toBeInTheDocument();
    });
  });

  it("saves a named layout when submitted", async () => {
    const onStatus = vi.fn();
    vi.mocked(invoke).mockImplementation((cmd, args: any) => {
      if (cmd === "hub_list_terminal_grid_layouts") {
        return Promise.resolve([]);
      }
      if (cmd === "hub_save_terminal_grid_layout") {
        return Promise.resolve({
          version: 1,
          name: args.name,
          savedAt: "2026-09-09T03:00:00Z",
          canvas: args.canvas,
          layout: args.layout,
        });
      }
      return Promise.resolve(null);
    });

    render(
      <GridLayoutMenu
        currentLayout={mockLayout}
        currentCanvas={mockCanvas}
        onLoadLayout={vi.fn()}
        onStatus={onStatus}
      />
    );

    fireEvent.click(screen.getByRole("button", { name: /Layouts/i }));

    const input = screen.getByPlaceholderText(/Layout name/i);
    fireEvent.change(input, { target: { value: "dual-coding" } });

    const saveBtn = screen.getByRole("button", { name: /^Save$/i });
    expect(saveBtn).not.toBeDisabled();

    fireEvent.click(saveBtn);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("hub_save_terminal_grid_layout", {
        name: "dual-coding",
        layout: mockLayout,
        canvas: { width: 1200, height: 700 },
      });
      expect(onStatus).toHaveBeenCalledWith('Layout "dual-coding" saved successfully.');
    });
  });

  it("loads a saved layout when Load is clicked", async () => {
    const onLoadLayout = vi.fn();
    const onStatus = vi.fn();

    vi.mocked(invoke).mockImplementation((cmd, args: any) => {
      if (cmd === "hub_list_terminal_grid_layouts") {
        return Promise.resolve([
          {
            name: "saved-pair",
            savedAt: "2026-09-09T02:00:00Z",
            version: 1,
            canvas: { width: 1000, height: 600 },
          },
        ]);
      }
      if (cmd === "hub_load_terminal_grid_layout") {
        return Promise.resolve({
          version: 1,
          name: args.name,
          savedAt: "2026-09-09T02:00:00Z",
          canvas: { width: 1000, height: 600 },
          layout: mockLayout,
        });
      }
      return Promise.resolve(null);
    });

    render(
      <GridLayoutMenu
        currentLayout={null}
        currentCanvas={null}
        onLoadLayout={onLoadLayout}
        onStatus={onStatus}
      />
    );

    fireEvent.click(screen.getByRole("button", { name: /Layouts/i }));

    await waitFor(() => {
      expect(screen.getByText("saved-pair")).toBeInTheDocument();
      expect(screen.getByText("1000×600px")).toBeInTheDocument();
    });

    const loadBtn = screen.getByRole("button", { name: /Load/i });
    fireEvent.click(loadBtn);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("hub_load_terminal_grid_layout", {
        name: "saved-pair",
      });
      expect(onLoadLayout).toHaveBeenCalledWith(mockLayout, { width: 1000, height: 600 });
      expect(onStatus).toHaveBeenCalledWith('Layout "saved-pair" loaded.');
      expect(screen.queryByText("Grid Layouts")).not.toBeInTheDocument();
    });
  });

  it("deletes a layout when delete button is clicked", async () => {
    const onStatus = vi.fn();
    let layouts = [
      {
        name: "to-delete",
        savedAt: "2026-09-09T02:00:00Z",
        version: 1,
        canvas: null,
      },
    ];

    vi.mocked(invoke).mockImplementation((cmd) => {
      if (cmd === "hub_list_terminal_grid_layouts") {
        return Promise.resolve(layouts);
      }
      if (cmd === "hub_delete_terminal_grid_layout") {
        layouts = [];
        return Promise.resolve(null);
      }
      return Promise.resolve(null);
    });

    render(
      <GridLayoutMenu
        currentLayout={mockLayout}
        currentCanvas={null}
        onLoadLayout={vi.fn()}
        onStatus={onStatus}
      />
    );

    fireEvent.click(screen.getByRole("button", { name: /Layouts/i }));

    await waitFor(() => {
      expect(screen.getByText("to-delete")).toBeInTheDocument();
    });

    const deleteBtn = screen.getByTitle("Delete this layout");
    fireEvent.click(deleteBtn);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("hub_delete_terminal_grid_layout", {
        name: "to-delete",
      });
      expect(onStatus).toHaveBeenCalledWith('Layout "to-delete" deleted.');
    });
  });

  it("closes the menu on Escape key", () => {
    vi.mocked(invoke).mockImplementation(() => Promise.resolve([]));

    render(
      <GridLayoutMenu
        currentLayout={mockLayout}
        currentCanvas={null}
        onLoadLayout={vi.fn()}
      />
    );

    fireEvent.click(screen.getByRole("button", { name: /Layouts/i }));
    expect(screen.getByText("Grid Layouts")).toBeInTheDocument();

    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByText("Grid Layouts")).not.toBeInTheDocument();
  });
});
