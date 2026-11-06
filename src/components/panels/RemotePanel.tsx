import { nocCardDenseStyle, nocPanelStyle } from "../common/telemetry/panel";
import { EmptyStateChip, HudStatusPill } from "../common/telemetry/status";
import { NOC_COLORS, NOC_SURFACES } from "../common/telemetry/tokens";

interface RemotePanelProps {
  remoteStatus: string;
  serverIP: string;
  startRemoteServer: () => Promise<void>;
  stopRemoteServer: () => Promise<void>;
  remoteLogs: string[];
}

export default function RemotePanel({
  remoteStatus,
  serverIP,
  startRemoteServer,
  stopRemoteServer,
  remoteLogs,
}: RemotePanelProps) {
  const isListening = remoteStatus.includes("listening");

  return (
    <div className="fade-in" style={{ ...nocPanelStyle, display: "flex", flexDirection: "column", gap: "1rem", marginTop: "1rem" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "1rem", flexWrap: "wrap" }}>
        <div style={{ display: "flex", alignItems: "center", gap: "0.75rem", flexWrap: "wrap" }}>
          <h2 style={{ margin: 0, fontSize: "1.1rem", fontWeight: 700, color: NOC_SURFACES.textPrimary }}>
            Remote Control Bridge
          </h2>
          <HudStatusPill
            label="TCP Server"
            value={isListening ? "LISTENING" : "STOPPED"}
            tone={isListening ? "ok" : "idle"}
          />
        </div>
        <button
          type="button"
          className={isListening ? "btn-secondary noc-focusable" : "btn-primary noc-focusable"}
          onClick={isListening ? stopRemoteServer : startRemoteServer}
          style={
            isListening
              ? { marginTop: 0, borderColor: "rgba(239, 68, 68, 0.5)", color: NOC_COLORS.crit }
              : { marginTop: 0, padding: "0.55rem 1.25rem" }
          }
        >
          {isListening ? "Stop Server" : "Start Server"}
        </button>
      </div>

      <div
        style={{
          ...nocCardDenseStyle,
          borderLeft: `3px solid ${isListening ? NOC_COLORS.ok : NOC_COLORS.idle}`,
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          gap: "1rem",
          flexWrap: "wrap",
        }}
      >
        <div style={{ flex: "1 1 320px" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", fontWeight: 600, fontSize: "0.92rem", color: NOC_SURFACES.textPrimary }}>
            <span
              style={{
                display: "inline-block",
                width: "8px",
                height: "8px",
                borderRadius: "50%",
                backgroundColor: isListening ? NOC_COLORS.ok : NOC_COLORS.idle,
                boxShadow: isListening ? `0 0 8px ${NOC_COLORS.ok}` : "none",
              }}
            />
            <span>Status:</span>
            <span style={{ color: isListening ? NOC_COLORS.ok : NOC_SURFACES.textMuted }}>{remoteStatus}</span>
          </div>
          <p style={{ margin: "0.35rem 0 0", fontSize: "0.8rem", color: NOC_SURFACES.textMuted, lineHeight: 1.45 }}>
            Control from Android app via WiFi. Pairing requires the LAN TCP auth token from Settings (
            <code>tool.tcp.auth_token</code> or <code>CA_TCP_AUTH_TOKEN</code>). Unauthenticated clients are rejected.
          </p>
        </div>

        {serverIP && (
          <div
            style={{
              background: "rgba(56, 189, 248, 0.1)",
              padding: "0.6rem 0.9rem",
              borderRadius: "8px",
              border: "1px solid rgba(56, 189, 248, 0.28)",
            }}
          >
            <div style={{ fontSize: "0.7rem", color: NOC_SURFACES.textMuted, textTransform: "uppercase", letterSpacing: "0.05em", fontWeight: 700 }}>
              Mobile Endpoint
            </div>
            <div
              className="tabular-nums"
              style={{ marginTop: "0.15rem", fontSize: "1rem", fontWeight: 700, color: NOC_COLORS.cyan, fontFamily: "var(--font-mono)" }}
            >
              {serverIP}
            </div>
          </div>
        )}
      </div>

      {remoteLogs.length > 0 ? (
        <div style={{ ...nocCardDenseStyle, padding: "0.85rem 1rem" }}>
          <div style={{ marginBottom: "0.6rem", fontSize: "0.72rem", fontWeight: 700, textTransform: "uppercase", letterSpacing: "0.05em", color: NOC_SURFACES.textMuted }}>
            Remote Connection Telemetry
          </div>
          <ul style={{ margin: 0, paddingLeft: "1.1rem", fontSize: "0.8rem", color: NOC_SURFACES.textSecondary, display: "flex", flexDirection: "column", gap: "0.35rem" }}>
            {remoteLogs.map((log, i) => (
              <li key={i} style={{ fontFamily: "var(--font-mono)" }}>
                {log}
              </li>
            ))}
          </ul>
        </div>
      ) : (
        <EmptyStateChip title="No remote client events recorded" detail="Start the TCP server to accept authenticated LAN connections." tone="idle" />
      )}
    </div>
  );
}
