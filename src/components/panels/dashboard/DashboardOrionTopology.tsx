import { useMemo, useState } from "react";
import {
  OrionConstellationGraph,
  OrionRadarChart,
  OrionHoneycombMap,
  OrionRadialRingMeter,
  type ConstellationNode,
  type ConstellationEdge,
  type RadarAxis,
  type RadarSeries,
  type HexCellData,
  type HexTrajectory,
  type RadialRing,
} from "../../common/telemetry/orion";
import { nocCardDenseStyle } from "../../common/telemetry/panel";
import { NOC_COLORS, NOC_SURFACES } from "../../common/telemetry/tokens";

export interface DashboardTopologyProps {
  agents: Array<{ id: string; display_name: string }>;
  metrics: Array<{
    agent_id: string;
    lines_written: number;
    tokens_used: number;
    tokens_cached: number;
    provider_calls: number;
  }>;
  budgets: Array<{
    agent_id: string;
    limit_units: number;
    spent_units: number;
    paused: boolean;
  }>;
  tasks: Array<{ id: string; title: string; status: string }>;
  messages: Array<{ from_agent: string; to_agent: string; status: string }>;
  wakes: Array<{ target_agent: string; status: string }>;
}

export function DashboardOrionTopology({
  agents,
  metrics,
  budgets,
  tasks,
  messages,
  wakes,
}: DashboardTopologyProps) {
  const [activeView, setActiveView] = useState<"constellation" | "radar" | "honeycomb" | "rings">("constellation");

  // Map agents, messages, and wakes into live constellation
  const constellationData = useMemo(() => {
    if (agents.length === 0) {
      return { nodes: [], edges: [] };
    }

    const coreNode: ConstellationNode = {
      id: "hub-core",
      label: "Workspace Nexus",
      role: "Hub Coordinator",
      tier: 0,
      status: "ok",
      metric: `${tasks.length} tasks`,
      metricLabel: "Workload",
      data: {
        "Enrolled Agents": agents.length,
        "Total Messages": messages.length,
        "Pending Wakes": wakes.filter((w) => w.status === "pending").length,
      },
    };

    const agentNodes: ConstellationNode[] = agents.map((agent, idx) => {
      const budget = budgets.find((b) => b.agent_id === agent.id);
      const metric = metrics.find((m) => m.agent_id === agent.id);
      const angle = (idx * 360) / Math.max(1, agents.length);

      let status: ConstellationNode["status"] = "ok";
      if (budget?.paused) status = "crit";
      else if (metric && metric.provider_calls > 0) status = "cyan";
      else status = "idle";

      return {
        id: agent.id,
        label: agent.display_name,
        role: "Active Agent",
        tier: 1,
        angle,
        distance: 0.62,
        status,
        metric: metric ? `${metric.tokens_used.toLocaleString()} tok` : "Unavailable",
        metricLabel: "Usage",
        data: {
          "Lines Written": metric?.lines_written ?? "N/A",
          "Provider Calls": metric?.provider_calls ?? "N/A",
          "Budget Spent": budget ? `${budget.spent_units}/${budget.limit_units}` : "N/A",
        },
      };
    });

    const edges: ConstellationEdge[] = [];
    agentNodes.forEach((node) => {
      edges.push({
        id: `e-core-${node.id}`,
        source: "hub-core",
        target: node.id,
        active: node.status === "cyan",
      });
    });

    messages.forEach((msg, idx) => {
      if (msg.from_agent && msg.to_agent && msg.from_agent !== msg.to_agent) {
        edges.push({
          id: `e-msg-${idx}`,
          source: msg.from_agent,
          target: msg.to_agent,
          active: msg.status === "delivered",
        });
      }
    });

    return { nodes: [coreNode, ...agentNodes], edges };
  }, [agents, metrics, budgets, tasks, messages, wakes]);

  // Map agent metrics into multi-axis radar
  const radarData = useMemo(() => {
    const axes: RadarAxis[] = [
      { key: "lines", label: "Lines Written", max: 500 },
      { key: "tokens", label: "Tokens Used", max: 50000 },
      { key: "cached", label: "Cached Tokens", max: 20000 },
      { key: "calls", label: "Provider Calls", max: 50 },
      { key: "budget", label: "Budget Spent", max: 100, unit: "%" },
    ];

    const colors = [NOC_COLORS.cyan, NOC_COLORS.purple, NOC_COLORS.emerald, NOC_COLORS.amber, NOC_COLORS.pink];

    const series: RadarSeries[] = agents.filter((agent) => metrics.some((metric) => metric.agent_id === agent.id)).map((agent, i) => {
      const metric = metrics.find((m) => m.agent_id === agent.id);
      const budget = budgets.find((b) => b.agent_id === agent.id);
      const budgetPct = budget ? Math.min(100, (budget.spent_units / (budget.limit_units || 1)) * 100) : 0;

      return {
        id: agent.id,
        name: agent.display_name,
        color: colors[i % colors.length],
        values: {
          lines: metric?.lines_written ?? 0,
          tokens: metric?.tokens_used ?? 0,
          cached: metric?.tokens_cached ?? 0,
          calls: metric?.provider_calls ?? 0,
          budget: budgetPct,
        },
      };
    });

    return { axes, series };
  }, [agents, metrics, budgets]);

  // Map agents & tasks into honeycomb matrix
  const honeycombData = useMemo(() => {
    const cells: HexCellData[] = agents.map((agent, idx) => {
      const metric = metrics.find((m) => m.agent_id === agent.id);
      const intensity = metric ? Math.min(95, Math.max(0, Math.round((metric.tokens_used / 10000) * 100))) : 0;
      const col = 2 + (idx % 8);
      const row = 1 + Math.floor(idx / 8);

      return {
        col,
        row,
        intensity,
        label: agent.display_name.slice(0, 4).toUpperCase(),
        service: agent.display_name,
        status: intensity > 80 ? "warn" : intensity > 40 ? "active" : "ok",
        metric: metric ? `${metric.lines_written} lines` : "Unavailable",
      };
    });

    const trajectories: HexTrajectory[] = [];
    if (cells.length >= 2) {
      trajectories.push({
        id: "traj-live-1",
        from: [cells[0].col, cells[0].row],
        to: [cells[1].col, cells[1].row],
        color: NOC_COLORS.cyan,
      });
    }

    return { cells, trajectories };
  }, [agents, metrics]);

  // Map aggregate health into radial arcs
  const radialData = useMemo(() => {
    const totalLines = metrics.reduce((acc, m) => acc + (m.lines_written || 0), 0);
    const totalTokens = metrics.reduce((acc, m) => acc + (m.tokens_used || 0), 0);
    const activeTasks = tasks.filter((t) => t.status === "running").length;
    const taskPct = tasks.length > 0 ? Math.round((activeTasks / tasks.length) * 100) : 0;

    const rings: RadialRing[] = [
      { id: "r1", label: "Active Task Load", percent: taskPct, color: NOC_COLORS.cyan, valueText: `${activeTasks} / ${tasks.length} active` },
      { id: "r2", label: "Token Quota Footprint", percent: Math.min(100, Math.round((totalTokens / 100000) * 100)), color: NOC_COLORS.purple, valueText: `${totalTokens.toLocaleString()} tokens` },
      { id: "r3", label: "Code Output Volume", percent: Math.min(100, Math.round((totalLines / 1000) * 100)), color: NOC_COLORS.emerald, valueText: `${totalLines} lines` },
      { id: "r4", label: "Inter-Agent Channels", percent: Math.min(100, messages.length * 10), color: NOC_COLORS.amber, valueText: `${messages.length} messages` },
    ];

    return rings;
  }, [metrics, tasks, messages]);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }} data-testid="dashboard-orion-topology">
      <div
        style={{
          ...nocCardDenseStyle,
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          gap: "1rem",
          flexWrap: "wrap",
        }}
      >
        <div>
          <span style={{ fontSize: "0.85rem", fontWeight: 700, color: NOC_COLORS.cyan }}>
            Orion Telemetry Topology & Multi-Agent Matrix
          </span>
          <p style={{ margin: "0.15rem 0 0", fontSize: "0.74rem", color: NOC_SURFACES.textMuted }}>
            Interactive constellation graph, radar performance axes, and honeycomb telemetry for workspace agents.
          </p>
        </div>

        <div style={{ display: "flex", gap: "0.35rem" }}>
          {[
            { id: "constellation", label: "Constellation Graph" },
            { id: "radar", label: "Radar Comparison" },
            { id: "honeycomb", label: "Honeycomb Matrix" },
            { id: "rings", label: "Radial Arcs" },
          ].map((tab) => (
            <button
              key={tab.id}
              type="button"
              onClick={() => setActiveView(tab.id as typeof activeView)}
              aria-pressed={activeView === tab.id}
              style={{
                background: activeView === tab.id ? "rgba(56, 189, 248, 0.2)" : "transparent",
                border: `1px solid ${activeView === tab.id ? NOC_COLORS.cyan : NOC_SURFACES.border}`,
                borderRadius: "4px",
                padding: "0.25rem 0.6rem",
                color: activeView === tab.id ? "#ffffff" : NOC_SURFACES.textMuted,
                fontSize: "0.74rem",
                fontWeight: activeView === tab.id ? 700 : 500,
                cursor: "pointer",
              }}
            >
              {tab.label}
            </button>
          ))}
        </div>
      </div>

      <div>
        {agents.length === 0 && activeView !== "rings" ? (
          <p role="status">No workspace agents enrolled.</p>
        ) : null}
        {activeView === "constellation" && agents.length > 0 && (
          <OrionConstellationGraph
            title="Live Workspace Constellation"
            subtitle="Inter-agent communications, coordinator nexus, and task links"
            nodes={constellationData.nodes}
            edges={constellationData.edges}
          />
        )}

        {activeView === "radar" && agents.length > 0 && (
          <>
            {radarData.series.length < agents.length && (
              <p role="status">Metrics unavailable for some agents; only agents with recorded metrics are shown.</p>
            )}
            <OrionRadarChart
              title="Agent Capability & Output Radar"
              subtitle="Cross-agent comparison across lines written, token volume, and budget spent"
              axes={radarData.axes}
              series={radarData.series}
            />
          </>
        )}

        {activeView === "honeycomb" && agents.length > 0 && (
          <OrionHoneycombMap
            title="Agent Situation Honeycomb"
            subtitle="Hex partition matrix of active workspace runtimes"
            cells={honeycombData.cells}
            trajectories={honeycombData.trajectories}
          />
        )}

        {activeView === "rings" && (
          <OrionRadialRingMeter
            title="Workspace Utilization Arcs"
            subtitle="Composite multi-tier load metrics for active tasks and tokens"
            rings={radialData}
            centerLabel="Workspace"
            centerValue={`${agents.length} Agents`}
            centerSubtext="Orchestration Ready"
          />
        )}
      </div>
    </div>
  );
}
