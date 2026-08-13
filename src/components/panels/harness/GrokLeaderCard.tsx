import { useCallback, useEffect, useState } from "react";
import { invoke } from "../../../lib/tauri";
import type { ActiveGrokSession, GrokConnectResult } from "../hub/types";
import { HARNESS_PREREQUISITES } from "./types";

export default function GrokLeaderCard({
  workspace,
  compact = false,
}: {
  workspace: string;
  compact?: boolean;
}) {
  const [status, setStatus] = useState<GrokConnectResult | null>(null);
  const [live, setLive] = useState<ActiveGrokSession[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const refresh = useCallback(async () => {
    try {
      const next = await invoke<GrokConnectResult>("hub_grok_leader_status", {
        workspace: workspace || null,
      });
      setStatus(next);
      setLive(await invoke<ActiveGrokSession[]>("hub_grok_list_live_sessions"));
      setError("");
    } catch (cause) {
      setError(String(cause));
    }
  }, [workspace]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const connect = async (resume: boolean) => {
    if (!workspace.startsWith("/")) {
      setError("Set an absolute Workspace Root before connecting Grok.");
      return;
    }
    setBusy(true);
    try {
      const result = await invoke<GrokConnectResult>("hub_grok_connect", {
        workspace,
        resume,
      });
      setStatus(result);
      setLive(await invoke<ActiveGrokSession[]>("hub_grok_list_live_sessions"));
      setError(result.leader_live ? "" : result.detail);
    } catch (cause) {
      setError(String(cause).replace(/^Error:\s*/, ""));
    } finally {
      setBusy(false);
    }
  };

  const workspaceLive = live.filter((row) => !workspace || row.cwd === workspace);

  return (
    <div style={{ display: "grid", gap: "0.65rem" }}>
      {!compact && (
        <>
          <strong style={{ color: "var(--text-main)" }}>Grok leader</strong>
          <p style={{ margin: 0, color: "var(--text-muted)", fontSize: "0.82rem" }}>
            {HARNESS_PREREQUISITES.grok}
          </p>
        </>
      )}
      <div style={{ display: "flex", flexWrap: "wrap", gap: "0.5rem", alignItems: "center" }}>
        <span
          className="status-badge"
          style={{
            padding: "0.15rem 0.6rem",
            borderRadius: "999px",
            fontSize: "0.72rem",
            fontWeight: 700,
            ...(status?.leader_live
              ? { background: "rgba(16, 185, 129, 0.15)", color: "#6ee7b7", border: "1px solid rgba(16, 185, 129, 0.3)" }
              : { background: "rgba(248, 113, 113, 0.12)", color: "#fecaca", border: "1px solid rgba(248, 113, 113, 0.45)" }),
          }}
        >
          {status?.leader_live ? "● Leader connected" : "○ No leader socket"}
        </span>
        <button type="button" className="btn-secondary" style={{ marginTop: 0 }} disabled={busy} onClick={() => void refresh()}>
          Refresh
        </button>
        <button type="button" className="btn-primary" style={{ marginTop: 0 }} disabled={busy} onClick={() => void connect(false)}>
          {busy ? "Connecting…" : "Start new leader session"}
        </button>
        <button
          type="button"
          className="btn-secondary"
          style={{ marginTop: 0 }}
          disabled={busy || (!workspaceLive.length && !status?.session_id)}
          title="Opens `grok --leader --resume` for the live or latest session. Close any standalone Grok window for this workspace first."
          onClick={() => void connect(true)}
        >
          Connect / resume live
        </button>
      </div>
      {status?.detail && (
        <div style={{ color: "var(--text-muted)", fontSize: "0.78rem" }}>{status.detail}</div>
      )}
      {error && (
        <div style={{ padding: "0.55rem 0.7rem", borderRadius: "8px", background: "rgba(239, 68, 68, 0.14)", border: "1px solid rgba(248, 113, 113, 0.55)", color: "#fecaca", fontSize: "0.82rem" }}>
          {error}
        </div>
      )}
      {workspaceLive.length > 0 && (
        <div style={{ fontSize: "0.78rem", color: "var(--text-main)" }}>
          Live TUI{workspaceLive.length === 1 ? "" : "s"}:{" "}
          {workspaceLive.map((row) => `${row.session_id} (pid ${row.pid})`).join(", ")}
          {status?.leader_live ? "" : " — standalone; Hub cannot inject until you Connect / resume in leader mode."}
        </div>
      )}
    </div>
  );
}
