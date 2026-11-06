import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import { act, fireEvent, render, screen } from "@testing-library/react";
import AppErrorBoundary from "../AppErrorBoundary";
import E2ECrashProbe from "../../../e2eCrashProbe";

function ThrowingComponent({ message = "Fatal render error" }: { message?: string }): React.JSX.Element {
  throw new Error(message);
}

describe("AppErrorBoundary (U14 / #143)", () => {
  let consoleErrorSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    // Suppress React's internal console.error output during deliberate render throws
    consoleErrorSpy = vi.spyOn(console, "error").mockImplementation(() => {});
  });

  afterEach(() => {
    consoleErrorSpy.mockRestore();
  });

  it("renders children normally when no exception occurs", () => {
    render(
      <AppErrorBoundary>
        <div>Normal application view</div>
      </AppErrorBoundary>,
    );

    expect(screen.getByText("Normal application view")).toBeTruthy();
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("catches render exception and displays accessible recovery view without stack trace", () => {
    const sensitiveMessage = "DB_SECRET_KEY=12345: Internal parser crashed unexpectedly";
    render(
      <AppErrorBoundary>
        <ThrowingComponent message={sensitiveMessage} />
      </AppErrorBoundary>,
    );

    const alert = screen.getByRole("alert");
    expect(alert).toBeTruthy();

    expect(screen.getByText("⚠ Application recovery")).toBeTruthy();
    expect(
      screen.getByRole("heading", { name: "Coding-Assistants needs to reload" }),
    ).toBeTruthy();
    expect(
      screen.getByText(
        "A screen could not be rendered. Your local Hub data has not been changed.",
      ),
    ).toBeTruthy();
    expect(screen.getByRole("button", { name: "Reload application" })).toBeTruthy();

    // Verify privacy & security: internal stack trace or raw sensitive error message is NOT rendered to DOM
    expect(screen.queryByText(sensitiveMessage)).toBeNull();
    expect(screen.queryByText(/DB_SECRET_KEY/i)).toBeNull();
    expect(screen.queryByText(/at ThrowingComponent/i)).toBeNull();
  });

  it("triggers onReload handler when Reload application button is clicked", () => {
    const onReload = vi.fn();
    render(
      <AppErrorBoundary onReload={onReload}>
        <ThrowingComponent />
      </AppErrorBoundary>,
    );

    const reloadBtn = screen.getByRole("button", { name: "Reload application" });
    fireEvent.click(reloadBtn);

    expect(onReload).toHaveBeenCalledTimes(1);
  });

  it("falls back to window.location.reload when onReload prop is omitted", () => {
    const originalLocation = window.location;
    const reloadMock = vi.fn();

    try {
      Object.defineProperty(window, "location", {
        configurable: true,
        value: { ...originalLocation, reload: reloadMock },
      });

      render(
        <AppErrorBoundary>
          <ThrowingComponent />
        </AppErrorBoundary>,
      );

      const reloadBtn = screen.getByRole("button", { name: "Reload application" });
      fireEvent.click(reloadBtn);

      expect(reloadMock).toHaveBeenCalledTimes(1);
    } finally {
      Object.defineProperty(window, "location", {
        configurable: true,
        value: originalLocation,
      });
    }
  });

  it("integrates with E2ECrashProbe to force render crashes on demand", () => {
    render(
      <AppErrorBoundary>
        <E2ECrashProbe />
        <div>Protected UI content</div>
      </AppErrorBoundary>,
    );

    // Initially healthy
    expect(screen.getByText("Protected UI content")).toBeTruthy();
    expect(screen.queryByRole("alert")).toBeNull();

    const w = window as unknown as { __E2E_FORCE_RENDER_CRASH__?: () => void };
    expect(typeof w.__E2E_FORCE_RENDER_CRASH__).toBe("function");

    // Force the render crash hook
    act(() => {
      w.__E2E_FORCE_RENDER_CRASH__!();
    });

    // Crash recovery view should now replace the protected UI
    expect(screen.getByRole("alert")).toBeTruthy();
    expect(
      screen.getByRole("heading", { name: "Coding-Assistants needs to reload" }),
    ).toBeTruthy();
    expect(screen.queryByText("Protected UI content")).toBeNull();
  });

  it("cleans up the E2ECrashProbe window hook on unmount", () => {
    const { unmount } = render(<E2ECrashProbe />);

    const w = window as unknown as { __E2E_FORCE_RENDER_CRASH__?: () => void };
    expect(typeof w.__E2E_FORCE_RENDER_CRASH__).toBe("function");

    unmount();
    expect(w.__E2E_FORCE_RENDER_CRASH__).toBeUndefined();
  });
});
