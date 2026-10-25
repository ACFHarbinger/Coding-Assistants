import { useState } from "react";
import {
  OrionRadarChart,
  OrionConstellationGraph,
  OrionOrbitalBubbleChart,
  OrionHoneycombMap,
  OrionDotMatrixHeatmap,
  OrionRadialRingMeter,
  getSampleOrionData,
} from "./orion";
import { nocCardDenseStyle } from "./panel";
import { NOC_COLORS, NOC_SPACING, NOC_SURFACES } from "./tokens";

type OrionFilter = "all" | "radar" | "constellation" | "orbital" | "honeycomb" | "matrix" | "radial";

export function KitGalleryOrionSection() {
  const [selectedViz, setSelectedViz] = useState<OrionFilter>("all");
  const data = getSampleOrionData();

  const filterTabs: { id: OrionFilter; label: string }[] = [
    { id: "all", label: "All Visualizations" },
    { id: "radar", label: "Radar" },
    { id: "constellation", label: "Constellation" },
    { id: "orbital", label: "Orbital Bubbles" },
    { id: "honeycomb", label: "Honeycomb Map" },
    { id: "matrix", label: "Dynamics Matrix" },
    { id: "radial", label: "Radial Rings" },
  ];

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: NOC_SPACING.space4 }} data-testid="orion-showcase-section">
      <div
        style={{
          ...nocCardDenseStyle,
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          gap: "1rem",
          flexWrap: "wrap",
          background: "linear-gradient(90deg, rgba(56, 189, 248, 0.08) 0%, rgba(99, 102, 241, 0.08) 100%)",
          border: `1px solid ${NOC_COLORS.cyan}44`,
        }}
      >
        <div>
          <span style={{ fontSize: "0.85rem", fontWeight: 700, color: NOC_COLORS.cyan }}>
            SetProduct Orion Data Visualization Suite
          </span>
          <p style={{ margin: "0.15rem 0 0", fontSize: "0.75rem", color: NOC_SURFACES.textMuted }}>
            Dark-NOC data visual primitives with glowing SVG neon shaders, multi-tier orbits, and interactive HUD telemetry.
          </p>
        </div>

        <div style={{ display: "flex", gap: "0.35rem", flexWrap: "wrap" }}>
          {filterTabs.map((tab) => (
            <button
              key={tab.id}
              type="button"
              onClick={() => setSelectedViz(tab.id)}
              aria-pressed={selectedViz === tab.id}
              style={{
                background: selectedViz === tab.id ? "rgba(56, 189, 248, 0.2)" : "rgba(148, 163, 184, 0.08)",
                border: `1px solid ${selectedViz === tab.id ? NOC_COLORS.cyan : "rgba(148, 163, 184, 0.2)"}`,
                borderRadius: "6px",
                padding: "0.25rem 0.6rem",
                color: selectedViz === tab.id ? "#ffffff" : NOC_SURFACES.textSecondary,
                fontSize: "0.74rem",
                fontWeight: selectedViz === tab.id ? 700 : 500,
                cursor: "pointer",
                transition: "all 140ms ease",
              }}
            >
              {tab.label}
            </button>
          ))}
        </div>
      </div>

      <div
        style={{
          display: "grid",
          gridTemplateColumns: selectedViz === "all" ? "repeat(auto-fit, minmax(460px, 1fr))" : "1fr",
          gap: NOC_SPACING.space4,
        }}
      >
        {(selectedViz === "all" || selectedViz === "radar") && (
          <OrionRadarChart
            title="Agent Performance & Safety Radar"
            subtitle="Comparing multi-axis capability profiles of active agent runtimes"
            axes={data.radarAxes}
            series={data.radarSeries}
          />
        )}

        {(selectedViz === "all" || selectedViz === "constellation") && (
          <OrionConstellationGraph
            title="Agent Constellation Topology"
            subtitle="Central orchestrator with satellite subagents and tool links"
            nodes={data.constellationNodes}
            edges={data.constellationEdges}
          />
        )}

        {(selectedViz === "all" || selectedViz === "orbital") && (
          <OrionOrbitalBubbleChart
            title="Provider Volume & Quota Orbits"
            subtitle="3D spherical clusters representing LLM usage and token footprints"
            bubbles={data.orbitalBubbles}
            centerValue="1.72M Tokens"
          />
        )}

        {(selectedViz === "all" || selectedViz === "honeycomb") && (
          <OrionHoneycombMap
            title="Service Mesh Honeycomb & Trajectories"
            subtitle="Situational partition grid with active message flight paths"
            cells={data.honeycombCells}
            trajectories={data.honeycombTrajectories}
          />
        )}

        {(selectedViz === "all" || selectedViz === "matrix") && (
          <OrionDotMatrixHeatmap
            title="Activity Dynamics Scatter Matrix"
            subtitle="Time-window event density with integrated audio-style pulse equalizer"
            columnLabels={data.matrixColumns}
            rows={data.matrixRows}
          />
        )}

        {(selectedViz === "all" || selectedViz === "radial") && (
          <OrionRadialRingMeter
            title="Resource Allocation & Bandwidth Arcs"
            subtitle="Concentric utilization rings with center telemetry readout"
            rings={data.radialRings}
            centerLabel="Fleet Health"
            centerValue="96.4%"
            centerSubtext="All Cores Nominal"
          />
        )}
      </div>
    </div>
  );
}
