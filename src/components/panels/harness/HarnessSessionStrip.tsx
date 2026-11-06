import HarnessBadge from "./HarnessBadge";
import { HARNESS_PREREQUISITES, type HarnessSessionRegistration } from "./types";

export default function HarnessSessionStrip({
  sessions,
  workspace,
}: {
  sessions: HarnessSessionRegistration[];
  workspace: string;
}) {
  const relevant = sessions.filter((row) => !workspace || row.workspace === workspace);
  return (
    <div style={{ padding: "0.55rem 1rem", borderBottom: "1px solid var(--border-color)", background: "rgba(15, 23, 42, 0.55)", display: "flex", flexWrap: "wrap", gap: "0.55rem", alignItems: "center" }}>
      {relevant.length === 0 && (
        <span style={{ color: "var(--text-muted)", fontSize: "0.78rem" }}>
          No registered harness for this workspace. Set up ownership in Orchestrate.
        </span>
      )}
      {relevant.map((row) => (
        <span key={`${row.harness}:${row.workspace}`} title={HARNESS_PREREQUISITES[row.harness] ?? ""} style={{ display: "inline-flex", alignItems: "center", gap: "0.4rem", fontSize: "0.78rem", color: "var(--text-main)" }}>
          <strong>{row.harness}</strong>
          <HarnessBadge mode={row.mode} state={row.state} />
        </span>
      ))}
    </div>
  );
}
