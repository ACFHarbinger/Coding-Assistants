export * from "./OrionRadarChart";
export * from "./OrionConstellationGraph";
export * from "./OrionOrbitalBubbleChart";
export * from "./OrionHoneycombMap";
export * from "./OrionDotMatrixHeatmap";
export * from "./OrionRadialRingMeter";

import { NOC_COLORS } from "../tokens";
import type { RadarAxis, RadarSeries } from "./OrionRadarChart";
import type { ConstellationEdge, ConstellationNode } from "./OrionConstellationGraph";
import type { OrbitalBubble } from "./OrionOrbitalBubbleChart";
import type { HexCellData, HexTrajectory } from "./OrionHoneycombMap";
import type { MatrixRow } from "./OrionDotMatrixHeatmap";
import type { RadialRing } from "./OrionRadialRingMeter";

/**
 * Realistic Mock Data Generator for Orion Visualizations.
 * Used in KitGallery, Acceptance tests, and offline telemetry demonstrations.
 */
export function getSampleOrionData() {
  const radarAxes: RadarAxis[] = [
    { key: "latency", label: "Latency", max: 100, unit: "ms" },
    { key: "throughput", label: "Throughput", max: 100, unit: "tps" },
    { key: "accuracy", label: "Accuracy", max: 100, unit: "%" },
    { key: "safety", label: "Safety Gating", max: 100, unit: "%" },
    { key: "cache", label: "Cache Hit", max: 100, unit: "%" },
    { key: "concurrency", label: "Concurrency", max: 100, unit: "th" },
  ];

  const radarSeries: RadarSeries[] = [
    {
      id: "agent-alpha",
      name: "Orchestrator v2 (Primary)",
      color: NOC_COLORS.cyan,
      values: {
        latency: 88,
        throughput: 92,
        accuracy: 96,
        safety: 90,
        cache: 84,
        concurrency: 78,
      },
    },
    {
      id: "agent-beta",
      name: "Harness Runner (Subagent)",
      color: NOC_COLORS.purple,
      values: {
        latency: 68,
        throughput: 74,
        accuracy: 82,
        safety: 94,
        cache: 62,
        concurrency: 88,
      },
    },
  ];

  const constellationNodes: ConstellationNode[] = [
    {
      id: "core-hub",
      label: "Nexus Coordinator",
      role: "Central Orchestrator",
      tier: 0,
      status: "ok",
      metric: "99.8%",
      metricLabel: "Uptime",
      data: { "Active Channels": 8, "Throughput": "4.2k msg/s" },
    },
    {
      id: "agent-coder",
      label: "Coder Subagent",
      role: "Synthesis & Refactor",
      tier: 1,
      angle: 30,
      distance: 0.52,
      status: "cyan",
      metric: "14 tps",
      data: { "Workspace": "Coding-Assistants", "Tokens": "320k" },
    },
    {
      id: "agent-tester",
      label: "Test Inspector",
      role: "Vitest & Verification",
      tier: 1,
      angle: 150,
      distance: 0.56,
      status: "ok",
      metric: "53/53",
      data: { "Suites": 53, "Status": "Passing" },
    },
    {
      id: "agent-reviewer",
      label: "Security Reviewer",
      role: "AST & Gatekeeper",
      tier: 1,
      angle: 270,
      distance: 0.55,
      status: "warn",
      metric: "2 Gates",
      data: { "Pending Approvals": 2, "Violations": 0 },
    },
    {
      id: "node-mcp",
      label: "MCP Gateway",
      role: "Tool Protocol",
      tier: 2,
      angle: 75,
      distance: 0.86,
      status: "ok",
      metric: "12 Tools",
      data: { "Servers Connected": 3, "Latency": "4ms" },
    },
    {
      id: "node-vector",
      label: "Vector Store",
      role: "Semantic Memory",
      tier: 2,
      angle: 210,
      distance: 0.88,
      status: "cyan",
      metric: "1.4M emb",
      data: { "Index": "HNSW", "Recall": "98.4%" },
    },
    {
      id: "node-bridge",
      label: "Tauri Bridge",
      role: "IPC Channel",
      tier: 2,
      angle: 330,
      distance: 0.84,
      status: "idle",
      metric: "Standby",
      data: { "Events": "18.2k", "Drop Rate": "0%" },
    },
  ];

  const constellationEdges: ConstellationEdge[] = [
    { id: "e1", source: "core-hub", target: "agent-coder", active: true },
    { id: "e2", source: "core-hub", target: "agent-tester", active: true },
    { id: "e3", source: "core-hub", target: "agent-reviewer", active: false },
    { id: "e4", source: "agent-coder", target: "node-mcp", active: true },
    { id: "e5", source: "agent-reviewer", target: "node-vector", active: false },
    { id: "e6", source: "core-hub", target: "node-bridge", active: false },
  ];

  const orbitalBubbles: OrbitalBubble[] = [
    { id: "b1", label: "GPT-4o Synthesizer", category: "LLM Provider", value: 384000, unit: "tokens", color: NOC_COLORS.cyan, orbitIndex: 0, angle: 45 },
    { id: "b2", label: "Claude 3.5 Sonnet", category: "LLM Provider", value: 512000, unit: "tokens", color: NOC_COLORS.purple, orbitIndex: 1, angle: 140 },
    { id: "b3", label: "Local Qwen 2.5", category: "Ollama / Edge", value: 168000, unit: "tokens", color: NOC_COLORS.emerald, orbitIndex: 0, angle: 220 },
    { id: "b4", label: "Gemini 1.5 Pro", category: "LLM Provider", value: 290000, unit: "tokens", color: NOC_COLORS.pink, orbitIndex: 1, angle: 300 },
    { id: "b5", label: "Kimi Moonshot", category: "LLM Provider", value: 142000, unit: "tokens", color: NOC_COLORS.amber, orbitIndex: 2, angle: 80 },
    { id: "b6", label: "DeepSeek Coder", category: "Code LLM", value: 225000, unit: "tokens", color: NOC_COLORS.teal, orbitIndex: 2, angle: 260 },
  ];

  const honeycombCells: HexCellData[] = [
    { col: 1, row: 1, intensity: 45, label: "DB-1", status: "active", service: "Postgres Read Replica" },
    { col: 2, row: 1, intensity: 80, label: "API-1", status: "warn", service: "HTTP Ingress Gateway" },
    { col: 3, row: 2, intensity: 92, label: "CORE", status: "crit", service: "Coordinator Node" },
    { col: 4, row: 1, intensity: 35, label: "VEC", status: "ok", service: "Embedding Engine" },
    { col: 5, row: 2, intensity: 60, label: "LLM", status: "active", service: "Provider Dispatcher" },
    { col: 6, row: 3, intensity: 20, label: "CACHE", status: "ok", service: "Redis Cluster" },
    { col: 7, row: 2, intensity: 15, label: "QUEUE", status: "ok", service: "Tokio Broadcast Bus" },
    { col: 8, row: 3, intensity: 75, label: "GATE", status: "warn", service: "Approval Gatekeeper" },
    { col: 9, row: 4, intensity: 95, label: "BUILD", status: "crit", service: "Cargo / Vite Builder" },
    { col: 10, row: 3, intensity: 25, label: "AUDIT", status: "ok", service: "Audit Log Streamer" },
  ];

  const honeycombTrajectories: HexTrajectory[] = [
    { id: "t1", from: [2, 1], to: [3, 2], color: NOC_COLORS.cyan },
    { id: "t2", from: [3, 2], to: [5, 2], color: NOC_COLORS.purple },
    { id: "t3", from: [5, 2], to: [8, 3], color: NOC_COLORS.amber },
    { id: "t4", from: [8, 3], to: [9, 4], color: NOC_COLORS.crit },
  ];

  const matrixColumns = ["00:00", "03:00", "06:00", "09:00", "12:00", "15:00", "18:00", "21:00"];
  const matrixRows: MatrixRow[] = [
    { id: "r1", label: "Orchestrator", values: [12, 14, 28, 85, 94, 88, 72, 34] },
    { id: "r2", label: "Coder Agent", values: [8, 6, 15, 62, 89, 96, 64, 22] },
    { id: "r3", label: "Reviewer Agent", values: [4, 2, 8, 44, 68, 75, 48, 16] },
    { id: "r4", label: "Test Runner", values: [18, 12, 32, 78, 82, 84, 58, 26] },
    { id: "r5", label: "MCP Tools", values: [22, 16, 45, 92, 98, 95, 84, 40] },
  ];

  const radialRings: RadialRing[] = [
    { id: "cpu", label: "Core Processing Load", percent: 74, color: NOC_COLORS.cyan, valueText: "12 / 16 Cores", detail: "Tokio worker pool" },
    { id: "mem", label: "Memory Allocation", percent: 58, color: NOC_COLORS.indigo, valueText: "4.8 / 8.0 GB", detail: "Workspace buffer" },
    { id: "tok", label: "Token Quota Window", percent: 86, color: NOC_COLORS.purple, valueText: "860k / 1M", detail: "5h rolling cap" },
    { id: "net", label: "IPC Bandwidth", percent: 42, color: NOC_COLORS.emerald, valueText: "24.5 MB/s", detail: "Tauri event stream" },
  ];

  return {
    radarAxes,
    radarSeries,
    constellationNodes,
    constellationEdges,
    orbitalBubbles,
    honeycombCells,
    honeycombTrajectories,
    matrixColumns,
    matrixRows,
    radialRings,
  };
}
