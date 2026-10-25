import React from "react";
import type { DigestResolutionCounts, DigestSessionGroup, OwnerDigest } from "./types";
import { nocCardDenseStyle, nocPanelStyle } from "../../common/telemetry/panel";
import { EmptyStateChip } from "../../common/telemetry/status";
import { NOC_COLORS, NOC_SURFACES } from "../../common/telemetry/tokens";

function formatAge(ageSeconds: number | null | undefined): string {
  if (ageSeconds == null) return "unknown age";
  const bounded = Math.max(ageSeconds, 0);
  const minutes = Math.floor(bounded / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);
  if (days > 0) return `${days}d ${hours % 24}h`;
  if (hours > 0) return `${hours}h ${minutes % 60}m`;
  if (minutes > 0) return `${minutes}m`;
  return `${bounded}s`;
}

function kindChip(kind: string): React.ReactElement {
  const palette: Record<string, { background: string; color: string }> = {
    wake: { background: "rgba(56, 189, 248, 0.15)", color: "#38bdf8" },
    gate_approval: { background: "rgba(168, 85, 247, 0.15)", color: "#c084fc" },
    os_tool: { background: "rgba(245, 158, 11, 0.15)", color: "#f59e0b" },
  };
  const tone = palette[kind] ?? { background: "rgba(148, 163, 184, 0.15)", color: "#94a3b8" };
  return (
    <span
      style={{
        padding: "0.1rem 0.4rem",
        borderRadius: "4px",
        fontSize: "0.7rem",
        fontWeight: 600,
        background: tone.background,
        color: tone.color,
        border: `1px solid ${tone.color}33`,
      }}
    >
      {kind}
    </span>
  );
}

function viaChips(counts: DigestResolutionCounts): React.ReactElement {
  const parts: { label: string; value: number }[] = [
    { label: "desktop", value: counts.desktop },
    { label: "telegram", value: counts.telegram },
    { label: "tcp", value: counts.tcp },
  ];
  return (
    <span style={{ fontSize: "0.75rem", color: "var(--text-muted, #94a3b8)" }}>
      {parts
        .filter((part) => part.value > 0)
        .map((part) => `${part.value} ${part.label}`)
        .join(" · ") || "none"}
    </span>
  );
}

function summaryStat(label: string, value: string, tone?: string): React.ReactElement {
  return (
    <div
      style={{
        ...nocCardDenseStyle,
        borderLeft: `3px solid ${tone ?? NOC_COLORS.indigo}`,
        padding: "0.6rem 0.9rem",
        minWidth: "140px",
        flex: "1 1 140px",
      }}
    >
      <div style={{ fontSize: "0.68rem", color: NOC_SURFACES.textMuted, textTransform: "uppercase", letterSpacing: "0.05em", fontWeight: 700 }}>
        {label}
      </div>
      <div className="tabular-nums" style={{ fontSize: "1.1rem", fontWeight: 800, color: tone ?? NOC_SURFACES.textPrimary, marginTop: "0.15rem" }}>{value}</div>
    </div>
  );
}

function GroupCard({ group, windowDays }: { group: DigestSessionGroup; windowDays: number }): React.ReactElement {
  const title = group.work_session_name ?? (group.work_session_id ? "Unlinked session" : "Ungrouped");
  return (
    <div
      style={{
        ...nocCardDenseStyle,
        padding: "0.8rem 1rem",
      }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline", gap: "0.5rem", flexWrap: "wrap" }}>
        <div>
          <span style={{ fontWeight: 600, fontSize: "0.9rem" }}>{title}</span>
          {group.workspace_id && (
            <code style={{ marginLeft: "0.5rem", fontSize: "0.7rem", color: "var(--text-muted, #94a3b8)" }}>
              ws:{group.workspace_id.slice(0, 8)}
            </code>
          )}
        </div>
        <div style={{ display: "flex", gap: "0.75rem", alignItems: "baseline" }}>
          <span style={{ fontSize: "0.75rem", color: group.unresolved_findings > 0 ? "#f87171" : "var(--text-muted, #94a3b8)" }}>
            {group.unresolved_findings} open finding{group.unresolved_findings === 1 ? "" : "s"}
          </span>
          <span style={{ fontSize: "0.75rem", color: "var(--text-muted, #94a3b8)" }}>
            resolved ({windowDays}d):
          </span>
          {viaChips(group.resolved_window)}
        </div>
      </div>

      {group.pending.length > 0 ? (
        <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "0.8rem", marginTop: "0.5rem" }}>
          <thead>
            <tr style={{ color: "var(--text-muted, #94a3b8)", textAlign: "left" }}>
              <th style={{ padding: "0.3rem 0.5rem", fontWeight: 500 }}>Kind</th>
              <th style={{ padding: "0.3rem 0.5rem", fontWeight: 500 }}>Subject</th>
              <th style={{ padding: "0.3rem 0.5rem", fontWeight: 500 }}>Agent</th>
              <th style={{ padding: "0.3rem 0.5rem", fontWeight: 500 }}>Age</th>
              <th style={{ padding: "0.3rem 0.5rem", fontWeight: 500 }}>Task</th>
              <th style={{ padding: "0.3rem 0.5rem", fontWeight: 500 }}>Issue</th>
            </tr>
          </thead>
          <tbody>
            {group.pending.map((decision) => (
              <tr key={decision.id} style={{ borderTop: "1px solid rgba(255, 255, 255, 0.05)" }}>
                <td style={{ padding: "0.3rem 0.5rem" }}>{kindChip(decision.kind)}</td>
                <td style={{ padding: "0.3rem 0.5rem", maxWidth: "280px", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={decision.subject}>
                  {decision.subject}
                </td>
                <td style={{ padding: "0.3rem 0.5rem", color: "var(--text-muted, #94a3b8)" }}>{decision.agent ?? "—"}</td>
                <td style={{ padding: "0.3rem 0.5rem" }}>{formatAge(decision.age_seconds)}</td>
                <td style={{ padding: "0.3rem 0.5rem" }}>
                  {decision.task_id ? <code>{decision.task_id}</code> : <span style={{ color: "var(--text-muted, #94a3b8)" }}>—</span>}
                </td>
                <td style={{ padding: "0.3rem 0.5rem" }}>
                  {decision.github_issue ? <code>{decision.github_issue}</code> : <span style={{ color: "var(--text-muted, #94a3b8)" }}>—</span>}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : (
        <div style={{ fontSize: "0.8rem", color: "var(--text-muted, #94a3b8)", marginTop: "0.5rem" }}>
          No pending decisions in this group.
        </div>
      )}
    </div>
  );
}

export function ScorecardDigest({ digest }: { digest: OwnerDigest | null }): React.ReactElement | null {
  if (!digest) return null;
  const { summary, groups, window_days: windowDays } = digest;
  return (
    <div style={nocPanelStyle}>
      <div style={{ display: "flex", alignItems: "baseline", gap: "0.5rem", marginBottom: "0.65rem", flexWrap: "wrap" }}>
        <h4 style={{ margin: 0, fontSize: "1rem", color: NOC_SURFACES.textPrimary }}>Owner Review Digest</h4>
        <span style={{ fontSize: "0.8rem", color: NOC_SURFACES.textMuted }}>
          Private, advisory — approve and reject from Messager and Journal. Remote approvals share the same human clock, tagged by channel.
        </span>
      </div>

      <div style={{ display: "flex", gap: "0.75rem", flexWrap: "wrap", marginBottom: "0.75rem" }}>
        {summaryStat("Pending decisions", String(summary.pending_count), summary.pending_count > 0 ? NOC_COLORS.warn : NOC_COLORS.ok)}
        {summaryStat("Oldest pending", summary.oldest_pending_at ? new Date(summary.oldest_pending_at).toLocaleString() : "—", NOC_COLORS.cyan)}
        {summaryStat("Open findings", String(summary.unresolved_findings), summary.unresolved_findings > 0 ? NOC_COLORS.crit : NOC_COLORS.ok)}
        {summaryStat(`Resolved (${windowDays}d)`, String(summary.resolved_window.total), NOC_COLORS.purple)}
      </div>
      <div style={{ marginBottom: "0.75rem", fontSize: "0.75rem", color: NOC_SURFACES.textMuted }}>
        Channels: {viaChips(summary.resolved_window)}
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
        {groups.map((group, idx) => (
          <GroupCard key={group.work_session_id ?? `ungrouped-${idx}`} group={group} windowDays={windowDays} />
        ))}
        {groups.length === 0 && (
          <EmptyStateChip
            title="Nothing awaiting your review"
            detail="No pending decisions, findings, or recent resolutions."
            tone="ok"
          />
        )}
      </div>
    </div>
  );
}
