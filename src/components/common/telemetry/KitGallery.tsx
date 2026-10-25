import { useState } from "react";
import { TelemetryTimeSeriesCard } from "./charts";
import { ConcentricArcGauge, Meter, ProvenanceBadge } from "./gauges";
import { nocCardDenseStyle, nocPanelStyle, nocSubsurfaceStyle } from "./panel";
import { SubtabBar } from "./SubtabBar";
import { EmptyStateChip, HudStatusPill, KpiCardGrid, MonitorTileStrip, StalenessDot } from "./status";
import { NOC_COLORS, NOC_SPACING, NOC_SURFACES } from "./tokens";
import { KitGalleryOrionSection } from "./KitGalleryOrionSection";

/**
 * Dev-only visual & state acceptance surface (`MISTRAL-UI-4` / `CODEX-UI-4`, V2 / #373).
 * Renders every telemetry kit primitive across severity (`ok`/`warn`/`crit`/`idle`),
 * provenance (`measured`/`estimated`/`unknown`), staleness (`fresh`/`stale`/`awaiting`),
 * interaction states, and the SetProduct Orion visualization suite.
 */
export function KitGallery({ forceRender = false }: { forceRender?: boolean }) {
  const [activeDemoTab, setDemoTab] = useState<"monitoring" | "orion" | "provenance" | "states">("monitoring");

  if (!import.meta.env.DEV && !forceRender) {
    return null;
  }

  return (
    <div
      data-testid="noc-kit-gallery"
      style={{
        ...nocPanelStyle,
        display: "flex",
        flexDirection: "column",
        gap: NOC_SPACING.space4,
      }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "1rem", flexWrap: "wrap" }}>
        <div>
          <h3 style={{ margin: 0, fontSize: "1rem", color: NOC_SURFACES.textPrimary }}>
            NOC Telemetry Kit Gallery (Dev Reference)
          </h3>
          <p style={{ margin: "0.2rem 0 0", fontSize: "0.78rem", color: NOC_SURFACES.textMuted }}>
            Severity tones, provenance tiers, staleness indicators, and Orion visual primitives.
          </p>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: "0.75rem", flexWrap: "wrap" }}>
          <StalenessDot lastUpdatedUnixSeconds={1000} nowUnixSeconds={1002} pollIntervalSeconds={3} />
          <StalenessDot lastUpdatedUnixSeconds={1000} nowUnixSeconds={1012} pollIntervalSeconds={3} />
          <StalenessDot lastUpdatedUnixSeconds={null} />
          <SubtabBar
            ariaLabel="Kit gallery sections"
            activeId={activeDemoTab}
            onSelect={(id) => setDemoTab(id as "monitoring" | "orion" | "provenance" | "states")}
            tabs={[
              { id: "monitoring", label: "Gauges & Charts" },
              { id: "orion", label: "Orion Visualizations", badgeCount: 6 },
              { id: "provenance", label: "Provenance & Meters", badgeCount: 3 },
              { id: "states", label: "Empty / Error / Loading" },
            ]}
          />
        </div>
      </div>

      {activeDemoTab === "orion" ? (
        <KitGalleryOrionSection />
      ) : (
        <>
          <div style={{ display: "flex", gap: "0.65rem", flexWrap: "wrap" }}>
            <HudStatusPill label="Workspace root" value="Coding-Assistants" title="/repo/Coding-Assistants" tone="ok" onClick={() => undefined} />
            <HudStatusPill label="Active team chat" value="UI Polish Sprint" tone="cyan" onClick={() => undefined} />
            <HudStatusPill label="Pending gates" value="2 awaiting approval" tone="warn" />
            <HudStatusPill label="Offline bridge" value="Standby" tone="idle" />
          </div>

          <MonitorTileStrip
            title="Severity Tile Matrix (ok / warn / crit / idle)"
            tiles={[
              { id: "t-ok", title: "Node-OK", subtitle: "34% load", percent: 34, tone: "ok" },
              { id: "t-warn", title: "Node-Warn", subtitle: "78% load", percent: 78, tone: "warn" },
              { id: "t-crit", title: "Node-Crit", subtitle: "95% load", percent: 95, tone: "crit" },
              { id: "t-idle", title: "Node-Idle", subtitle: "Standby", available: false, tone: "idle" },
            ]}
          />

          <KpiCardGrid
            cards={[
              { title: "Measured Tokens", subtitle: "Direct HTTP", value: "128.4", unit: "k", footer: "14 requests", accent: NOC_COLORS.cyan, provenance: "measured" },
              { title: "Estimated Quota", subtitle: "Window delta", value: "64.0", unit: "%", footer: "5h window", accent: NOC_COLORS.amber, provenance: "estimated" },
              { title: "Unknown Cap", subtitle: "Unreadable /spend-limit", value: "999", unit: "USD", footer: "Spend cap unavailable", provenance: "unknown", unknown: true },
            ]}
          />

          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(240px, 1fr))", gap: NOC_SPACING.space4 }}>
            <div style={nocSubsurfaceStyle}>
              <ConcentricArcGauge
                title="Dual Window"
                size={136}
                centerPrimary="42%"
                centerSecondary="5h / 7d"
                rings={[
                  { label: "5h Window", percent: 42, color: NOC_COLORS.cyan, detail: "42% used" },
                  { label: "Unknown Cap", percent: 85, color: NOC_COLORS.ok, detail: "unavailable", unknown: true },
                ]}
              />
            </div>

            <div style={{ ...nocSubsurfaceStyle, display: "flex", flexDirection: "column", justifyContent: "center", gap: "0.7rem" }}>
              <div style={{ display: "flex", gap: "0.4rem", flexWrap: "wrap" }}>
                <ProvenanceBadge provenance="measured" />
                <ProvenanceBadge provenance="estimated" />
                <ProvenanceBadge provenance="unknown" />
              </div>
              <Meter label="Measured Utilization" percent={62} detail="62.0%" tone="ok" provenance="measured" />
              <Meter label="Elevated Quota" percent={81} detail="81.0%" warnThreshold={75} critThreshold={90} provenance="estimated" />
              <Meter label="Unreadable Spend Cap" percent={50} detail="$12.00" provenance="unknown" unknown />
            </div>

            <TelemetryTimeSeriesCard
              title="Sample Telemetry Series"
              unitLabel="%"
              fixedMax={100}
              showAverageLine
              timestamps={[1700000000, 1700000003, 1700000006, 1700000009]}
              series={[
                { label: "Primary", color: NOC_COLORS.cyan, fill: "rgba(56, 189, 248, 0.2)", values: [24, 38, 52, 46] },
              ]}
            />
          </div>

          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(240px, 1fr))", gap: NOC_SPACING.space3 }}>
            <EmptyStateChip title="No pending approvals" detail="All task and tool gates are clear." tone="ok" />
            <EmptyStateChip
              title="Approvals could not be loaded"
              detail="Hub connection timed out; showing last cached snapshot."
              tone="crit"
              actionLabel="Retry"
              onAction={() => undefined}
            />
            <div style={{ ...nocCardDenseStyle, display: "flex", alignItems: "center", justifyContent: "space-between", gap: "0.5rem" }}>
              <span style={{ fontSize: "0.78rem", color: NOC_SURFACES.textSecondary }}>Refreshing snapshot (prior data preserved)…</span>
              <button type="button" className="btn-primary noc-focusable" disabled style={{ padding: "0.35rem 0.75rem", fontSize: "0.75rem" }}>
                Disabled Action
              </button>
            </div>
          </div>
        </>
      )}
    </div>
  );
}
