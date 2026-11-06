//! Oversight sections for the Team Scorecard tab (T7 / #347): advisory
//! consistency probes and the reviewer-side signature. Descriptive only —
//! no composite scores, no latency alerts, nothing an agent can game.

import React from "react";
import type { ScorecardProbe, ReviewerSignatureRow, ProbeState, FigureProvenance, ScorecardFigure } from "./types";
import { nocCardDenseStyle, nocPanelStyle } from "../../common/telemetry/panel";
import { EmptyStateChip } from "../../common/telemetry/status";
import { NOC_COLORS, NOC_SURFACES } from "../../common/telemetry/tokens";

const stateChip = (state: ProbeState): React.CSSProperties => ({
  display: "inline-block",
  padding: "0.15rem 0.5rem",
  borderRadius: "9999px",
  fontSize: "0.72rem",
  fontWeight: 600,
  textTransform: "uppercase",
  ...(state === "ok"
    ? { background: "rgba(16, 185, 129, 0.15)", color: "#10b981", border: "1px solid rgba(16, 185, 129, 0.35)" }
    : state === "disagreement"
      ? { background: "rgba(239, 68, 68, 0.15)", color: "#f87171", border: "1px solid rgba(239, 68, 68, 0.35)" }
      : { background: "rgba(148, 163, 184, 0.15)", color: "#94a3b8", border: "1px solid rgba(148, 163, 184, 0.35)" }),
});

function figureText(figure: ScorecardFigure, digits = 2): string {
  if (figure.value == null) return "unknown";
  return `${figure.value.toLocaleString(undefined, { maximumFractionDigits: digits })} ${figure.unit}`;
}

function ProvenanceChip({ prov }: { prov: FigureProvenance }) {
  const color = prov === "measured" ? "#10b981" : prov === "estimated" ? "#f59e0b" : "#94a3b8";
  return (
    <span style={{ fontSize: "0.68rem", color, textTransform: "uppercase", fontWeight: 600 }}>
      {prov}
    </span>
  );
}

export function ScorecardOversight({ probes, reviewers }: { probes?: ScorecardProbe[]; reviewers?: ReviewerSignatureRow[] }) {
  return (
    <>
      {/* Section: Consistency Probes */}
      <div>
        <div style={{ display: "flex", alignItems: "baseline", gap: "0.5rem", marginBottom: "0.75rem" }}>
          <h4 style={{ margin: 0, fontSize: "1rem", color: "var(--text-color, #fff)" }}>Consistency Probes</h4>
          <span style={{ fontSize: "0.8rem", color: "var(--text-muted, #94a3b8)" }}>
            Advisory cross-checks over stamped rows — unknown means a source is missing, never "all clear"
          </span>
        </div>
        {probes && probes.length > 0 ? (
          <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem" }}>
            {probes.map((probe, idx) => (
              <div
                key={`${probe.probe}-${probe.subject}-${idx}`}
                style={{
                  ...nocCardDenseStyle,
                  display: "flex",
                  alignItems: "flex-start",
                  gap: "0.75rem",
                  padding: "0.65rem 0.85rem",
                  borderLeft:
                    probe.state === "disagreement"
                      ? `3px solid ${NOC_COLORS.crit}`
                      : probe.state === "ok"
                        ? `3px solid ${NOC_COLORS.ok}`
                        : `3px solid ${NOC_COLORS.idle}`,
                  fontSize: "0.82rem",
                }}
              >
                <span style={stateChip(probe.state)}>{probe.state}</span>
                <div style={{ flex: 1 }}>
                  <div>
                    <code style={{ color: NOC_COLORS.cyan }}>{probe.probe}</code>
                    <span style={{ color: NOC_SURFACES.textMuted }}> · {probe.subject}</span>
                  </div>
                  {probe.reason && (
                    <div style={{ color: probe.state === "disagreement" ? "#fca5a5" : NOC_SURFACES.textMuted, marginTop: "0.15rem" }}>
                      {probe.reason}
                    </div>
                  )}
                </div>
                {probe.source_event_ids.length > 0 && (
                  <span className="tabular-nums" style={{ color: NOC_SURFACES.textMuted, whiteSpace: "nowrap" }}>
                    {probe.source_event_ids.length} source row{probe.source_event_ids.length > 1 ? "s" : ""}
                  </span>
                )}
              </div>
            ))}
          </div>
        ) : (
          <EmptyStateChip title="No probes visible for this caller" detail="Consistency probes compare stamped audit rows across sources." tone="idle" />
        )}
      </div>

      {/* Section: Reviewer Signature */}
      <div>
        <div style={{ display: "flex", alignItems: "baseline", gap: "0.5rem", marginBottom: "0.75rem" }}>
          <h4 style={{ margin: 0, fontSize: "1rem", color: NOC_SURFACES.textPrimary }}>Reviewer Signature</h4>
          <span style={{ fontSize: "0.8rem", color: NOC_SURFACES.textMuted }}>
            Descriptive figures per reviewer — never a single reviewer-quality score
          </span>
        </div>
        {reviewers && reviewers.length > 0 ? (
          <div style={{ ...nocPanelStyle, padding: 0, overflowX: "auto" }}>
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "0.82rem", textAlign: "left" }}>
              <thead>
                <tr style={{ borderBottom: "1px solid rgba(148, 163, 184, 0.16)", background: "rgba(9, 14, 26, 0.7)" }}>
                  <th style={{ padding: "0.6rem 1rem", color: NOC_SURFACES.textMuted, fontSize: "0.72rem", textTransform: "uppercase", letterSpacing: "0.05em" }}>Reviewer</th>
                  <th style={{ padding: "0.6rem 1rem", color: NOC_SURFACES.textMuted, fontSize: "0.72rem", textTransform: "uppercase", letterSpacing: "0.05em" }}>Findings</th>
                  <th style={{ padding: "0.6rem 1rem", color: NOC_SURFACES.textMuted, fontSize: "0.72rem", textTransform: "uppercase", letterSpacing: "0.05em" }}>Owner Corrections</th>
                  <th style={{ padding: "0.6rem 1rem", color: NOC_SURFACES.textMuted, fontSize: "0.72rem", textTransform: "uppercase", letterSpacing: "0.05em" }}>Findings / Documented Task</th>
                  <th style={{ padding: "0.6rem 1rem", color: NOC_SURFACES.textMuted, fontSize: "0.72rem", textTransform: "uppercase", letterSpacing: "0.05em" }}>Reversal Rate</th>
                  <th
                    title="Always unknown for now: gate decisions record no reviewer identity, and a send is not proof evidence was inspected"
                    style={{ padding: "0.6rem 1rem", color: NOC_SURFACES.textMuted, fontSize: "0.72rem", textTransform: "uppercase", letterSpacing: "0.05em" }}
                  >
                    Zero-evidence Approvals
                  </th>
                  <th style={{ padding: "0.6rem 1rem", color: NOC_SURFACES.textMuted, fontSize: "0.72rem", textTransform: "uppercase", letterSpacing: "0.05em" }}>Review Dwell</th>
                </tr>
              </thead>
              <tbody>
                {reviewers.map((reviewer) => (
                  <tr key={reviewer.reviewer_id} style={{ borderBottom: "1px solid rgba(148, 163, 184, 0.08)" }}>
                    <td style={{ padding: "0.6rem 1rem", fontWeight: 700, color: NOC_SURFACES.textPrimary }}>
                      {reviewer.display_name}
                      <div style={{ fontSize: "0.72rem", color: NOC_SURFACES.textMuted }}>
                        <code>{reviewer.reviewer_id}</code>
                      </div>
                    </td>
                    <td className="tabular-nums" style={{ padding: "0.6rem 1rem" }}>
                      {figureText(reviewer.findings_recorded, 0)} <ProvenanceChip prov={reviewer.findings_recorded.provenance} />
                    </td>
                    <td className="tabular-nums" style={{ padding: "0.6rem 1rem" }}>
                      {figureText(reviewer.findings_corrected_by_owner, 0)} <ProvenanceChip prov={reviewer.findings_corrected_by_owner.provenance} />
                    </td>
                    <td className="tabular-nums" style={{ padding: "0.6rem 1rem" }}>
                      {figureText(reviewer.findings_per_task)} <ProvenanceChip prov={reviewer.findings_per_task.provenance} />
                    </td>
                    <td className="tabular-nums" style={{ padding: "0.6rem 1rem" }}>
                      {figureText(reviewer.owner_override_reversal_rate)} <ProvenanceChip prov={reviewer.owner_override_reversal_rate.provenance} />
                    </td>
                    <td className="tabular-nums" style={{ padding: "0.6rem 1rem" }}>
                      {figureText(reviewer.zero_evidence_approvals, 0)} <ProvenanceChip prov={reviewer.zero_evidence_approvals.provenance} />
                    </td>
                    <td className="tabular-nums" style={{ padding: "0.6rem 1rem" }}>
                      {figureText(reviewer.review_dwell, 0)} <ProvenanceChip prov={reviewer.review_dwell.provenance} />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <EmptyStateChip
            title="Reviewer signature is private to the owner"
            detail="Or no reviewers are enrolled yet."
            tone="idle"
          />
        )}
      </div>
    </>
  );
}
