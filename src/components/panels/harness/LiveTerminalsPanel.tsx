import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "../../../lib/tauri";
import type { ActiveGrokSession, GrokConnectResult } from "../hub/types";
import LiveTerminalCard from "./LiveTerminalCard";
import EmbeddedTerminal from "./EmbeddedTerminal";
import ResizableTerminalFrame from "./ResizableTerminalFrame";
import TerminalPaneErrorBoundary from "./TerminalPaneErrorBoundary";
import {
  LIVE_TERMINAL_HARNESSES,
  presenceLive,
  sessionAliases,
  type EmbeddedRelaunchOutcome,
  type HarnessSessionRegistration,
  type LiveTerminalHarness,
  type StartManagedHarnessOutcome,
  type StopManagedOutcome,
  type WorkspaceAgentPresence,
} from "./types";

function findSession(sessions: HarnessSessionRegistration[], harness: string, workspace: string) {
  const aliases = sessionAliases(harness);
  return sessions.find((row) => aliases.includes(row.harness) && (!workspace || row.workspace === workspace)) ?? null;
}

export default function LiveTerminalsPanel({ workspace }: { workspace: string }) {
  const [sessions, setSessions] = useState<HarnessSessionRegistration[]>([]);
  const [presence, setPresence] = useState<WorkspaceAgentPresence | null>(null);
  const [grokLive, setGrokLive] = useState<ActiveGrokSession[]>([]);
  const [error, setError] = useState("");
  const [detail, setDetail] = useState("");
  const [diskId, setDiskId] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [terminals, setTerminals] = useState<Record<string, string>>({});

  const refresh = useCallback(async () => {
    if (!workspace.startsWith("/")) {
      setPresence(null);
      setSessions([]);
      setGrokLive([]);
      return;
    }
    try {
      const [listed, nextPresence, grok] = await Promise.all([
        invoke<HarnessSessionRegistration[]>("hub_list_harness_sessions"),
        invoke<WorkspaceAgentPresence>("hub_workspace_agent_presence", { workspace }),
        invoke<ActiveGrokSession[]>("hub_grok_list_live_sessions"),
      ]);
      setSessions(listed.filter((row) => !workspace || row.workspace === workspace));
      setPresence(nextPresence);
      setGrokLive(grok.filter((row) => !workspace || row.cwd === workspace));
      setError("");
    } catch (cause) {
      setError(String(cause).replace(/^Error:\s*/, ""));
    }
  }, [workspace]);

  useEffect(() => {
    void refresh();
    const interval = setInterval(() => { void refresh(); }, 4000);
    return () => clearInterval(interval);
  }, [refresh]);

  const requireWorkspace = () => {
    if (!workspace.startsWith("/")) {
      throw new Error("Set an absolute Workspace Root before managing live terminals.");
    }
  };

  const run = async (harness: string, work: () => Promise<string>) => {
    setBusy(harness);
    try {
      requireWorkspace();
      setDetail(await work());
      setError("");
      await refresh();
    } catch (cause) {
      setError(String(cause).replace(/^Error:\s*/, ""));
    } finally {
      setBusy(null);
    }
  };

  const resume = (harness: string) => void run(harness, async () => {
    setDetail(`Starting ${harness} terminal…`);
    const row = findSession(sessions, harness, workspace);
    const outcome = await invoke<EmbeddedRelaunchOutcome>("hub_relaunch_harness_embedded", {
      harness,
      workspace,
      existingPid: row?.managed_pid ?? null,
    });
    const sid = outcome.sessionId ?? (outcome as { session_id?: string }).session_id;
    if (!sid) {
      throw new Error("Resume did not return an in-app terminal session id.");
    }
    setTerminals((prev) => ({ ...prev, [harness]: sid }));
    return outcome.detail;
  });

  const closeTerminal = async (harness: string) => {
    const sessionId = terminals[harness];
    if (!sessionId) return;
    try {
      await invoke("pty_kill", { sessionId });
    } catch {
      // Already exited — nothing to clean up.
    }
    setTerminals((prev) => {
      const next = { ...prev };
      delete next[harness];
      return next;
    });
  };

  const startManaged = (harness: LiveTerminalHarness) => void run(harness, async () => {
    if (harness === "grok") {
      throw new Error("Grok uses Connect / resume, not a fabricated managed worker.");
    }
    const row = findSession(sessions, harness, workspace);
    const id = diskId.trim() || row?.disk_session_id || "";
    if (harness !== "claude" && !id) {
      throw new Error(`Start managed needs a real ${harness} thread / conversation / disk session id.`);
    }
    const outcome = await invoke<StartManagedHarnessOutcome>("hub_start_managed_harness", {
      harness,
      workspace,
      diskSessionId: id || "channel",
      prompt: "Coding-Assistants managed session",
    });
    setDiskId("");
    return outcome.start.detail;
  });

  const connectGrok = () => void run("grok", async () => {
    const result = await invoke<GrokConnectResult>("hub_grok_connect", { workspace, resume: true });
    return result.detail;
  });

  const kill = (harness: string) => void run(harness, async () => {
    const outcome = await invoke<StopManagedOutcome>("hub_stop_managed_harness", { harness, workspace });
    return outcome.detail;
  });

  const liveIds = useMemo(
    () => LIVE_TERMINAL_HARNESSES.filter((id) => presenceLive(id, presence)),
    [presence],
  );
  const idleIds = useMemo(
    () => LIVE_TERMINAL_HARNESSES.filter((id) => !presenceLive(id, presence)),
    [presence],
  );

  const grokPid = grokLive[0]?.pid ?? null;
  const signalFor = (id: LiveTerminalHarness, row: HarnessSessionRegistration | null): { text: string; pid: number | null } => {
    if (id === "claude") {
      return { text: "Channel session live (terminal-emulator pid is not liveness)", pid: null };
    }
    if (id === "grok") {
      if (row?.managed_pid && presenceLive("grok", presence)) {
        return { text: "Managed worker live", pid: row.managed_pid };
      }
      return { text: grokPid != null ? "Leader TUI live" : "Live via leader TUI + socket", pid: grokPid };
    }
    return { text: "Managed worker live", pid: row?.managed_pid ?? null };
  };

  const startRow = (id: LiveTerminalHarness) => (
    <div key={id} style={{ display: "flex", flexDirection: "column", gap: "0.6rem", padding: "0.55rem 0.7rem", borderRadius: "8px", background: "rgba(0,0,0,0.25)" }}>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: "0.65rem", flexWrap: "wrap" }}>
        <strong style={{ color: "var(--text-main)", textTransform: "capitalize", minWidth: "4.5rem" }}>{id}</strong>
        <div style={{ display: "flex", gap: "0.4rem", flexWrap: "wrap" }}>
          {id === "grok" ? (
            <button type="button" className="btn-primary" style={{ marginTop: 0 }} disabled={busy !== null} onClick={connectGrok}>
              Connect / resume
            </button>
          ) : (
            <button type="button" className="btn-primary" style={{ marginTop: 0 }} disabled={busy !== null} onClick={() => startManaged(id)}>
              Start managed
            </button>
          )}
          <button type="button" className="btn-secondary" style={{ marginTop: 0 }} disabled={busy !== null} onClick={() => resume(id)}>
            Resume in terminal
          </button>
          {terminals[id] && (
            <button type="button" className="btn-secondary" style={{ marginTop: 0 }} onClick={() => void closeTerminal(id)}>
              Close terminal
            </button>
          )}
        </div>
      </div>
      {terminals[id] && (
        <ResizableTerminalFrame persistId={`idle-${id}`}>
          <TerminalPaneErrorBoundary>
            <EmbeddedTerminal sessionId={terminals[id]} onExit={(detail) => setDetail(`${id} terminal: ${detail}`)} onError={(detail) => setError(detail)} />
          </TerminalPaneErrorBoundary>
        </ResizableTerminalFrame>
      )}
    </div>
  );

  return (
    <section style={{ marginBottom: "1.5rem", padding: "1.25rem", border: "1px solid rgba(99, 102, 241, 0.38)", borderRadius: "12px", background: "rgba(99, 102, 241, 0.06)" }}>
      <div style={{ display: "flex", justifyContent: "space-between", gap: "1rem", flexWrap: "wrap", alignItems: "baseline" }}>
        <div>
          <strong style={{ color: "var(--text-main)" }}>Live terminals</strong>
          <div style={{ color: "var(--text-muted)", fontSize: "0.82rem", marginTop: "0.25rem" }}>
            What is honestly live for this workspace right now. Claude: Channel session. Chat / Gemini: managed pid still running. Grok: managed pid, or a live <code>--leader</code> TUI with the socket up. A Hub row alone is not live.
          </div>
        </div>
        <button type="button" className="btn-secondary" style={{ marginTop: 0 }} onClick={() => void refresh()} disabled={busy !== null}>
          Refresh
        </button>
      </div>

      {error && (
        <div style={{ marginTop: "0.85rem", padding: "0.65rem 0.85rem", borderRadius: "8px", background: "rgba(239, 68, 68, 0.14)", border: "1px solid rgba(248, 113, 113, 0.55)", color: "#fecaca", fontSize: "0.85rem" }}>
          {error}
        </div>
      )}
      {detail && !error && (
        <div style={{ marginTop: "0.85rem", padding: "0.65rem 0.85rem", borderRadius: "8px", background: "rgba(16, 185, 129, 0.12)", border: "1px solid rgba(16, 185, 129, 0.35)", color: "#a7f3d0", fontSize: "0.85rem" }}>
          {detail}
        </div>
      )}

      {liveIds.length === 0 ? (
        <div style={{ marginTop: "0.95rem", padding: "1.5rem 1.15rem", minHeight: "220px", border: "2px dashed var(--border-color)", borderRadius: "12px", background: "rgba(255, 255, 255, 0.02)", display: "flex", flexDirection: "column", gap: "0.85rem", justifyContent: "center" }}>
          <div>
            <div style={{ fontWeight: 700, color: "var(--text-primary)", fontSize: "1.02rem" }}>No live terminals</div>
            <div style={{ color: "var(--text-muted)", fontSize: "0.84rem", marginTop: "0.35rem", maxWidth: "42rem" }}>
              {workspace.startsWith("/")
                ? "Nothing is actually running for this workspace. Observed or stale registrations stay in Harness interfaces below and are not shown here as live."
                : "Set an absolute Workspace Root above to see and start live terminals for this repo."}
            </div>
          </div>
          {workspace.startsWith("/") && (
            <>
              <input
                value={diskId}
                onChange={(event) => setDiskId(event.target.value)}
                placeholder="thread / disk session id (Chat & Gemini start managed)"
                style={{ padding: "0.5rem 0.7rem", borderRadius: "8px", background: "rgba(0,0,0,0.35)", color: "white", border: "1px solid var(--border-color)" }}
              />
              <div style={{ display: "grid", gap: "0.45rem" }}>{idleIds.map(startRow)}</div>
            </>
          )}
        </div>
      ) : (
        <>
          <div style={{ display: "grid", gridTemplateColumns: "1fr", gap: "0.85rem", marginTop: "0.95rem" }}>
            {liveIds.map((id) => {
              const row = findSession(sessions, id, workspace);
              const { text, pid } = signalFor(id, row);
              return (
                <LiveTerminalCard
                  key={id}
                  harness={id}
                  session={row}
                  signal={text}
                  pid={pid}
                  busy={busy === id}
                  onResume={() => resume(id)}
                  onKill={() => kill(id)}
                  terminalSessionId={terminals[id] ?? null}
                  onCloseTerminal={() => void closeTerminal(id)}
                  onTerminalExit={(detail) => setDetail(`${id} terminal: ${detail}`)}
                  onTerminalError={(detail) => setError(detail)}
                />
              );
            })}
          </div>
          {idleIds.length > 0 && (
            <div style={{ marginTop: "1rem" }}>
              <div style={{ color: "var(--text-muted)", fontSize: "0.8rem", marginBottom: "0.45rem" }}>Not live — start or resume</div>
              <input
                value={diskId}
                onChange={(event) => setDiskId(event.target.value)}
                placeholder="thread / disk session id (Chat & Gemini start managed)"
                style={{ width: "100%", marginBottom: "0.45rem", padding: "0.45rem 0.6rem", borderRadius: "8px", background: "rgba(0,0,0,0.35)", color: "white", border: "1px solid var(--border-color)" }}
              />
              <div style={{ display: "grid", gap: "0.4rem" }}>{idleIds.map(startRow)}</div>
            </div>
          )}
        </>
      )}
    </section>
  );
}
