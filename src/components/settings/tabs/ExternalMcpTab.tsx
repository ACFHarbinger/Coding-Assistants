import { useCallback, useEffect, useState } from "react";
import type { ExternalMcpStatus, ExternalServerStatus } from "../types";
import {
  getExternalMcpStatus,
  reapplyExternalMcp,
  setExternalMcpEnabled,
} from "../api";
import { ToggleRow, shortenPath } from "./shared";

export interface ExternalMcpTabProps {
  workspaceRoot: string | null;
  busy: boolean;
}

export default function ExternalMcpTab({ workspaceRoot, busy }: ExternalMcpTabProps) {
  const [status, setStatus] = useState<ExternalMcpStatus | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [writtenNotice, setWrittenNotice] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!workspaceRoot) {
      setStatus(null);
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const res = await getExternalMcpStatus(workspaceRoot);
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

  const handleToggleServer = async (server: ExternalServerStatus) => {
    if (!workspaceRoot) return;
    setLoading(true);
    setError(null);
    try {
      const res = await setExternalMcpEnabled(workspaceRoot, server.key, !server.enabled);
      setStatus(res);
      if (res.writtenConfigs.length > 0) {
        setWrittenNotice(
          `Updated MCP configs: ${res.writtenConfigs.map((p) => shortenPath(p, 30)).join(", ")}`,
        );
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
      const res = await reapplyExternalMcp(workspaceRoot);
      setStatus(res);
      if (res.writtenConfigs.length > 0) {
        setWrittenNotice(
          `Re-applied MCP configs: ${res.writtenConfigs.map((p) => shortenPath(p, 30)).join(", ")}`,
        );
      } else {
        setWrittenNotice("No MCP configs were modified.");
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  if (!workspaceRoot) {
    return (
      <div
        style={{
          padding: "1.5rem",
          textAlign: "center",
          color: "var(--text-muted)",
          fontSize: "0.9rem",
        }}
      >
        Select an active workspace in Orchestrate to configure workspace-scoped External MCP
        servers.
      </div>
    );
  }

  const renderAuthChip = (server: ExternalServerStatus) => {
    const authKind = "kind" in server.auth ? server.auth.kind : "";
    if (authKind === "api_key") {
      const configured = server.authConfigured === true;
      return (
        <span
          style={{
            fontSize: "0.7rem",
            fontWeight: 600,
            padding: "0.12rem 0.5rem",
            borderRadius: "999px",
            background: configured ? "rgba(16, 185, 129, 0.12)" : "rgba(245, 158, 11, 0.12)",
            border: `1px solid ${configured ? "rgba(16, 185, 129, 0.35)" : "rgba(245, 158, 11, 0.35)"}`,
            color: configured ? "#6ee7b7" : "#fde68a",
          }}
          title={
            configured
              ? "API key detected in application environment"
              : "API key environment variable not detected"
          }
        >
          {configured ? "API Key Detected" : "API Key Not In App Env"}
        </span>
      );
    }
    if (authKind === "session_login") {
      const active = server.authConfigured === true;
      return (
        <span
          style={{
            fontSize: "0.7rem",
            fontWeight: 600,
            padding: "0.12rem 0.5rem",
            borderRadius: "999px",
            background: active ? "rgba(16, 185, 129, 0.12)" : "rgba(99, 102, 241, 0.12)",
            border: `1px solid ${active ? "rgba(16, 185, 129, 0.35)" : "rgba(99, 102, 241, 0.35)"}`,
            color: active ? "#6ee7b7" : "#a5b4fc",
          }}
          title="CLI session login authentication"
        >
          {active ? "Session Active" : "Session Login Required"}
        </span>
      );
    }
    return (
      <span
        style={{
          fontSize: "0.7rem",
          fontWeight: 600,
          padding: "0.12rem 0.5rem",
          borderRadius: "999px",
          background: "rgba(255, 255, 255, 0.06)",
          border: "1px solid var(--border-color)",
          color: "var(--text-muted)",
        }}
      >
        No Auth Required
      </span>
    );
  };

  const renderAuthHint = (server: ExternalServerStatus) => {
    const auth = server.auth;
    const authKind = "kind" in auth ? auth.kind : "";
    const envVar = "env_var" in auth && typeof auth.env_var === "string" ? auth.env_var : "PERPLEXITY_API_KEY";
    const setupCmd = "setup_cmd" in auth && typeof auth.setup_cmd === "string" ? auth.setup_cmd : "pwm login";

    if (authKind === "api_key") {
      return (
        <div style={{ display: "grid", gap: "0.25rem", color: "var(--text-muted)", fontSize: "0.8rem" }}>
          <div>
            Auth hint: export <code>{envVar}</code> in the shell you start Claude Code / Gemini CLI from.
          </div>
          <div style={{ color: "rgba(255,255,255,0.4)", fontSize: "0.74rem" }}>
            The registry never writes secrets into workspace files; app env status is an advisory hint only.
          </div>
        </div>
      );
    }
    if (authKind === "session_login") {
      return (
        <div style={{ display: "grid", gap: "0.25rem", color: "var(--text-muted)", fontSize: "0.8rem" }}>
          <div>
            Auth hint: run <code>{setupCmd}</code> in your terminal (OTP session token, ~30-day expiry, quota-metered).
          </div>
          {server.notes && (
            <div style={{ color: "#fde68a", fontSize: "0.76rem" }}>
              {server.notes}
            </div>
          )}
        </div>
      );
    }
    return null;
  };

  return (
    <div style={{ display: "grid", gap: "1.25rem" }}>
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "flex-start",
          gap: "1rem",
          flexWrap: "wrap",
        }}
      >
        <div>
          <h3 style={{ margin: 0, fontSize: "1rem", fontWeight: 700 }}>External MCP Servers</h3>
          <p style={{ color: "var(--text-muted)", fontSize: "0.82rem", margin: "0.25rem 0 0" }}>
            Register and configure shared external MCP servers (Perplexity, etc.) for coding agents
            in <strong>{shortenPath(workspaceRoot, 32)}</strong>.
          </p>
        </div>
        <button
          type="button"
          className="btn-secondary"
          style={{ marginTop: 0, padding: "0.35rem 0.75rem", fontSize: "0.78rem" }}
          disabled={busy || loading}
          onClick={() => void handleReapply()}
        >
          Re-apply to Configs
        </button>
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
        {status?.servers.map((server) => (
          <div
            key={server.key}
            style={{
              padding: "0.9rem 1.1rem",
              borderRadius: "10px",
              border: `1px solid ${server.enabled ? "rgba(99, 102, 241, 0.4)" : "var(--border-color)"}`,
              background: server.enabled ? "rgba(99, 102, 241, 0.06)" : "rgba(0,0,0,0.22)",
              display: "grid",
              gap: "0.6rem",
            }}
          >
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                flexWrap: "wrap",
                gap: "0.5rem",
              }}
            >
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: "0.6rem",
                  flexWrap: "wrap",
                }}
              >
                <strong style={{ fontSize: "0.95rem", color: "var(--text-main)" }}>
                  {server.displayName}
                </strong>

                {/* Launcher Status Chip */}
                <span
                  style={{
                    fontSize: "0.7rem",
                    fontWeight: 600,
                    padding: "0.12rem 0.5rem",
                    borderRadius: "999px",
                    background: server.launcherFound
                      ? "rgba(16, 185, 129, 0.12)"
                      : "rgba(245, 158, 11, 0.12)",
                    border: `1px solid ${server.launcherFound ? "rgba(16, 185, 129, 0.35)" : "rgba(245, 158, 11, 0.35)"}`,
                    color: server.launcherFound ? "#6ee7b7" : "#fde68a",
                  }}
                  title={server.launcherPath ?? "Command launcher path"}
                >
                  {server.launcherFound ? "Launcher Installed" : "Launcher Missing in App PATH"}
                </span>

                {/* Auth Status Chip */}
                {renderAuthChip(server)}

                {server.docsUrl && (
                  <a
                    href={server.docsUrl}
                    target="_blank"
                    rel="noreferrer"
                    style={{
                      fontSize: "0.72rem",
                      color: "var(--primary-color, #60a5fa)",
                      textDecoration: "none",
                    }}
                  >
                    Docs ↗
                  </a>
                )}
              </div>

              <ToggleRow
                label=""
                checked={server.enabled}
                disabled={busy || loading}
                onToggle={() => void handleToggleServer(server)}
              />
            </div>

            {renderAuthHint(server)}

            {server.launcherPath && (
              <div style={{ fontSize: "0.74rem", color: "var(--text-muted)" }}>
                Launcher path: <code>{server.launcherPath}</code>
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
