import { useCallback, useEffect, useState } from "react";
import { invoke } from "../../../lib/tauri";
import GrokLeaderCard from "../harness/GrokLeaderCard";
import type { ChannelWorkspace } from "./types";
import { cardStyle, inputStyle } from "./HubCharts";

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
    refreshChannelWorkspaces();
  }, [refreshChannelWorkspaces]);

  const grokWorkspace = typeof localStorage !== "undefined" && typeof localStorage.getItem === "function" ? (localStorage.getItem("ca.workspaceRoot") || "") : "";

  return (
    <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1.5rem" }}>
      <div>
        {error && <span style={{ color: "#ef4444", fontSize: "0.85rem", background: "rgba(239, 68, 68, 0.1)", padding: "0.2rem 0.5rem", borderRadius: "4px" }}>Error: {error}</span>}
        {status && !error && <span style={{ color: "#22c55e", fontSize: "0.85rem", background: "rgba(34, 197, 94, 0.1)", padding: "0.2rem 0.5rem", borderRadius: "4px", display: "inline-flex", alignItems: "center", gap: "0.25rem" }}>
          <div style={{ width: "6px", height: "6px", borderRadius: "50%", background: "#22c55e" }} /> {status}
        </span>}
      </div>
      <div style={{ ...cardStyle, display: "grid", gap: "0.85rem" }}>
        <h3 style={{ margin: 0, fontSize: "1.2rem", color: "var(--text-main)" }}>Grok leader</h3>
        <p style={{ margin: 0, color: "var(--text-muted)", fontSize: "0.9rem" }}>
          Grok Build uses a documented leader daemon and ACP socket (<code style={{ background: "rgba(0,0,0,0.3)", padding: "0.1rem 0.4rem", borderRadius: "4px" }}>~/.grok/leader.sock</code>) for Hub task injection (C14.6 / #154).
          Standalone terminals are capture-only; click Connect below or start Grok with <code style={{ background: "rgba(0,0,0,0.3)", padding: "0.1rem 0.4rem", borderRadius: "4px" }}>grok --leader</code> to enable two-way task delivery.
        </p>
        <GrokLeaderCard workspace={grokWorkspace} />
      </div>

      <div style={{ ...cardStyle, display: "grid", gap: "1rem" }}>
        <h3 style={{ margin: 0, fontSize: "1.2rem", color: "var(--text-main)" }}>Claude Channel workspaces</h3>
        <p style={{ margin: 0, color: "var(--text-muted)", fontSize: "0.9rem" }}>
          Every workspace configured for the opt-in Claude Code Channel bridge (C14.3). Configs live under{" "}
          <code style={{ background: "rgba(0,0,0,0.3)", padding: "0.1rem 0.4rem", borderRadius: "4px" }}>~/.coding-assistants/servers/</code>{" "}
          — run <code style={{ background: "rgba(0,0,0,0.3)", padding: "0.1rem 0.4rem", borderRadius: "4px" }}>coding-assistants-claude-channel --setup --workspace &lt;abs path&gt;</code> to add one.
        </p>
        <button className="btn-secondary" onClick={refreshChannelWorkspaces} style={{ alignSelf: "flex-start" }}>Refresh</button>
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
        {channelWorkspaces.length === 0 && (
          <p style={{ color: "var(--text-muted)" }}>No Claude Channel workspaces configured yet.</p>
        )}
        {channelWorkspaces.map((workspace) => (
          <div key={workspace.workspace} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", gap: "1rem", alignItems: "center", flexWrap: "wrap" }}>
            <div style={{ minWidth: 0, flex: "1 1 260px" }}>
              <div style={{ color: "var(--text-muted)", fontSize: "0.8rem", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={workspace.workspace}>
                {workspace.workspace}
              </div>
              <span
                className="status-badge"
                style={{
                  display: "inline-block",
                  marginTop: "0.35rem",
                  padding: "0.15rem 0.6rem",
                  borderRadius: "999px",
                  fontSize: "0.72rem",
                  fontWeight: 600,
                  ...(channelConnected[workspace.workspace]
                    ? { background: "rgba(16, 185, 129, 0.15)", color: "#6ee7b7", border: "1px solid rgba(16, 185, 129, 0.3)" }
                    : { background: "rgba(148, 163, 184, 0.12)", color: "var(--text-muted)", border: "1px solid var(--border-color)" }),
                }}
              >
                {channelConnected[workspace.workspace] ? "● Session connected" : "○ No live session"}
              </span>
            </div>
            <div style={{ display: "flex", gap: "0.5rem", alignItems: "center" }}>
              {!channelConnected[workspace.workspace] && (
                <button
                  className="btn-secondary"
                  disabled={!!channelConnecting[workspace.workspace]}
                  onClick={() => connectChannelWorkspace(workspace.workspace)}
                  title="Open a terminal running `claude --dangerously-load-development-channels server:coding-assistants-channel` in this workspace"
                >
                  {channelConnecting[workspace.workspace] ? "Connecting…" : "Connect"}
                </button>
              )}
              <input
                type="text"
                value={channelRenameDrafts[workspace.workspace] ?? workspace.display_name}
                onChange={(e) => setChannelRenameDrafts((prev) => ({ ...prev, [workspace.workspace]: e.target.value }))}
                style={{ ...inputStyle, width: 200 }}
              />
              <button className="btn-secondary" onClick={() => renameChannelWorkspace(workspace.workspace)}>Rename</button>
              <button className="btn-secondary" style={{ color: "#fca5a5", borderColor: "rgba(248, 113, 113, 0.45)" }} onClick={() => deleteChannelWorkspace(workspace.workspace)}>Remove</button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
