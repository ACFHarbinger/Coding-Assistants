import GrokLeaderCard from "../harness/GrokLeaderCard";
import type { ChannelWorkspace } from "./types";
import { cardStyle, inputStyle } from "./HubCharts";

export default function ChannelsTab(props: {
  channelWorkspaces: ChannelWorkspace[];
  channelRenameDrafts: Record<string, string>;
  setChannelRenameDrafts: (next: (prev: Record<string, string>) => Record<string, string>) => void;
  renameChannelWorkspace: (workspace: string) => void;
  deleteChannelWorkspace: (workspace: string) => void;
  refreshChannelWorkspaces: () => void;
  channelConnected: Record<string, boolean>;
  channelConnecting: Record<string, boolean>;
  connectChannelWorkspace: (workspace: string) => void;
  grokWorkspace: string;
}) {
  const {
    channelWorkspaces,
    channelRenameDrafts,
    setChannelRenameDrafts,
    renameChannelWorkspace,
    deleteChannelWorkspace,
    refreshChannelWorkspaces,
    channelConnected,
    channelConnecting,
    connectChannelWorkspace,
    grokWorkspace,
  } = props;

  return (
    <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1.5rem" }}>
      <div style={{ ...cardStyle, display: "grid", gap: "0.85rem" }}>
        <h3 style={{ margin: 0, fontSize: "1.2rem", color: "var(--text-main)" }}>Grok leader</h3>
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
