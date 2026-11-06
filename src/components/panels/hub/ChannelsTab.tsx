import { useCallback, useEffect, useState } from "react";
import { invoke } from "../../../lib/tauri";
import GrokLeaderCard from "../harness/GrokLeaderCard";
import type { ChannelWorkspace } from "./types";
import { nocPanelStyle, nocCardDenseStyle } from "../../common/telemetry/panel";
import { EmptyStateChip } from "../../common/telemetry/status";
import { NOC_COLORS, NOC_SURFACES } from "../../common/telemetry/tokens";

export default function ChannelsTab() {
  const [channelWorkspaces, setChannelWorkspaces] = useState<ChannelWorkspace[]>([]);
  const [channelRenameDrafts, setChannelRenameDrafts] = useState<Record<string, string>>({});
  const [channelConnected, setChannelConnected] = useState<Record<string, boolean>>({});
  const [channelConnecting, setChannelConnecting] = useState<Record<string, boolean>>({});
  const [error, setError] = useState<string>("");
  const [status, setStatus] = useState<string>("");

  const run = useCallback(async <T,>(label: string, fn: () => Promise<T>, pendingMessage?: string): Promise<T | null> => {
    setError("");
    if (pendingMessage) {
      setStatus(pendingMessage);
    }
    try {
      const result = await fn();
      setStatus(label);
      return result;
    } catch (e) {
      setError(String(e));
      setStatus("");
      return null;
    }
  }, []);

  const refreshChannelWorkspaces = useCallback(async () => {
    const list = await run("channels refreshed", () =>
      invoke<ChannelWorkspace[]>("claude_channel_list_workspaces")
    );
    if (list) {
      setChannelWorkspaces(list);
      for (const cw of list) {
        invoke<boolean>("hub_is_channel_workspace_connected", { workspace: cw.workspace })
          .then((connected) => {
            setChannelConnected((prev) => ({ ...prev, [cw.workspace]: connected }));
          })
          .catch(() => {});
      }
    }
  }, [run]);

  const connectChannelWorkspace = useCallback(async (workspace: string) => {
    setChannelConnecting((prev) => ({ ...prev, [workspace]: true }));
    try {
      const outcome = await run(
        "channel connection requested",
        () => invoke<{ ok: boolean; pid?: number; detail: string }>("hub_connect_channel_workspace", { workspace }),
        "Starting Claude Code channel…",
      );
      if (outcome?.ok) {
        setChannelConnected((prev) => ({ ...prev, [workspace]: true }));
        setStatus(`Claude Channel connected (pid ${outcome.pid ?? "unknown"})`);
      } else if (outcome) {
        setError(outcome.detail);
      }
    } finally {
      setChannelConnecting((prev) => ({ ...prev, [workspace]: false }));
    }
  }, [run]);

  const renameChannelWorkspace = async (workspace: string) => {
    const draft = channelRenameDrafts[workspace]?.trim();
    if (!draft) return;
    await run("channel renamed", () =>
      invoke("hub_rename_channel_workspace", { workspace, displayName: draft })
    );
    await refreshChannelWorkspaces();
  };

  const deleteChannelWorkspace = async (workspace: string) => {
    await run("channel removed", () =>
      invoke("hub_delete_channel_workspace", { workspace })
    );
    await refreshChannelWorkspaces();
  };

  useEffect(() => {
    void refreshChannelWorkspaces();
  }, [refreshChannelWorkspaces]);

  const grokWorkspace = typeof localStorage !== "undefined" && typeof localStorage.getItem === "function" ? (localStorage.getItem("ca.workspaceRoot") || "") : "";

  return (
    <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1.25rem" }}>
      <div>
        {error && <span style={{ color: NOC_COLORS.crit, fontSize: "0.85rem", background: "rgba(239, 68, 68, 0.1)", padding: "0.2rem 0.5rem", borderRadius: "4px" }}>Error: {error}</span>}
        {status && !error && (
          <span style={{ color: NOC_COLORS.ok, fontSize: "0.85rem", background: "rgba(16, 185, 129, 0.1)", padding: "0.2rem 0.5rem", borderRadius: "4px", display: "inline-flex", alignItems: "center", gap: "0.25rem" }}>
            <div style={{ width: "6px", height: "6px", borderRadius: "50%", background: NOC_COLORS.ok }} /> {status}
          </span>
        )}
      </div>

      {/* Grok Leader Section */}
      <div style={{ ...nocPanelStyle, display: "grid", gap: "0.75rem" }}>
        <h3 style={{ margin: 0, fontSize: "1.1rem", fontWeight: 700, color: NOC_SURFACES.textPrimary }}>Grok Leader</h3>
        <p style={{ margin: 0, color: NOC_SURFACES.textMuted, fontSize: "0.84rem" }}>
          Grok Build uses a documented leader daemon and ACP socket (<code style={{ background: "rgba(0,0,0,0.3)", padding: "0.1rem 0.4rem", borderRadius: "4px" }}>~/.grok/leader.sock</code>) for Hub task injection.
          Click Connect below or start Grok with <code style={{ background: "rgba(0,0,0,0.3)", padding: "0.1rem 0.4rem", borderRadius: "4px" }}>grok --leader</code> to enable two-way delivery.
        </p>
        <GrokLeaderCard workspace={grokWorkspace} />
      </div>

      {/* Claude Channel Workspaces Section */}
      <div style={{ ...nocPanelStyle, display: "grid", gap: "0.75rem" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: "0.5rem" }}>
          <div>
            <h3 style={{ margin: 0, fontSize: "1.1rem", fontWeight: 700, color: NOC_SURFACES.textPrimary }}>Claude Channel Workspaces</h3>
            <p style={{ margin: "0.2rem 0 0", color: NOC_SURFACES.textMuted, fontSize: "0.84rem" }}>
              Workspaces configured for the opt-in Claude Code Channel bridge. Configs reside in <code style={{ background: "rgba(0,0,0,0.3)", padding: "0.1rem 0.4rem", borderRadius: "4px" }}>~/.coding-assistants/servers/</code>.
            </p>
          </div>
          <button className="btn-secondary noc-focusable" onClick={refreshChannelWorkspaces} style={{ fontSize: "0.78rem", padding: "0.3rem 0.75rem" }}>
            Refresh
          </button>
        </div>

        {channelWorkspaces.length === 0 ? (
          <EmptyStateChip
            title="No Claude Channel workspaces configured"
            detail="Run coding-assistants-claude-channel --setup --workspace <abs path> to add one."
            tone="idle"
          />
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: "0.65rem", marginTop: "0.35rem" }}>
            {channelWorkspaces.map((workspace) => {
              const connected = !!channelConnected[workspace.workspace];
              return (
                <div key={workspace.workspace} style={{ ...nocCardDenseStyle, display: "flex", justifyContent: "space-between", gap: "1rem", alignItems: "center", flexWrap: "wrap" }}>
                  <div style={{ minWidth: 0, flex: "1 1 260px" }}>
                    <div style={{ color: NOC_SURFACES.textPrimary, fontSize: "0.85rem", fontWeight: 600, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={workspace.workspace}>
                      {workspace.workspace}
                    </div>
                    <span
                      style={{
                        display: "inline-flex",
                        alignItems: "center",
                        gap: "0.35rem",
                        marginTop: "0.35rem",
                        padding: "0.15rem 0.55rem",
                        borderRadius: "999px",
                        fontSize: "0.72rem",
                        fontWeight: 700,
                        ...(connected
                          ? { background: "rgba(16, 185, 129, 0.15)", color: NOC_COLORS.ok, border: "1px solid rgba(16, 185, 129, 0.35)" }
                          : { background: "rgba(148, 163, 184, 0.12)", color: NOC_COLORS.idle, border: "1px solid rgba(148, 163, 184, 0.25)" }),
                      }}
                    >
                      <i style={{ width: 6, height: 6, borderRadius: "50%", background: connected ? NOC_COLORS.ok : NOC_COLORS.idle, display: "inline-block" }} />
                      {connected ? "Session connected" : "No live session"}
                    </span>
                  </div>
                  <div style={{ display: "flex", gap: "0.45rem", alignItems: "center", flexWrap: "wrap" }}>
                    {!connected && (
                      <button
                        className="btn-secondary noc-focusable"
                        style={{ fontSize: "0.76rem", padding: "0.3rem 0.65rem" }}
                        disabled={!!channelConnecting[workspace.workspace]}
                        onClick={() => connectChannelWorkspace(workspace.workspace)}
                        title="Open a terminal running `claude --dangerously-load-development-channels server:coding-assistants-channel`"
                      >
                        {channelConnecting[workspace.workspace] ? "Connecting…" : "Connect"}
                      </button>
                    )}
                    <input
                      type="text"
                      className="noc-focusable"
                      value={channelRenameDrafts[workspace.workspace] ?? workspace.display_name}
                      onChange={(e) => setChannelRenameDrafts((prev) => ({ ...prev, [workspace.workspace]: e.target.value }))}
                      style={{
                        padding: "0.3rem 0.6rem",
                        borderRadius: "6px",
                        border: "1px solid rgba(148, 163, 184, 0.2)",
                        background: "rgba(0,0,0,0.35)",
                        color: NOC_SURFACES.textPrimary,
                        fontSize: "0.82rem",
                        width: 180,
                      }}
                    />
                    <button className="btn-secondary noc-focusable" style={{ fontSize: "0.76rem", padding: "0.3rem 0.65rem" }} onClick={() => renameChannelWorkspace(workspace.workspace)}>
                      Rename
                    </button>
                    <button className="btn-secondary noc-focusable" style={{ fontSize: "0.76rem", padding: "0.3rem 0.65rem", color: "#fca5a5", borderColor: "rgba(248, 113, 113, 0.45)" }} onClick={() => deleteChannelWorkspace(workspace.workspace)}>
                      Remove
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
