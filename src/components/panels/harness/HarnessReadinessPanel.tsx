import { useCallback, useEffect, useState } from "react";
import { invoke } from "../../../lib/tauri";
import HarnessBadge from "./HarnessBadge";
import GrokLeaderCard from "./GrokLeaderCard";
import EmbeddedTerminal from "./EmbeddedTerminal";
import { HARNESS_PREREQUISITES, HARNESS_STATE_LEGEND, type EmbeddedRelaunchOutcome, type HarnessSessionRegistration, type StartManagedHarnessOutcome } from "./types";

const PROVIDERS = ["grok", "chat", "claude", "gemini"] as const;

export default function HarnessReadinessPanel({ workspace }: { workspace: string }) {
  const [sessions, setSessions] = useState<HarnessSessionRegistration[]>([]);
  const [error, setError] = useState("");
  const [detail, setDetail] = useState("");
  const [diskId, setDiskId] = useState("");
  const [harness, setHarness] = useState<string>("grok");
  const [busy, setBusy] = useState(false);
  const [relaunching, setRelaunching] = useState<string | null>(null);
  const [terminals, setTerminals] = useState<Record<string, string>>({});

  const refresh = useCallback(async () => {
    try {
      const listed = await invoke<HarnessSessionRegistration[]>("hub_list_harness_sessions");
      setSessions(listed.filter((row) => !workspace || row.workspace === workspace));
      setError("");
    } catch (cause) {
      setError(String(cause));
    }
  }, [workspace]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const requireWorkspace = () => {
    if (!workspace.startsWith("/")) {
      throw new Error("Set an absolute Workspace Root before registering a harness.");
    }
  };

  const registerObserved = async () => {
    setBusy(true);
    try {
      requireWorkspace();
      await invoke("hub_register_harness_session", {
        harness,
        workspace,
        diskSessionId: diskId.trim() || null,
        leaderSocket: null,
      });
      setDiskId("");
      await refresh();
    } catch (cause) {
      setError(String(cause).replace(/^Error:\s*/, ""));
    } finally {
      setBusy(false);
    }
  };

  const relaunchInTerminal = async (target: string, existingPid: number | null) => {
    setBusy(true);
    setRelaunching(target);
    try {
      requireWorkspace();
      setDetail(`Starting ${target} terminal…`);
      const outcome = await invoke<EmbeddedRelaunchOutcome>("hub_relaunch_harness_embedded", {
        harness: target,
        workspace,
        existingPid,
      });
      setDetail(outcome.detail);
      setError("");
      setTerminals((prev) => ({ ...prev, [outcome.harness]: outcome.session_id }));
      await refresh();
    } catch (cause) {
      setError(String(cause).replace(/^Error:\s*/, ""));
    } finally {
      setBusy(false);
      setRelaunching(null);
    }
  };

  const closeTerminal = async (target: string) => {
    const sessionId = terminals[target];
    if (!sessionId) return;
    try {
      await invoke("pty_kill", { sessionId });
    } catch {
      // Already exited — nothing to clean up.
    }
    setTerminals((prev) => {
      const next = { ...prev };
      delete next[target];
      return next;
    });
  };

  const startManaged = async () => {
    setBusy(true);
    try {
      requireWorkspace();
      if (harness === "grok") {
        throw new Error("Use Connect / resume live below. Grok delivery needs a real leader session, not a fabricated thread id.");
      }
      if (harness !== "claude" && !diskId.trim()) {
        throw new Error(`Start managed needs a real ${harness} thread / conversation / disk session id. Do not invent a placeholder.`);
      }
      // Claude: Channel-connected terminal (no disk-session id). Others:
      // kill any prior managed pid, spawn, register — one atomic call.
      const outcome = await invoke<StartManagedHarnessOutcome>("hub_start_managed_harness", {
        harness,
        workspace,
        diskSessionId: diskId.trim() || "channel",
        prompt: "Coding-Assistants managed session",
      });
      setDetail(outcome.start.detail);
      setDiskId("");
      await refresh();
    } catch (cause) {
      setError(String(cause).replace(/^Error:\s*/, ""));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section style={{ marginBottom: "1.5rem", padding: "1.25rem", border: "1px solid rgba(251, 191, 36, 0.35)", borderRadius: "12px", background: "rgba(251, 191, 36, 0.06)" }}>
      <div style={{ display: "flex", justifyContent: "space-between", gap: "1rem", flexWrap: "wrap", alignItems: "baseline" }}>
        <div>
          <strong style={{ color: "var(--text-main)" }}>Harness interfaces</strong>
          <div style={{ color: "var(--text-muted)", fontSize: "0.82rem", marginTop: "0.25rem" }}>
            Observed = capture only. Managed = app-owned writer. Busy/queued are retryable. Resume in terminal kills an optional managed pid and opens a real interactive CLI — it does not attach to an undocumented socket or TTY.
          </div>
        </div>
        <button type="button" className="btn-secondary" style={{ marginTop: 0 }} onClick={() => void refresh()} disabled={busy}>
          Refresh
        </button>
      </div>

      <div style={{ display: "flex", flexWrap: "wrap", gap: "0.4rem", margin: "0.85rem 0" }}>
        {HARNESS_STATE_LEGEND.map((sample) => (
          <HarnessBadge key={`${sample.mode}:${sample.state}`} mode={sample.mode} state={sample.state} />
        ))}
      </div>

      {error && (
        <div style={{ marginBottom: "0.75rem", padding: "0.65rem 0.85rem", borderRadius: "8px", background: "rgba(239, 68, 68, 0.14)", border: "1px solid rgba(248, 113, 113, 0.55)", color: "#fecaca", fontSize: "0.85rem" }}>
          {error}
        </div>
      )}
      {detail && !error && (
        <div style={{ marginBottom: "0.75rem", padding: "0.65rem 0.85rem", borderRadius: "8px", background: "rgba(16, 185, 129, 0.12)", border: "1px solid rgba(16, 185, 129, 0.35)", color: "#a7f3d0", fontSize: "0.85rem" }}>
          {detail}
        </div>
      )}

      <div style={{ display: "flex", gap: "0.5rem", flexWrap: "wrap", marginBottom: "0.85rem" }}>
        <select value={harness} onChange={(event) => setHarness(event.target.value)} style={{ padding: "0.45rem 0.6rem", borderRadius: "8px", background: "rgba(0,0,0,0.35)", color: "white", border: "1px solid var(--border-color)" }}>
          {PROVIDERS.map((id) => (
            <option key={id} value={id}>{id}</option>
          ))}
        </select>
        <input
          value={diskId}
          onChange={(event) => setDiskId(event.target.value)}
          placeholder="thread / disk session id"
          style={{ flex: "1 1 180px", padding: "0.45rem 0.6rem", borderRadius: "8px", background: "rgba(0,0,0,0.35)", color: "white", border: "1px solid var(--border-color)" }}
        />
        <button type="button" className="btn-secondary" style={{ marginTop: 0 }} disabled={busy} onClick={() => void registerObserved()}>
          Register observed
        </button>
        <button type="button" className="btn-primary" style={{ marginTop: 0 }} disabled={busy} onClick={() => void startManaged()}>
          Start managed
        </button>
        <button
          type="button"
          className="btn-secondary"
          style={{ marginTop: 0 }}
          disabled={busy}
          title="Opens a real terminal for this harness, resuming the latest on-disk session when one exists."
          onClick={() => void relaunchInTerminal(harness, sessions.find((row) => row.harness === harness)?.managed_pid ?? null)}
        >
          {relaunching === harness ? "Opening terminal…" : "Resume in terminal"}
        </button>
        {terminals[harness] && (
          <button type="button" className="btn-secondary" style={{ marginTop: 0 }} onClick={() => void closeTerminal(harness)}>
            Close terminal
          </button>
        )}
      </div>
      {terminals[harness] && (
        <div style={{ height: "320px", marginBottom: "0.85rem" }}>
          <EmbeddedTerminal sessionId={terminals[harness]} onExit={(detail) => setDetail(`${harness} terminal: ${detail}`)} onError={(detail) => setError(detail)} />
        </div>
      )}
      <p style={{ color: "var(--text-muted)", fontSize: "0.8rem", margin: "0 0 0.85rem" }}>
        {HARNESS_PREREQUISITES[harness]} {harness === "grok"
          ? "Connect starts `grok agent leader` and a `grok --leader` TUI."
          : harness === "claude"
            ? "Start managed kills any prior registered Claude process, then opens a Channel-connected `claude` terminal (same as Channels → Connect). No thread id is required. The row is ready only once that Channel session is live."
            : "Start managed uses the documented wake spawn, then marks the Hub row owned only when you supply a real thread/conversation id."} It does not attach to an undocumented socket.
      </p>

      {harness === "grok" && (
        <div style={{ marginBottom: "0.85rem" }}>
          <GrokLeaderCard workspace={workspace} compact />
        </div>
      )}

      <div style={{ display: "grid", gap: "0.55rem" }}>
        {sessions.length === 0 && (
          <div style={{ color: "var(--text-muted)", fontSize: "0.85rem" }}>No harness sessions registered for this workspace.</div>
        )}
        {sessions.map((row) => (
          <div key={`${row.harness}:${row.workspace}`} style={{ display: "flex", flexDirection: "column", gap: "0.55rem", padding: "0.65rem 0.75rem", borderRadius: "9px", border: "1px solid var(--border-color)", background: "rgba(0,0,0,0.22)" }}>
            <div style={{ display: "flex", justifyContent: "space-between", gap: "0.75rem", flexWrap: "wrap" }}>
              <div>
                <strong style={{ color: "var(--text-main)" }}>{row.harness}</strong>
                <div style={{ color: "var(--text-muted)", fontSize: "0.78rem" }}>
                  thread {row.disk_session_id}
                  {row.writer_owner ? ` · writer ${row.writer_owner}` : ""}
                  {row.managed_pid ? ` · pid ${row.managed_pid}` : ""}
                </div>
              </div>
              <div style={{ display: "flex", gap: "0.45rem", alignItems: "center", flexWrap: "wrap" }}>
                <HarnessBadge mode={row.mode} state={row.state} />
                <button
                  type="button"
                  className="btn-secondary"
                  style={{ marginTop: 0 }}
                  disabled={busy}
                  title="Kill the managed pid if one is registered, then resume this harness in a real terminal."
                  onClick={() => void relaunchInTerminal(row.harness, row.managed_pid)}
                >
                  {relaunching === row.harness ? "Opening…" : "Resume in terminal"}
                </button>
                {terminals[row.harness] && (
                  <button type="button" className="btn-secondary" style={{ marginTop: 0 }} onClick={() => void closeTerminal(row.harness)}>
                    Close terminal
                  </button>
                )}
              </div>
            </div>
            {terminals[row.harness] && (
              <div style={{ height: "320px" }}>
                <EmbeddedTerminal sessionId={terminals[row.harness]} onExit={(detail) => setDetail(`${row.harness} terminal: ${detail}`)} onError={(detail) => setError(detail)} />
              </div>
            )}
          </div>
        ))}
      </div>
    </section>
  );
}
