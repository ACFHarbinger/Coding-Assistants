import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import { render, screen } from "@testing-library/react";
import TerminalPaneErrorBoundary from "../TerminalPaneErrorBoundary";

function TerminalCrashingChild({ message }: { message?: string }): React.JSX.Element {
  if (message !== undefined) {
    throw new Error(message);
  }
  const err = new Error();
  err.message = "";
  throw err;
}

describe("TerminalPaneErrorBoundary (#143)", () => {
  let consoleErrorSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    consoleErrorSpy = vi.spyOn(console, "error").mockImplementation(() => {});
  });

  afterEach(() => {
    consoleErrorSpy.mockRestore();
  });

  it("renders terminal children normally when no exception occurs", () => {
    render(
      <TerminalPaneErrorBoundary>
        <div data-testid="xterm-mock">Active Terminal Session</div>
      </TerminalPaneErrorBoundary>,
    );

    expect(screen.getByTestId("xterm-mock")).toBeTruthy();
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("catches terminal render error and displays pane crash recovery alert", () => {
    render(
      <TerminalPaneErrorBoundary>
        <TerminalCrashingChild message="WebGL context loss during resize" />
      </TerminalPaneErrorBoundary>,
    );

    const alert = screen.getByRole("alert");
    expect(alert).toBeTruthy();
    expect(screen.getByText("In-app terminal crashed.")).toBeTruthy();
    expect(screen.getByText("WebGL context loss during resize")).toBeTruthy();
    expect(
      screen.getByText(
        "The rest of the app is still running. Close this pane and try Resume again.",
      ),
    ).toBeTruthy();
  });

  it("falls back to default error message when error.message is empty", () => {
    render(
      <TerminalPaneErrorBoundary>
        <TerminalCrashingChild />
      </TerminalPaneErrorBoundary>,
    );

    expect(screen.getByText("Terminal view failed to render")).toBeTruthy();
  });

  it("isolates failure so surrounding components in parent continue to render", () => {
    render(
      <div>
        <nav data-testid="app-navigation">Navigation Header</nav>
        <div data-testid="grid-container">
          <TerminalPaneErrorBoundary>
            <TerminalCrashingChild message="PTY pipe broken" />
          </TerminalPaneErrorBoundary>
          <div data-testid="sibling-pane">Healthy Sibling Pane</div>
        </div>
      </div>,
    );

    // Sibling pane and parent navigation remain intact
    expect(screen.getByTestId("app-navigation")).toBeTruthy();
    expect(screen.getByTestId("sibling-pane")).toBeTruthy();
    expect(screen.getByText("PTY pipe broken")).toBeTruthy();
  });
});
