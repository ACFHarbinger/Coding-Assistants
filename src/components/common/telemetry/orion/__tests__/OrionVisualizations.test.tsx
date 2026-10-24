import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  OrionRadarChart,
  OrionConstellationGraph,
  OrionOrbitalBubbleChart,
  OrionHoneycombMap,
  OrionDotMatrixHeatmap,
  OrionRadialRingMeter,
  getSampleOrionData,
} from "../index";
import { KitGalleryOrionSection } from "../../KitGalleryOrionSection";

afterEach(cleanup);

describe("Orion Data Visualizations Suite", () => {
  const sample = getSampleOrionData();

  describe("OrionRadarChart (35-omparison_chart_dark.webp)", () => {
    it("renders radar axes, series polygons, and legend toggles", () => {
      render(
        <OrionRadarChart
          title="Test Radar"
          axes={sample.radarAxes}
          series={sample.radarSeries}
        />,
      );

      expect(screen.getByTestId("orion-radar-chart")).toBeTruthy();
      expect(screen.getByText("Test Radar")).toBeTruthy();
      expect(screen.getByText("Latency")).toBeTruthy();
      expect(screen.getByText("Throughput")).toBeTruthy();
      expect(screen.getByText("Orchestrator v2 (Primary)")).toBeTruthy();
      expect(screen.getByText("Harness Runner (Subagent)")).toBeTruthy();
    });

    it("displays hover HUD tooltip when hovering an axis", () => {
      render(
        <OrionRadarChart
          title="Test Radar"
          axes={sample.radarAxes}
          series={sample.radarSeries}
        />,
      );

      const latencyAxis = screen.getByRole("button", { name: /Axis Latency/ });
      fireEvent.mouseEnter(latencyAxis);

      expect(screen.getAllByText(/Latency/).length).toBeGreaterThan(0);
      expect(screen.getByText(/88 ms/)).toBeTruthy();
    });

    it("toggles series visibility when clicking legend item", () => {
      render(
        <OrionRadarChart
          title="Test Radar"
          axes={sample.radarAxes}
          series={sample.radarSeries}
        />,
      );

      const btn = screen.getByRole("button", { name: /Orchestrator v2/ });
      expect(btn.getAttribute("aria-pressed")).toBe("true");
      fireEvent.click(btn);
      expect(btn.getAttribute("aria-pressed")).toBe("false");
    });
  });

  describe("OrionConstellationGraph (19-Skill_tree_dark-min.webp)", () => {
    it("renders core hub, satellite nodes, and interconnecting edges", () => {
      const onNodeClick = vi.fn();
      render(
        <OrionConstellationGraph
          title="Test Constellation"
          nodes={sample.constellationNodes}
          edges={sample.constellationEdges}
          onNodeClick={onNodeClick}
        />,
      );

      expect(screen.getByTestId("orion-constellation-graph")).toBeTruthy();
      expect(screen.getByText("Nexus Coordinator")).toBeTruthy();
      expect(screen.getByText("Coder Subagent")).toBeTruthy();
      expect(screen.getByText("Security Reviewer")).toBeTruthy();

      const coderBtn = screen.getByRole("button", { name: /Coder Subagent/ });
      fireEvent.click(coderBtn);
      expect(onNodeClick).toHaveBeenCalled();
      expect(screen.getByText("Reset View")).toBeTruthy();
    });
  });

  describe("OrionOrbitalBubbleChart (3-Bubble_chart_dark-mi.webp)", () => {
    it("renders orbital tracks and 3D glowing sphere bubbles with inspection card", () => {
      render(
        <OrionOrbitalBubbleChart
          title="Test Orbital Bubbles"
          bubbles={sample.orbitalBubbles}
        />,
      );

      expect(screen.getByTestId("orion-orbital-bubble-chart")).toBeTruthy();
      expect(screen.getByText("GPT-4o Synthesizer")).toBeTruthy();
      expect(screen.getByText("Claude 3.5 Sonnet")).toBeTruthy();

      const bubble = screen.getByRole("button", { name: /GPT-4o Synthesizer/ });
      fireEvent.mouseEnter(bubble);
      expect(screen.getByText("LLM Provider")).toBeTruthy();
      expect(screen.getByText(/384,000/)).toBeTruthy();
    });
  });

  describe("OrionHoneycombMap (11-Hexagon_map_dark-min.webp & 31-Movement_infographic.webp)", () => {
    it("renders hex cells, trajectory paths, and severity filter controls", () => {
      render(
        <OrionHoneycombMap
          title="Test Honeycomb"
          cells={sample.honeycombCells}
          trajectories={sample.honeycombTrajectories}
        />,
      );

      expect(screen.getByTestId("orion-honeycomb-map")).toBeTruthy();
      expect(screen.getByText("CORE")).toBeTruthy();
      expect(screen.getByText("BUILD")).toBeTruthy();

      // Test filter pill
      const alertFilter = screen.getByRole("button", { name: /alert/i });
      fireEvent.click(alertFilter);
      expect(alertFilter.getAttribute("aria-pressed")).toBe("true");

      const allFilter = screen.getByRole("button", { name: /all/i });
      fireEvent.click(allFilter);
      expect(allFilter.getAttribute("aria-pressed")).toBe("true");
    });
  });

  describe("OrionDotMatrixHeatmap (9-Heat_map_dark-min.webp)", () => {
    it("renders dot matrix grid, column labels, row labels, and pulse bars", () => {
      render(
        <OrionDotMatrixHeatmap
          title="Test Dynamics Matrix"
          columnLabels={sample.matrixColumns}
          rows={sample.matrixRows}
        />,
      );

      expect(screen.getByTestId("orion-dot-matrix-heatmap")).toBeTruthy();
      expect(screen.getByText("Orchestrator")).toBeTruthy();
      expect(screen.getByText("Coder Agent")).toBeTruthy();
      expect(screen.getByText("12:00")).toBeTruthy();
      expect(screen.getByText("Pulse")).toBeTruthy();
    });
  });

  describe("OrionRadialRingMeter (Radial arc gauges)", () => {
    it("renders concentric circular arcs, center KPI, and legend rows", () => {
      render(
        <OrionRadialRingMeter
          title="Test Radial Rings"
          rings={sample.radialRings}
          centerLabel="Fleet Health"
          centerValue="96.4%"
        />,
      );

      expect(screen.getByTestId("orion-radial-ring-meter")).toBeTruthy();
      expect(screen.getByText("Fleet Health")).toBeTruthy();
      expect(screen.getByText("96.4%")).toBeTruthy();
      expect(screen.getByText("Core Processing Load")).toBeTruthy();
      expect(screen.getByText("Memory Allocation")).toBeTruthy();

      const legendItem = screen.getByRole("button", { name: /Core Processing Load/ });
      fireEvent.mouseEnter(legendItem);
      expect(screen.getAllByText("74%").length).toBeGreaterThan(0);
    });
  });

  describe("KitGalleryOrionSection", () => {
    it("renders all visualizations and switches category filters", () => {
      render(<KitGalleryOrionSection />);

      expect(screen.getByTestId("orion-showcase-section")).toBeTruthy();
      expect(screen.getByText("SetProduct Orion Data Visualization Suite")).toBeTruthy();

      const radarFilter = screen.getByRole("button", { name: "Radar" });
      fireEvent.click(radarFilter);
      expect(screen.getByTestId("orion-radar-chart")).toBeTruthy();
      expect(screen.queryByTestId("orion-constellation-graph")).toBeNull();

      const allFilter = screen.getByRole("button", { name: "All Visualizations" });
      fireEvent.click(allFilter);
      expect(screen.getByTestId("orion-radar-chart")).toBeTruthy();
      expect(screen.getByTestId("orion-constellation-graph")).toBeTruthy();
      expect(screen.getByTestId("orion-orbital-bubble-chart")).toBeTruthy();
      expect(screen.getByTestId("orion-honeycomb-map")).toBeTruthy();
      expect(screen.getByTestId("orion-dot-matrix-heatmap")).toBeTruthy();
      expect(screen.getByTestId("orion-radial-ring-meter")).toBeTruthy();
    });
  });
});
