import { useCallback, useEffect, useState } from "react";
import type { CreativeToolStatus, CreativeToolsStatus } from "../types";
import {
  getCreativeToolsCodexSnippet,
  getCreativeToolsStatus,
  reapplyCreativeTools,
  setCreativeToolEnabled,
} from "../api";
import { ToggleRow, shortenPath } from "./shared";

export interface CreativeToolsTabProps {
  workspaceRoot: string | null;
  busy: boolean;
}

export default function CreativeToolsTab({ workspaceRoot, busy }: CreativeToolsTabProps) {
  const [status, setStatus] = useState<CreativeToolsStatus | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copiedCodex, setCopiedCodex] = useState(false);
  const [writtenNotice, setWrittenNotice] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!workspaceRoot) {
      setStatus(null);
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const res = await getCreativeToolsStatus(workspaceRoot);
      setStatus(res);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, [workspaceRoot]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const handleToggleTool = async (tool: CreativeToolStatus) => {
    if (!workspaceRoot) return;
    setLoading(true);
    setError(null);
    try {
      const res = await setCreativeToolEnabled(workspaceRoot, tool.key, !tool.enabled);
      setStatus(res);
      if (res.writtenConfigs.length > 0) {
        setWrittenNotice(`Updated MCP configs: ${res.writtenConfigs.map((p) => shortenPath(p, 30)).join(", ")}`);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleReapply = async () => {
    if (!workspaceRoot) return;
    setLoading(true);
    setError(null);
    try {
      const res = await reapplyCreativeTools(workspaceRoot);
      setStatus(res);
      if (res.writtenConfigs.length > 0) {
        setWrittenNotice(`Re-applied MCP configs: ${res.writtenConfigs.map((p) => shortenPath(p, 30)).join(", ")}`);
      } else {
        setWrittenNotice("No MCP configs were modified.");
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleCopyCodex = async () => {
    if (!workspaceRoot) return;
    setLoading(true);
    setError(null);
    try {
      const snippet = await getCreativeToolsCodexSnippet(workspaceRoot);
      if (!snippet.trim()) {
        setError("No creative tools are currently enabled for this workspace.");
        return;
      }
      await navigator.clipboard.writeText(snippet);
      setCopiedCodex(true);
      setTimeout(() => setCopiedCodex(false), 2000);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  if (!workspaceRoot) {
    return (
      <div style={{ padding: "1.5rem", textAlign: "center", color: "var(--text-muted)", fontSize: "0.9rem" }}>
        Select an active workspace in Orchestrate to configure workspace-scoped Creative Tools MCP bridges.
      </div>
    );
  }

  return (
    <div style={{ display: "grid", gap: "1.25rem" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: "1rem", flexWrap: "wrap" }}>
        <div>
          <h3 style={{ margin: 0, fontSize: "1rem", fontWeight: 700 }}>Creative Tools MCP Bridges</h3>
          <p style={{ color: "var(--text-muted)", fontSize: "0.82rem", margin: "0.25rem 0 0" }}>
            Expose local creative software (Blender, Krita, Godot, Aseprite, Unreal, Unity, OpenToonz) to coding agents in{" "}
            <strong>{shortenPath(workspaceRoot, 32)}</strong>.
          </p>
        </div>
        <div style={{ display: "flex", gap: "0.5rem", flexWrap: "wrap" }}>
          <button
            type="button"
            className="btn-secondary"
            style={{ marginTop: 0, padding: "0.35rem 0.75rem", fontSize: "0.78rem" }}
            disabled={busy || loading}
            onClick={() => void handleReapply()}
          >
            Re-apply to Configs
          </button>
          <button
            type="button"
            className="btn-primary"
            style={{ marginTop: 0, padding: "0.35rem 0.75rem", fontSize: "0.78rem" }}
            disabled={busy || loading}
            onClick={() => void handleCopyCodex()}
          >
            {copiedCodex ? "Copied Codex TOML!" : "Copy Codex Snippet"}
          </button>
        </div>
      </div>

      {error && (
        <div
          style={{
            padding: "0.6rem 0.85rem",
            borderRadius: "8px",
            background: "rgba(239, 68, 68, 0.12)",
            border: "1px solid rgba(248, 113, 113, 0.45)",
            color: "#fca5a5",
            fontSize: "0.82rem",
          }}
        >
          {error}
        </div>
      )}

      {writtenNotice && (
        <div
          style={{
            padding: "0.6rem 0.85rem",
            borderRadius: "8px",
            background: "rgba(16, 185, 129, 0.12)",
            border: "1px solid rgba(16, 185, 129, 0.35)",
            color: "#6ee7b7",
            fontSize: "0.82rem",
          }}
        >
          {writtenNotice}
        </div>
      )}

      <div style={{ display: "grid", gap: "0.85rem" }}>
        {status?.tools.map((tool) => (
          <div
            key={tool.key}
            style={{
              padding: "0.9rem 1.1rem",
              borderRadius: "10px",
              border: `1px solid ${tool.enabled ? "rgba(99, 102, 241, 0.4)" : "var(--border-color)"}`,
              background: tool.enabled ? "rgba(99, 102, 241, 0.06)" : "rgba(0,0,0,0.22)",
              display: "grid",
              gap: "0.55rem",
            }}
          >
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: "0.5rem" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "0.6rem", flexWrap: "wrap" }}>
                <strong style={{ fontSize: "0.95rem", color: "var(--text-main)" }}>{tool.displayName}</strong>
                <span
                  style={{
                    fontSize: "0.7rem",
                    fontWeight: 600,
                    padding: "0.12rem 0.5rem",
                    borderRadius: "999px",
                    background: "rgba(255,255,255,0.06)",
                    border: "1px solid var(--border-color)",
                    color: "var(--text-muted)",
                    textTransform: "capitalize",
                  }}
                >
                  {tool.transport} {tool.port ? `:${tool.port}` : ""}
                </span>
                <span
                  style={{
                    fontSize: "0.7rem",
                    fontWeight: 600,
                    padding: "0.12rem 0.5rem",
                    borderRadius: "999px",
                    background: tool.binaryFound ? "rgba(16, 185, 129, 0.12)" : "rgba(245, 158, 11, 0.12)",
                    border: `1px solid ${tool.binaryFound ? "rgba(16, 185, 129, 0.35)" : "rgba(245, 158, 11, 0.35)"}`,
                    color: tool.binaryFound ? "#6ee7b7" : "#fde68a",
                  }}
                >
                  {tool.binaryFound ? "Bridge Installed" : "Bridge Binary Missing"}
                </span>
                <span
                  style={{
                    fontSize: "0.7rem",
                    color: tool.appRunning ? "#6ee7b7" : "var(--text-muted)",
                    display: "inline-flex",
                    alignItems: "center",
                    gap: "0.3rem",
                  }}
                >
                  <span style={{ fontSize: "0.6rem" }}>●</span> {tool.appRunning ? "App Running" : "App Idle"}
                </span>
              </div>

              <ToggleRow
                label=""
                checked={tool.enabled}
                disabled={busy || loading}
                onToggle={() => void handleToggleTool(tool)}
              />
            </div>

            <div style={{ fontSize: "0.78rem", color: "var(--text-muted)", display: "grid", gap: "0.25rem" }}>
              {tool.binaryPath && <div>Binary: <code>{tool.binaryPath}</code></div>}
              {!tool.binaryFound && (
                <div style={{ color: "#fde68a" }}>
                  Bridge executable <code>{tool.key}</code> not found in app directory or on $PATH.
                </div>
              )}
              {tool.gatedFlag && (
                <div>
                  Script execution requires passing <code>{tool.gatedFlag}</code> (disabled by default for safety).
                </div>
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
