import HarnessBadge from "./HarnessBadge";
import type { HarnessSessionRegistration } from "./types";

export default function LiveTerminalCard({
  harness,
  session,
  signal,
  pid,
  busy,
  onResume,
  onKill,
}: {
  harness: string;
  session: HarnessSessionRegistration | null;
  signal: string;
  pid: number | null;
  busy: boolean;
  onResume: () => void;
  onKill: () => void;
}) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "0.7rem", padding: "0.9rem 1rem", borderRadius: "10px", border: "1px solid rgba(52, 211, 153, 0.45)", background: "rgba(0,0,0,0.28)", minHeight: "148px" }}>
      <div style={{ display: "flex", justifyContent: "space-between", gap: "0.6rem", flexWrap: "wrap", alignItems: "center" }}>
        <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", flexWrap: "wrap" }}>
          <strong style={{ color: "var(--text-main)", textTransform: "capitalize" }}>{harness}</strong>
          <span
            role="status"
            style={{ display: "inline-flex", alignItems: "center", gap: "0.3rem", padding: "0.12rem 0.5rem", borderRadius: "999px", fontSize: "0.7rem", fontWeight: 700, letterSpacing: "0.03em", textTransform: "uppercase", color: "#6ee7b7", border: "1px solid rgba(16, 185, 129, 0.35)", background: "rgba(16, 185, 129, 0.15)" }}
          >
            ● Live
          </span>
        </div>
        {session && <HarnessBadge mode={session.mode} state={session.state} />}
      </div>
      <div style={{ color: "var(--text-muted)", fontSize: "0.8rem", lineHeight: 1.45 }}>
        <div>{signal}</div>
        {session && <div>thread {session.disk_session_id}</div>}
        {pid != null && <div>pid {pid}</div>}
        {session?.writer_owner && <div>writer {session.writer_owner}</div>}
        {!session && <div>No Hub registration — liveness is the live process, not a row.</div>}
      </div>
      <div style={{ display: "flex", gap: "0.45rem", flexWrap: "wrap", marginTop: "auto" }}>
        <button
          type="button"
          className="btn-secondary"
          style={{ marginTop: 0 }}
          disabled={busy}
          title="Kill the managed pid if one is registered, then resume this harness in a real terminal."
          onClick={onResume}
        >
          {busy ? "Working…" : "Resume in terminal"}
        </button>
        <button
          type="button"
          className="btn-secondary"
          style={{ marginTop: 0, background: "rgba(239, 68, 68, 0.14)", color: "#fecaca", borderColor: "rgba(248, 113, 113, 0.55)" }}
          disabled={busy}
          title="Stop the process that actually backs liveness for this harness. Does not open a replacement."
          onClick={onKill}
        >
          Kill
        </button>
      </div>
    </div>
  );
}
