import { cardStyle, inputStyle } from "../hub/HubCharts";
import type { AgentRecord, WakeRecord } from "../hub/types";

export default function WakesPanel({
  agents,
  wakeTarget,
  setWakeTarget,
  wakeReason,
  setWakeReason,
  requestWake,
  refreshWakes,
  wakes,
}: {
  agents: AgentRecord[];
  wakeTarget: string;
  setWakeTarget: (next: string) => void;
  wakeReason: string;
  setWakeReason: (next: string) => void;
  requestWake: () => void;
  refreshWakes: () => void;
  wakes: WakeRecord[];
}) {
  return (
    <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1.5rem" }}>
      <div style={{ ...cardStyle, display: "grid", gap: "1rem" }}>
        <h3 style={{ margin: 0, fontSize: "1rem", fontWeight: 600, color: "var(--text-main)" }}>Request Wake</h3>
        <div style={{ display: "flex", gap: "0.75rem", flexWrap: "wrap" }}>
          <select value={wakeTarget} onChange={(e) => setWakeTarget(e.target.value)} style={inputStyle}>
            {agents.map((a) => <option key={a.id} value={a.id}>{a.display_name}</option>)}
          </select>
          <input
            style={{ ...inputStyle, flex: 1, minWidth: 200 }}
            placeholder="Reason for waking..."
            value={wakeReason}
            onChange={(e) => setWakeReason(e.target.value)}
          />
          <button className="btn-primary" onClick={requestWake}>Wake (human gate)</button>
          <button className="btn-secondary" onClick={refreshWakes}>Refresh List</button>
        </div>
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: "1rem", maxHeight: 400, overflowY: "auto", paddingRight: "0.5rem" }}>
        {wakes.length === 0 && (
          <div style={{ padding: "3rem", textAlign: "center", background: "rgba(0,0,0,0.2)", borderRadius: "12px", border: "1px dashed var(--border-color)" }}>
            <p style={{ color: "var(--text-muted)", fontSize: "0.95rem", margin: 0 }}>No wakes recorded.</p>
          </div>
        )}
        {wakes.map((w) => (
          <div key={w.id} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <div>
              <div style={{ marginBottom: "0.25rem", fontSize: "1.05rem", fontWeight: 600, color: "var(--primary)" }}>
                {w.target_agent}
              </div>
              <div style={{ fontSize: "0.9rem", color: "var(--text-main)" }}>
                {w.reason || <span style={{ color: "var(--text-muted)", fontStyle: "italic" }}>No reason provided</span>}
              </div>
              <div style={{ fontSize: "0.75rem", color: "var(--text-muted)", marginTop: "0.5rem" }}>
                {w.created_at}
              </div>
            </div>
            <div style={{ display: "flex", flexDirection: "column", alignItems: "flex-end", gap: "0.5rem" }}>
              <span style={{ fontSize: "0.75rem", padding: "0.2rem 0.6rem", background: "rgba(255,255,255,0.1)", borderRadius: "20px" }}>
                {w.status}
              </span>
              {w.requires_human_gate && (
                <span style={{ fontSize: "0.7rem", color: "#eab308", display: "flex", alignItems: "center", gap: "0.25rem" }}>
                  <span style={{ display: "inline-block", width: "6px", height: "6px", background: "#eab308", borderRadius: "50%" }} />
                  Human Gate Required
                </span>
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
