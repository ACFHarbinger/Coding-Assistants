import type { WorkSession } from "./types";
import { nocCardDenseStyle, nocPanelStyle } from "../../common/telemetry/panel";
import { HudStatusPill } from "../../common/telemetry/status";
import { NOC_COLORS, NOC_SURFACES } from "../../common/telemetry/tokens";

export default function WorkSessionSection({
  workSessions,
  activeWorkSessionId,
  activeWorkSessionName,
  workSessionName,
  setWorkSessionName,
  creatingWorkSession,
  sessionError,
  setSessionError,
  createWorkSession,
  loadWorkSession,
  onSelectWorkSession,
}: {
  workSessions: WorkSession[];
  activeWorkSessionId: string | null;
  activeWorkSessionName: string | null;
  workSessionName: string;
  setWorkSessionName: (value: string) => void;
  creatingWorkSession: boolean;
  sessionError: string;
  setSessionError: (value: string) => void;
  createWorkSession: () => void;
  loadWorkSession: (sessionId: string) => void;
  onSelectWorkSession?: (sessionId: string | null) => void;
}) {
  return (
    <section style={{ ...nocPanelStyle, borderLeft: `3px solid ${NOC_COLORS.cyan}`, display: "flex", flexDirection: "column", gap: "0.85rem" }}>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: "1rem", flexWrap: "wrap" }}>
        <div>
          <strong style={{ color: NOC_SURFACES.textPrimary, fontSize: "1.02rem" }}>Team Work Session Chat</strong>
          <div style={{ color: NOC_SURFACES.textMuted, fontSize: "0.82rem", marginTop: "0.2rem" }}>
            Create a new team chat or load an existing work session. Focuses the Chat &amp; Memory window automatically.
          </div>
        </div>
        <HudStatusPill
          label="Session"
          value={activeWorkSessionName ? `Active: ${activeWorkSessionName}` : "No active session"}
          tone={activeWorkSessionName ? "ok" : "idle"}
        />
      </div>

      {sessionError && (
        <div style={{ padding: "0.65rem 0.85rem", borderRadius: "8px", background: "rgba(239, 68, 68, 0.12)", border: "1px solid rgba(239, 68, 68, 0.4)", color: "#fca5a5", fontSize: "0.85rem", display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <span>⚠️ {sessionError}</span>
          <button type="button" onClick={() => setSessionError("")} style={{ background: "none", border: "none", color: "#fca5a5", cursor: "pointer", fontSize: "0.9rem" }}>×</button>
        </div>
      )}

      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(280px, 1fr))", gap: "0.85rem", marginTop: "0.15rem" }}>
        <div style={{ ...nocCardDenseStyle, display: "flex", flexDirection: "column", gap: "0.5rem", padding: "0.85rem" }}>
          <span style={{ fontSize: "0.78rem", fontWeight: 700, textTransform: "uppercase", letterSpacing: "0.04em", color: NOC_COLORS.indigo }}>Create New Team Chat</span>
          <div style={{ display: "flex", gap: "0.5rem" }}>
            <input
              className="noc-focusable"
              value={workSessionName}
              onChange={(event) => setWorkSessionName(event.target.value)}
              onKeyDown={(event) => { if (event.key === "Enter") createWorkSession(); }}
              placeholder="Session name, e.g. Cloud sync design"
              style={{ flex: 1, padding: "0.5rem 0.75rem", borderRadius: "8px", background: "rgba(0,0,0,0.4)", color: "white", border: "1px solid var(--border-color)", outline: "none", fontSize: "0.85rem" }}
            />
            <button className="btn-primary noc-focusable" onClick={createWorkSession} disabled={!workSessionName.trim() || creatingWorkSession} style={{ marginTop: 0, padding: "0.5rem 1rem", fontSize: "0.82rem", whiteSpace: "nowrap" }}>
              {creatingWorkSession ? "Creating…" : "Create & Open"}
            </button>
          </div>
        </div>

        <div style={{ ...nocCardDenseStyle, display: "flex", flexDirection: "column", gap: "0.5rem", padding: "0.85rem" }}>
          <span style={{ fontSize: "0.78rem", fontWeight: 700, textTransform: "uppercase", letterSpacing: "0.04em", color: NOC_COLORS.cyan }}>Load Existing Team Chat</span>
          <div style={{ display: "flex", gap: "0.5rem" }}>
            <select
              value={activeWorkSessionId || ""}
              onChange={(event) => loadWorkSession(event.target.value)}
              style={{ flex: 1, padding: "0.5rem 0.75rem", borderRadius: "8px", background: "rgba(0,0,0,0.4)", color: "white", border: "1px solid var(--border-color)", outline: "none", fontSize: "0.85rem" }}
            >
              <option value="" disabled>-- Select a Team Chat --</option>
              {workSessions.map((session) => (
                <option key={session.id} value={session.id}>
                  {session.name} ({session.member_ids.length} members)
                </option>
              ))}
            </select>
            <button
              className="btn-secondary"
              onClick={() => { if (activeWorkSessionId) loadWorkSession(activeWorkSessionId); }}
              disabled={!activeWorkSessionId}
              style={{ padding: "0.5rem 1rem", fontSize: "0.85rem", whiteSpace: "nowrap" }}
            >
              Load & Open
            </button>
            {activeWorkSessionId && (
              <button
                className="btn-secondary"
                onClick={() => { if (onSelectWorkSession) onSelectWorkSession(null); }}
                style={{ padding: "0.5rem 0.75rem", fontSize: "0.85rem", whiteSpace: "nowrap", opacity: 0.85 }}
                title="Deselect active work session and return to general channels"
              >
                Clear Selection
              </button>
            )}
          </div>
        </div>
      </div>
    </section>
  );
}
