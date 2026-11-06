import { useState } from "react";
import type { HubAgent } from "../../../../app/hubState";
import { setAgentAnimatedAvatar, testVtuberBridge, type VtuberBridgeStatus } from "../../api";
import { inputStyle } from "../shared";

export interface VTuberProfileControlProps {
  agent: HubAgent;
  onChanged?: () => void;
}

export default function VTuberProfileControl({ agent, onChanged }: VTuberProfileControlProps) {
  const [enabled, setEnabled] = useState<boolean>(agent.animated_avatar ?? false);
  const [character, setCharacter] = useState<string>(agent.vtuber_character ?? "");
  const [saving, setSaving] = useState<boolean>(false);
  const [testing, setTesting] = useState<boolean>(false);
  const [bridgeStatus, setBridgeStatus] = useState<VtuberBridgeStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<boolean>(false);

  const handleToggle = async (newVal: boolean) => {
    setEnabled(newVal);
    setError(null);
    setSaving(true);
    try {
      await setAgentAnimatedAvatar(agent.id, newVal, character ? character.trim() : null);
      onChanged?.();
    } catch (e) {
      setError(String(e));
      setEnabled(agent.animated_avatar ?? false);
    } finally {
      setSaving(false);
    }
  };

  const handleSaveCharacter = async () => {
    setError(null);
    setSaving(true);
    try {
      await setAgentAnimatedAvatar(agent.id, enabled, character.trim() || null);
      onChanged?.();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const handleTestBridge = async () => {
    setTesting(true);
    setError(null);
    try {
      const status = await testVtuberBridge();
      setBridgeStatus(status);
    } catch (e) {
      setError(String(e));
    } finally {
      setTesting(false);
    }
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "0.35rem", marginTop: "0.35rem" }}>
      <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", flexWrap: "wrap" }}>
        <button
          type="button"
          className={enabled ? "btn-primary" : "btn-secondary"}
          style={{
            marginTop: 0,
            padding: "0.2rem 0.6rem",
            fontSize: "0.72rem",
            borderRadius: "6px",
            display: "inline-flex",
            alignItems: "center",
            gap: "0.35rem",
          }}
          disabled={saving}
          onClick={() => void handleToggle(!enabled)}
          title={enabled ? "Switch to standard static avatar" : "Opt into animated Open-LLM-VTuber avatar"}
        >
          <span>{enabled ? "🎭 Animated Live2D" : "🖼️ Static Avatar"}</span>
        </button>

        {enabled && (
          <button
            type="button"
            className="btn-secondary"
            style={{
              marginTop: 0,
              padding: "0.2rem 0.5rem",
              fontSize: "0.72rem",
              borderRadius: "6px",
            }}
            onClick={() => setExpanded(!expanded)}
          >
            {expanded ? "Hide Config" : "Config…"}
          </button>
        )}

        {enabled && (
          <span
            style={{
              fontSize: "0.72rem",
              color: "var(--text-muted)",
              display: "inline-flex",
              alignItems: "center",
              gap: "0.25rem",
            }}
          >
            {agent.vtuber_character ? `Model: ${agent.vtuber_character}` : "Default model"}
          </span>
        )}
      </div>

      {enabled && expanded && (
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            gap: "0.4rem",
            padding: "0.5rem 0.65rem",
            borderRadius: "6px",
            background: "rgba(0, 0, 0, 0.2)",
            border: "1px solid var(--border-color)",
            marginTop: "0.2rem",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", flexWrap: "wrap" }}>
            <label style={{ fontSize: "0.75rem", color: "var(--text-muted)" }}>
              Character / Model:
            </label>
            <input
              type="text"
              placeholder="e.g. shizuku, hiyori"
              value={character}
              onChange={(e) => setCharacter(e.target.value)}
              style={{
                ...inputStyle,
                padding: "0.2rem 0.45rem",
                fontSize: "0.78rem",
                width: "140px",
              }}
              disabled={saving}
            />
            <button
              type="button"
              className="btn-primary"
              style={{ marginTop: 0, padding: "0.2rem 0.55rem", fontSize: "0.72rem" }}
              onClick={() => void handleSaveCharacter()}
              disabled={saving}
            >
              Save
            </button>
            <button
              type="button"
              className="btn-secondary"
              style={{ marginTop: 0, padding: "0.2rem 0.55rem", fontSize: "0.72rem" }}
              onClick={() => void handleTestBridge()}
              disabled={testing}
            >
              {testing ? "Probing…" : "Test Bridge"}
            </button>
          </div>

          {bridgeStatus && (
            <div
              style={{
                fontSize: "0.73rem",
                color: bridgeStatus.available ? "#4ade80" : "#f87171",
                display: "inline-flex",
                alignItems: "center",
                gap: "0.35rem",
              }}
            >
              <span>{bridgeStatus.available ? "● Available:" : "○ Offline:"}</span>
              <span>{bridgeStatus.message}</span>
            </div>
          )}
        </div>
      )}

      {error && (
        <span style={{ color: "#fca5a5", fontSize: "0.72rem" }}>
          {error}
        </span>
      )}
    </div>
  );
}
