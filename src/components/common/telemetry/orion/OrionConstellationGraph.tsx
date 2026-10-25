import { useState, useId, useMemo } from "react";
import { nocCardDenseStyle, nocPanelStyle } from "../panel";
import { NOC_COLORS, NOC_SURFACES, NOC_TYPOGRAPHY } from "../tokens";

export interface ConstellationNode {
  id: string;
  label: string;
  role?: string;
  tier: 0 | 1 | 2; // 0 = core, 1 = primary satellite, 2 = outer peripheral
  angle?: number; // radians or degrees
  distance?: number; // distance from center (0 to 1)
  status: "ok" | "warn" | "crit" | "idle" | "cyan";
  metric?: string;
  metricLabel?: string;
  data?: Record<string, string | number>;
}

export interface ConstellationEdge {
  id: string;
  source: string;
  target: string;
  strength?: number;
  label?: string;
  active?: boolean;
}

export interface OrionConstellationGraphProps {
  title?: string;
  subtitle?: string;
  nodes: ConstellationNode[];
  edges: ConstellationEdge[];
  width?: number;
  height?: number;
  onNodeClick?: (node: ConstellationNode) => void;
}

export function OrionConstellationGraph({
  title = "Agent Constellation Topology",
  subtitle = "Hub & satellite network with live connection links",
  nodes,
  edges,
  width = 540,
  height = 420,
  onNodeClick,
}: OrionConstellationGraphProps) {
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [hoveredNodeId, setHoveredNodeId] = useState<string | null>(null);
  const filterId = useId();

  const cx = width / 2;
  const cy = height / 2;
  const maxRadius = Math.min(cx, cy) - 46;

  // Compute (x, y) coordinates for nodes based on tier and distribution
  const nodePositions = useMemo(() => {
    const map = new Map<string, { x: number; y: number; r: number }>();

    // Core nodes (tier 0) placed at center or tight inner cluster
    const tier0 = nodes.filter((n) => n.tier === 0);
    const tier1 = nodes.filter((n) => n.tier === 1);
    const tier2 = nodes.filter((n) => n.tier === 2);

    tier0.forEach((n, idx) => {
      if (tier0.length === 1) {
        map.set(n.id, { x: cx, y: cy, r: 24 });
      } else {
        const angle = (idx * 2 * Math.PI) / tier0.length;
        map.set(n.id, {
          x: cx + 32 * Math.cos(angle),
          y: cy + 32 * Math.sin(angle),
          r: 20,
        });
      }
    });

    tier1.forEach((n, idx) => {
      const angle = n.angle !== undefined ? (n.angle * Math.PI) / 180 : (idx * 2 * Math.PI) / tier1.length;
      const dist = n.distance !== undefined ? n.distance * maxRadius : maxRadius * 0.55;
      map.set(n.id, {
        x: cx + dist * Math.cos(angle),
        y: cy + dist * Math.sin(angle),
        r: 16,
      });
    });

    tier2.forEach((n, idx) => {
      const angle = n.angle !== undefined ? (n.angle * Math.PI) / 180 : (idx * 2 * Math.PI) / tier2.length;
      const dist = n.distance !== undefined ? n.distance * maxRadius : maxRadius * 0.88;
      map.set(n.id, {
        x: cx + dist * Math.cos(angle),
        y: cy + dist * Math.sin(angle),
        r: 12,
      });
    });

    return map;
  }, [nodes, cx, cy, maxRadius]);

  const activeFocusId = hoveredNodeId || selectedNodeId;

  // Determine connected edges and neighbor nodes
  const { connectedEdgeIds, connectedNodeIds } = useMemo(() => {
    if (!activeFocusId) return { connectedEdgeIds: new Set<string>(), connectedNodeIds: new Set<string>() };
    const eSet = new Set<string>();
    const nSet = new Set<string>([activeFocusId]);

    edges.forEach((e) => {
      if (e.source === activeFocusId || e.target === activeFocusId) {
        eSet.add(e.id);
        nSet.add(e.source);
        nSet.add(e.target);
      }
    });

    return { connectedEdgeIds: eSet, connectedNodeIds: nSet };
  }, [activeFocusId, edges]);

  const selectedNode = nodes.find((n) => n.id === (hoveredNodeId || selectedNodeId));

  const getStatusColor = (status: ConstellationNode["status"]) => {
    switch (status) {
      case "ok":
        return NOC_COLORS.ok;
      case "warn":
        return NOC_COLORS.warn;
      case "crit":
        return NOC_COLORS.crit;
      case "cyan":
        return NOC_COLORS.cyan;
      default:
        return NOC_COLORS.idle;
    }
  };

  return (
    <div
      style={{
        ...nocPanelStyle,
        position: "relative",
        display: "flex",
        flexDirection: "column",
        gap: "0.75rem",
      }}
      data-testid="orion-constellation-graph"
    >
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: "1rem", flexWrap: "wrap" }}>
        <div>
          <h4 style={{ margin: 0, fontSize: "0.92rem", fontWeight: 700, color: NOC_SURFACES.textPrimary, letterSpacing: "0.02em" }}>
            {title}
          </h4>
          <p style={{ margin: "0.15rem 0 0", fontSize: NOC_TYPOGRAPHY.micro, color: NOC_SURFACES.textMuted }}>
            {subtitle}
          </p>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: "0.6rem" }}>
          <span style={{ fontSize: NOC_TYPOGRAPHY.micro, color: NOC_SURFACES.textMuted }}>
            {nodes.length} Nodes &bull; {edges.length} Links
          </span>
          {selectedNodeId && (
            <button
              type="button"
              onClick={() => setSelectedNodeId(null)}
              style={{
                background: "rgba(148, 163, 184, 0.12)",
                border: `1px solid ${NOC_SURFACES.border}`,
                borderRadius: "4px",
                color: NOC_SURFACES.textSecondary,
                fontSize: "0.7rem",
                padding: "0.15rem 0.45rem",
                cursor: "pointer",
              }}
            >
              Reset View
            </button>
          )}
        </div>
      </div>

      <div style={{ position: "relative", width: "100%", overflow: "hidden" }}>
        <svg
          viewBox={`0 0 ${width} ${height}`}
          width="100%"
          height="100%"
          role="img"
          aria-label={title}
          style={{ display: "block", overflow: "visible" }}
        >
          <defs>
            <filter id={`constellation-glow-${filterId}`} x="-40%" y="-40%" width="180%" height="180%">
              <feGaussianBlur stdDeviation="4" result="blur" />
              <feComposite in="SourceGraphic" in2="blur" operator="over" />
            </filter>
            <radialGradient id={`constellation-bg-glow-${filterId}`} cx="50%" cy="50%" r="50%">
              <stop offset="0%" stopColor="#38bdf8" stopOpacity="0.14" />
              <stop offset="50%" stopColor="#6366f1" stopOpacity="0.06" />
              <stop offset="100%" stopColor="#090e1a" stopOpacity="0" />
            </radialGradient>
          </defs>

          {/* Background atmosphere radial gradient */}
          <circle cx={cx} cy={cy} r={maxRadius + 20} fill={`url(#constellation-bg-glow-${filterId})`} />

          {/* Orbit rings with radar-like tick rings */}
          {[0.28, 0.55, 0.88].map((lvl, idx) => (
            <g key={`orbit-${lvl}`}>
              <circle
                cx={cx}
                cy={cy}
                r={maxRadius * lvl}
                fill="none"
                stroke="rgba(148, 163, 184, 0.14)"
                strokeWidth={1}
                strokeDasharray={idx === 1 ? "4 4" : undefined}
              />
              {/* Subtle cardinal tick marks on the middle ring */}
              {idx === 1 &&
                [0, Math.PI / 2, Math.PI, (3 * Math.PI) / 2].map((ang, i) => {
                  const tx1 = cx + (maxRadius * lvl - 4) * Math.cos(ang);
                  const ty1 = cy + (maxRadius * lvl - 4) * Math.sin(ang);
                  const tx2 = cx + (maxRadius * lvl + 4) * Math.cos(ang);
                  const ty2 = cy + (maxRadius * lvl + 4) * Math.sin(ang);
                  return (
                    <line
                      key={`tick-${i}`}
                      x1={tx1}
                      y1={ty1}
                      x2={tx2}
                      y2={ty2}
                      stroke="rgba(56, 189, 248, 0.4)"
                      strokeWidth={1.5}
                    />
                  );
                })}
            </g>
          ))}

          {/* Render connecting edges */}
          {edges.map((e) => {
            const srcPos = nodePositions.get(e.source);
            const tgtPos = nodePositions.get(e.target);
            if (!srcPos || !tgtPos) return null;

            const isHighlighted = connectedEdgeIds.has(e.id);
            const isDimmed = activeFocusId !== null && !isHighlighted;

            return (
              <g key={e.id} opacity={isDimmed ? 0.18 : 1} style={{ transition: "opacity 160ms ease" }}>
                <line
                  x1={srcPos.x}
                  y1={srcPos.y}
                  x2={tgtPos.x}
                  y2={tgtPos.y}
                  stroke={isHighlighted ? NOC_COLORS.cyan : "rgba(148, 163, 184, 0.24)"}
                  strokeWidth={isHighlighted ? 2.5 : 1.2}
                  strokeDasharray={e.active ? "5 4" : undefined}
                  filter={isHighlighted ? `url(#constellation-glow-${filterId})` : undefined}
                />
              </g>
            );
          })}

          {/* Render constellation nodes */}
          {nodes.map((node) => {
            const pos = nodePositions.get(node.id);
            if (!pos) return null;

            const isFocused = activeFocusId === node.id;
            const isConnected = connectedNodeIds.has(node.id);
            const isDimmed = activeFocusId !== null && !isConnected;
            const statusColor = getStatusColor(node.status);
            const isCore = node.tier === 0;

            return (
              <g
                key={node.id}
                tabIndex={0}
                role="button"
                aria-label={`${node.label} (${node.status})`}
                opacity={isDimmed ? 0.28 : 1}
                onClick={() => {
                  setSelectedNodeId(node.id === selectedNodeId ? null : node.id);
                  onNodeClick?.(node);
                }}
                onKeyDown={(evt) => {
                  if (evt.key === "Enter" || evt.key === " ") {
                    evt.preventDefault();
                    setSelectedNodeId(node.id === selectedNodeId ? null : node.id);
                    onNodeClick?.(node);
                  }
                }}
                onMouseEnter={() => setHoveredNodeId(node.id)}
                onMouseLeave={() => setHoveredNodeId(null)}
                style={{ cursor: "pointer", outline: "none", transition: "opacity 160ms ease" }}
              >
                {/* Outer luminous glow aura for focused or core nodes */}
                {(isCore || isFocused) && (
                  <circle
                    cx={pos.x}
                    cy={pos.y}
                    r={pos.r + 9}
                    fill={statusColor}
                    fillOpacity={isFocused ? 0.26 : 0.14}
                    filter={`url(#constellation-glow-${filterId})`}
                  />
                )}

                {/* Base node circle */}
                <circle
                  cx={pos.x}
                  cy={pos.y}
                  r={pos.r}
                  fill={isCore ? "rgba(9, 14, 26, 0.95)" : NOC_SURFACES.bgPanel}
                  stroke={statusColor}
                  strokeWidth={isFocused ? 2.5 : 1.5}
                />

                {/* Center dot/indicator */}
                <circle
                  cx={pos.x}
                  cy={pos.y}
                  r={isCore ? 7 : 4}
                  fill={statusColor}
                />

                {/* Node text label */}
                <text
                  x={pos.x}
                  y={pos.y + pos.r + 14}
                  textAnchor="middle"
                  fill={isFocused ? "#ffffff" : NOC_SURFACES.textPrimary}
                  fontSize={isCore ? 11 : 9.5}
                  fontWeight={isFocused || isCore ? 700 : 500}
                >
                  {node.label}
                </text>

                {/* Metric pill underneath label if present */}
                {node.metric && (
                  <text
                    x={pos.x}
                    y={pos.y + pos.r + 26}
                    textAnchor="middle"
                    fill={statusColor}
                    fontSize={8.5}
                    fontWeight={600}
                    className="tabular-nums"
                  >
                    {node.metric}
                  </text>
                )}
              </g>
            );
          })}
        </svg>

        {/* Selected / Hovered Node HUD telemetry card */}
        {selectedNode && (
          <div
            style={{
              ...nocCardDenseStyle,
              position: "absolute",
              top: "12px",
              right: "12px",
              background: "rgba(9, 14, 26, 0.92)",
              border: `1px solid ${getStatusColor(selectedNode.status)}`,
              boxShadow: "0 8px 24px rgba(0, 0, 0, 0.7)",
              minWidth: "170px",
              maxWidth: "240px",
              zIndex: 10,
              pointerEvents: "none",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: "0.5rem" }}>
              <span style={{ fontSize: "0.82rem", fontWeight: 700, color: NOC_SURFACES.textPrimary }}>
                {selectedNode.label}
              </span>
              <span
                style={{
                  fontSize: "0.68rem",
                  fontWeight: 700,
                  color: getStatusColor(selectedNode.status),
                  textTransform: "uppercase",
                }}
              >
                {selectedNode.status}
              </span>
            </div>
            {selectedNode.role && (
              <div style={{ fontSize: "0.72rem", color: NOC_SURFACES.textMuted, marginTop: "0.15rem" }}>
                Role: {selectedNode.role}
              </div>
            )}
            {selectedNode.metric && (
              <div style={{ fontSize: "0.74rem", color: NOC_COLORS.cyan, fontWeight: 600, marginTop: "0.3rem" }}>
                {selectedNode.metricLabel || "Metric"}: {selectedNode.metric}
              </div>
            )}
            {selectedNode.data && (
              <div style={{ marginTop: "0.4rem", display: "flex", flexDirection: "column", gap: "0.15rem", borderTop: `1px solid ${NOC_SURFACES.border}`, paddingTop: "0.35rem" }}>
                {Object.entries(selectedNode.data).map(([k, v]) => (
                  <div key={k} style={{ display: "flex", justifyContent: "space-between", fontSize: "0.68rem" }}>
                    <span style={{ color: NOC_SURFACES.textMuted }}>{k}</span>
                    <span className="tabular-nums" style={{ color: NOC_SURFACES.textSecondary, fontWeight: 600 }}>
                      {v}
                    </span>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
