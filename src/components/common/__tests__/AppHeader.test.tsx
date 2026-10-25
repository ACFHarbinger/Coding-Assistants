import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { AppHeader } from "../AppHeader";

describe("AppHeader cockpit HUD (V3 / #374)", () => {
  it("renders brand, HUD pills, mode switcher, settings, and hub status", () => {
    const onSelectView = vi.fn();
    const onOpenSettings = vi.fn();

    render(
      <AppHeader
        mainView="messager"
        onSelectView={onSelectView}
        workspacePath="/home/developer/projects/my-cool-app"
        activeWorkSessionName="Sprint Planning"
        onOpenSettings={onOpenSettings}
        hubStatus="Local hub online"
      />
    );

    // Title
    expect(screen.getByRole("heading", { level: 1, name: "Coding Assistants" })).toBeTruthy();

    // Mode switcher buttons
    const chatBtn = screen.getByRole("tab", { name: /Chat & Memory/i });
    const orchBtn = screen.getByRole("tab", { name: "Orchestrate" });
    const opsBtn = screen.getByRole("tab", { name: "Operations" });
    const workBtn = screen.getByRole("tab", { name: "Work" });

    expect(chatBtn.getAttribute("aria-selected")).toBe("true");
    expect(orchBtn.getAttribute("aria-selected")).toBe("false");

    // Switching views
    fireEvent.click(opsBtn);
    expect(onSelectView).toHaveBeenCalledWith("operations");

    fireEvent.click(workBtn);
    expect(onSelectView).toHaveBeenCalledWith("work");

    // Workspace Root pill shows basename and has full path in title
    const workspacePill = screen.getByTestId("hud-pill-workspace");
    expect(workspacePill.textContent).toContain("my-cool-app");
    expect(workspacePill.getAttribute("title")).toContain("/home/developer/projects/my-cool-app");

    fireEvent.click(workspacePill);
    expect(onSelectView).toHaveBeenCalledWith("orchestrate");

    // Active session pill
    const sessionPill = screen.getByTestId("hud-pill-session");
    expect(sessionPill.textContent).toContain("Sprint Planning");
    fireEvent.click(sessionPill);
    expect(onSelectView).toHaveBeenCalledWith("messager");

    // Settings button
    const settingsBtn = screen.getByRole("button", { name: /Settings/i });
    fireEvent.click(settingsBtn);
    expect(onOpenSettings).toHaveBeenCalled();

    // Hub status
    expect(screen.getByTestId("hud-status-hub").textContent).toContain("Local hub online");
  });

  it("handles empty workspace and null session cleanly", () => {
    const onSelectView = vi.fn();
    render(
      <AppHeader
        mainView="orchestrate"
        onSelectView={onSelectView}
        workspacePath=""
        activeWorkSessionName={null}
        onOpenSettings={vi.fn()}
      />
    );

    const workspacePill = screen.getByTestId("hud-pill-workspace");
    expect(workspacePill.textContent).toContain("Not set");

    const sessionPill = screen.getByTestId("hud-pill-session");
    expect(sessionPill.textContent).toContain("None selected");
  });

  it("renders pending gates pill when count > 0 and routes to operations", () => {
    const onSelectView = vi.fn();
    render(
      <AppHeader
        mainView="work"
        onSelectView={onSelectView}
        workspacePath="/test"
        activeWorkSessionName="Test"
        onOpenSettings={vi.fn()}
        pendingGatesWakes={3}
      />
    );

    const gatesPill = screen.getByTestId("hud-pill-gates");
    expect(gatesPill.textContent).toContain("3 gates");

    fireEvent.click(gatesPill);
    expect(onSelectView).toHaveBeenCalledWith("operations");
  });

  it("preserves #213 containing-block safety by omitting inline backdropFilter", () => {
    const { container } = render(
      <AppHeader
        mainView="messager"
        onSelectView={vi.fn()}
        workspacePath="/test"
        activeWorkSessionName={null}
        onOpenSettings={vi.fn()}
      />
    );

    const header = container.querySelector("header");
    expect(header).toBeTruthy();
    expect(header?.style.backdropFilter).toBeFalsy();
  });
});
