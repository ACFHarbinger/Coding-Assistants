import { useState } from "react";
import type { HubAgent } from "../../../app/hubState";
import { testVtuberBridge, type VtuberBridgeStatus } from "../../settings/api";

export interface VTuberPresenceDockProps {
  hubAgents: HubAgent[];
  lastNotice?: string | null;
  onDismissNotice?: () => void;
  baseUrl?: string;
}

export default function VTuberPresenceDock({
  hubAgents,
  lastNotice,
  onDismissNotice,
  baseUrl = "http://127.0.0.1:12393",
}: VTuberPresenceDockProps) {
  const animatedAgents = hubAgents.filter((a) => a.animated_avatar);

  const [expanded, setExpanded] = useState<boolean>(false);
  const [bridgeStatus, setBridgeStatus] = useState<VtuberBridgeStatus | null>(null);
  const [checking, setChecking] = useState<boolean>(false);

  // If no agents have animated_avatar enabled, render nothing (zero resource use / zero network)
  if (animatedAgents.length === 0 && !lastNotice) {
    return null;
  }

  const handleCheckConnection = async () => {
    setChecking(true);
    try {
      const status = await testVtuberBridge(baseUrl);
      setBridgeStatus(status);
    } catch (e) {
      setBridgeStatus({
        available: false,
        endpoint: baseUrl,
        message: String(e),
      });
    } finally {
      setChecking(false);
    }
  };

  return (
    <div
      data-testid="vtuber-presence-dock"
      style={{
        margin: "0.4rem 0.85rem",
        padding: "0.5rem 0.75rem",
        borderRadius: "8px",
        background: "rgba(15, 23, 42, 0.65)",
        border: "1px solid rgba(56, 189, 248, 0.25)",
        backdropFilter: "blur(8px)",
        display: "flex",
        flexDirection: "column",
        gap: "0.4rem",
        fontSize: "0.8rem",
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          flexWrap: "wrap",
          gap: "0.5rem",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", flexWrap: "wrap" }}>
          <span
            style={{
              fontWeight: 600,
              color: "#38bdf8",
              display: "inline-flex",
              alignItems: "center",
              gap: "0.35rem",
            }}
          >
            <span>🎭</span> V-Tuber Presence:
          </span>

          {animatedAgents.map((agent) => (
            <span
              key={agent.id}
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: "0.72rem",
                color: "var(--text-main)",
                background: "rgba(56, 189, 248, 0.15)",
                border: "1px solid rgba(56, 189, 248, 0.35)",
                borderRadius: "4px",
                padding: "0.1rem 0.4rem",
              }}
            >
              @{agent.id}
              {agent.vtuber_character ? ` (${agent.vtuber_character})` : ""}
            </span>
          ))}

          {bridgeStatus && (
            <span
              style={{
                fontSize: "0.72rem",
                color: bridgeStatus.available ? "#4ade80" : "#f87171",
                display: "inline-flex",
                alignItems: "center",
                gap: "0.25rem",
              }}
            >
              <span>{bridgeStatus.available ? "● Online" : "○ Offline"}</span>
            </span>
          )}
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: "0.4rem" }}>
          <button
            type="button"
            className="btn-secondary"
            style={{
              marginTop: 0,
              padding: "0.15rem 0.5rem",
              fontSize: "0.72rem",
              borderRadius: "4px",
            }}
            onClick={() => void handleCheckConnection()}
            disabled={checking}
            title="Probe local Open-LLM-VTuber connection"
          >
            {checking ? "Checking…" : "Check Bridge"}
          </button>

          <button
            type="button"
            className="btn-secondary"
            style={{
              marginTop: 0,
              padding: "0.15rem 0.5rem",
              fontSize: "0.72rem",
              borderRadius: "4px",
            }}
            onClick={() => setExpanded(!expanded)}
          >
            {expanded ? "Hide Live2D View" : "Show Live2D View"}
          </button>
        </div>
      </div>

      {lastNotice && (
        <div
          data-testid="vtuber-notice"
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            background: "rgba(239, 68, 68, 0.15)",
            border: "1px solid rgba(248, 113, 113, 0.35)",
            color: "#fca5a5",
            borderRadius: "4px",
            padding: "0.25rem 0.5rem",
            fontSize: "0.75rem",
          }}
        >
          <span>{lastNotice}</span>
          {onDismissNotice && (
            <button
              type="button"
              aria-label="Dismiss warning"
              onClick={onDismissNotice}
              style={{
                background: "transparent",
                border: "none",
                color: "#fca5a5",
                cursor: "pointer",
                padding: "0 0.3rem",
                fontSize: "0.8rem",
              }}
              title="Dismiss warning"
            >
              ✕
            </button>
          )}
        </div>
      )}

      {expanded && (
        <div
          style={{
            marginTop: "0.3rem",
            borderRadius: "6px",
            overflow: "hidden",
            border: "1px solid var(--border-color)",
            background: "rgba(0, 0, 0, 0.3)",
            height: "260px",
            position: "relative",
          }}
        >
          <iframe
            src={baseUrl}
            title="Open-LLM-VTuber Live2D Viewport"
            style={{
              width: "100%",
              height: "100%",
              border: "none",
            }}
            sandbox="allow-scripts allow-same-origin"
          />
        </div>
      )}
    </div>
  );
}
