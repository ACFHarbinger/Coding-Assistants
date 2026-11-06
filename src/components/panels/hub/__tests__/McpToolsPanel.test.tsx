import { beforeEach, describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import McpToolsPanel from "../McpToolsPanel";

describe("McpToolsPanel", () => {
  beforeEach(() => {
    try {
      window.localStorage.removeItem("ca.workspaceRoot");
    } catch {
      // Some sandboxed test environments provide a non-functional
      // localStorage stub; the panel falls back to "./workspace" either way.
    }
  });

  it("asks for a workspace when none is set", () => {
    render(<McpToolsPanel />);
    expect(screen.getByText(/Set a workspace root in Config/)).toBeTruthy();
  });
});
