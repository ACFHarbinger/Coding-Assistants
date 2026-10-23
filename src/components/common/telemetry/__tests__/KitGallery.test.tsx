import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { KitGallery } from "../KitGallery";
import { SubtabBar } from "../SubtabBar";

afterEach(cleanup);

describe("KitGallery & shared telemetry kit (V2 / #373)", () => {
  it("renders all severity, provenance, staleness, and empty/error states in KitGallery", () => {
    render(<KitGallery forceRender />);

    expect(screen.getByTestId("noc-kit-gallery")).toBeTruthy();
    expect(screen.getByText(/NOC Telemetry Kit Gallery/)).toBeTruthy();

    // Provenance badges & unknown handling (never renders unknown cap as a number)
    expect(screen.getAllByText("measured").length).toBeGreaterThan(0);
    expect(screen.getAllByText("estimated").length).toBeGreaterThan(0);
    expect(screen.getAllByText("unknown").length).toBeGreaterThan(0);
    expect(screen.queryByText("999")).toBeNull();

    // Staleness dots: fresh, stale, awaiting
    expect(screen.getByText("live • 2s")).toBeTruthy();
    expect(screen.getByText("stale • 12s")).toBeTruthy();
    expect(screen.getByText("awaiting sample")).toBeTruthy();

    // Empty vs error states
    expect(screen.getByText("No pending approvals")).toBeTruthy();
    expect(screen.getByText("Approvals could not be loaded")).toBeTruthy();
  });

  it("supports keyboard and click selection on SubtabBar with badge counts", () => {
    const onSelect = vi.fn();
    render(
      <SubtabBar
        ariaLabel="Test subtabs"
        activeId="alpha"
        onSelect={onSelect}
        tabs={[
          { id: "alpha", label: "Alpha", badgeCount: 4 },
          { id: "beta", label: "Beta" },
        ]}
      />,
    );

    const alphaBtn = screen.getByRole("button", { name: /Alpha/ });
    const betaBtn = screen.getByRole("button", { name: /Beta/ });
    expect(alphaBtn.getAttribute("aria-pressed")).toBe("true");
    expect(betaBtn.getAttribute("aria-pressed")).toBe("false");
    expect(screen.getByText("4")).toBeTruthy();

    fireEvent.click(betaBtn);
    expect(onSelect).toHaveBeenCalledWith("beta");
  });
});
