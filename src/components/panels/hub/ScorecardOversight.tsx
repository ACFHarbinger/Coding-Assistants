//! Oversight sections for the Team Scorecard tab (T7 / #347): advisory
//! consistency probes and the reviewer-side signature. Descriptive only —
//! no composite scores, no latency alerts, nothing an agent can game.

import React from "react";
import type { ScorecardProbe, ReviewerSignatureRow, ProbeState, FigureProvenance, ScorecardFigure } from "./types";

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
                  display: "flex",
                  alignItems: "flex-start",
                  gap: "0.75rem",
                  padding: "0.6rem 0.8rem",
                  borderRadius: "6px",
                  background: "rgba(255, 255, 255, 0.02)",
                  border:
                    probe.state === "disagreement"
                      ? "1px solid rgba(239, 68, 68, 0.35)"
                      : "1px solid rgba(255, 255, 255, 0.08)",
                  fontSize: "0.82rem",
                }}
              >
                <span style={stateChip(probe.state)}>{probe.state}</span>
                <div style={{ flex: 1 }}>
                  <div>
                    <code>{probe.probe}</code>
                    <span style={{ color: "var(--text-muted, #94a3b8)" }}> · {probe.subject}</span>
                  </div>
                  {probe.reason && (
                    <div style={{ color: probe.state === "disagreement" ? "#fca5a5" : "var(--text-muted, #94a3b8)", marginTop: "0.15rem" }}>
                      {probe.reason}
                    </div>
                  )}
                </div>
                {probe.source_event_ids.length > 0 && (
                  <span style={{ color: "var(--text-muted, #94a3b8)", whiteSpace: "nowrap" }}>
                    {probe.source_event_ids.length} source row{probe.source_event_ids.length > 1 ? "s" : ""}
                  </span>
                )}
              </div>
            ))}
          </div>
        ) : (
          <div style={{ color: "var(--text-muted, #94a3b8)", fontSize: "0.85rem" }}>
            No probes visible for this caller.
          </div>
        )}
      </div>

      {/* Section: Reviewer Signature */}
      <div>
        <div style={{ display: "flex", alignItems: "baseline", gap: "0.5rem", marginBottom: "0.75rem" }}>
          <h4 style={{ margin: 0, fontSize: "1rem", color: "var(--text-color, #fff)" }}>Reviewer Signature</h4>
          <span style={{ fontSize: "0.8rem", color: "var(--text-muted, #94a3b8)" }}>
            Descriptive figures per reviewer — never a single reviewer-quality score
          </span>
        </div>
        {reviewers && reviewers.length > 0 ? (
          <div style={{ overflowX: "auto", background: "rgba(255, 255, 255, 0.02)", borderRadius: "8px", border: "1px solid rgba(255, 255, 255, 0.08)" }}>
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "0.82rem", textAlign: "left" }}>
              <thead>
                <tr style={{ borderBottom: "1px solid rgba(255, 255, 255, 0.1)", background: "rgba(255, 255, 255, 0.02)" }}>
                  <th style={{ padding: "0.6rem 1rem", color: "var(--text-muted, #94a3b8)" }}>Reviewer</th>
                  <th style={{ padding: "0.6rem 1rem", color: "var(--text-muted, #94a3b8)" }}>Findings</th>
                  <th style={{ padding: "0.6rem 1rem", color: "var(--text-muted, #94a3b8)" }}>Owner Corrections</th>
                  <th style={{ padding: "0.6rem 1rem", color: "var(--text-muted, #94a3b8)" }}>Fix-find Rate</th>
                  <th style={{ padding: "0.6rem 1rem", color: "var(--text-muted, #94a3b8)" }}>Reversal Rate</th>
                  <th style={{ padding: "0.6rem 1rem", color: "var(--text-muted, #94a3b8)" }}>Zero-evidence Approvals</th>
                  <th style={{ padding: "0.6rem 1rem", color: "var(--text-muted, #94a3b8)" }}>Review Dwell</th>
                </tr>
              </thead>
              <tbody>
                {reviewers.map((reviewer) => (
                  <tr key={reviewer.reviewer_id} style={{ borderBottom: "1px solid rgba(255, 255, 255, 0.04)" }}>
                    <td style={{ padding: "0.6rem 1rem", fontWeight: 600 }}>
                      {reviewer.display_name}
                      <div style={{ fontSize: "0.72rem", color: "var(--text-muted, #94a3b8)" }}>
                        <code>{reviewer.reviewer_id}</code>
                      </div>
                    </td>
                    <td style={{ padding: "0.6rem 1rem" }}>
                      {figureText(reviewer.findings_recorded, 0)} <ProvenanceChip prov={reviewer.findings_recorded.provenance} />
                    </td>
                    <td style={{ padding: "0.6rem 1rem" }}>
                      {figureText(reviewer.findings_corrected_by_owner, 0)} <ProvenanceChip prov={reviewer.findings_corrected_by_owner.provenance} />
                    </td>
                    <td style={{ padding: "0.6rem 1rem" }}>
                      {figureText(reviewer.fix_find_rate)} <ProvenanceChip prov={reviewer.fix_find_rate.provenance} />
                    </td>
                    <td style={{ padding: "0.6rem 1rem" }}>
                      {figureText(reviewer.owner_override_reversal_rate)} <ProvenanceChip prov={reviewer.owner_override_reversal_rate.provenance} />
                    </td>
                    <td style={{ padding: "0.6rem 1rem" }}>
                      {figureText(reviewer.zero_evidence_approvals, 0)} <ProvenanceChip prov={reviewer.zero_evidence_approvals.provenance} />
                    </td>
                    <td style={{ padding: "0.6rem 1rem" }}>
                      {figureText(reviewer.review_dwell, 0)} <ProvenanceChip prov={reviewer.review_dwell.provenance} />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <div style={{ color: "var(--text-muted, #94a3b8)", fontSize: "0.85rem" }}>
            Reviewer signature is private to the owner, or no reviewers are enrolled yet.
          </div>
        )}
      </div>
    </>
  );
}
