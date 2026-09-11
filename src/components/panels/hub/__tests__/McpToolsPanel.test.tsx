import { beforeEach, describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import McpToolsPanel from "../McpToolsPanel";

describe("McpToolsPanel", () => {
  beforeEach(() => {
    localStorage.removeItem("ca.workspaceRoot");
  });

  it("asks for a workspace when none is set", () => {
    render(<McpToolsPanel />);
    expect(screen.getByText(/Set a workspace root in Config/)).toBeTruthy();
  });
});
