import { useCallback, useEffect, useState } from "react";
import { invoke } from "../../../lib/tauri";
import { nocPanelStyle, nocCardDenseStyle } from "../../common/telemetry/panel";
import { EmptyStateChip, StalenessDot } from "../../common/telemetry/status";
import { NOC_COLORS, NOC_SURFACES } from "../../common/telemetry/tokens";
import { inputStyle } from "./HubCharts";
import {
  emptyArgsFromSchema,
  JsonSchemaForm,
  preparedArguments,
  type JsonSchema,
} from "./JsonSchemaForm";

type InvokeServer = {
  key: string;
  displayName: string;
  kind: string;
  command: string;
  args: string[];
};

type InvokeTool = {
  name: string;
  description?: string | null;
  inputSchema: JsonSchema;
};

type CallResult = {
  is_error: boolean;
  text: string;
};

function workspaceRoot(): string | null {
  try {
    const value = localStorage.getItem("ca.workspaceRoot");
    return value && value.startsWith("/") ? value : null;
  } catch {
    return null;
  }
}

export default function McpToolsPanel() {
  const workspace = workspaceRoot();
  const [servers, setServers] = useState<InvokeServer[]>([]);
  const [serverKey, setServerKey] = useState("");
  const [tools, setTools] = useState<InvokeTool[]>([]);
  const [toolName, setToolName] = useState("");
  const [args, setArgs] = useState<Record<string, unknown>>({});
  const [result, setResult] = useState<CallResult | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState<"servers" | "tools" | "call" | null>(null);
  const [lastLoadedAt, setLastLoadedAt] = useState<number | null>(null);

  const selectedTool = tools.find((tool) => tool.name === toolName);

  const loadServers = useCallback(async () => {
    if (!workspace) {
      setServers([]);
      return;
    }
    setBusy("servers");
    setError("");
    try {
      const list = await invoke<InvokeServer[]>("hub_mcp_client_servers", { workspace });
      setServers(list);
      setServerKey((current) => current || list[0]?.key || "");
      setLastLoadedAt(Math.floor(Date.now() / 1000));
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(null);
    }
  }, [workspace]);

  useEffect(() => {
    void loadServers();
  }, [loadServers]);

  useEffect(() => {
    if (!workspace || !serverKey) {
      setTools([]);
      setToolName("");
      return;
    }
    let cancelled = false;
    setBusy("tools");
    setError("");
    setResult(null);
    void invoke<InvokeTool[]>("hub_mcp_client_tools", { workspace, key: serverKey })
      .then((list) => {
        if (cancelled) return;
        setTools(list);
        const next = list[0]?.name || "";
        setToolName(next);
        setArgs(emptyArgsFromSchema(list[0]?.inputSchema));
      })
      .catch((err) => {
        if (!cancelled) {
          setTools([]);
          setError(String(err));
        }
      })
      .finally(() => {
        if (!cancelled) setBusy(null);
      });
    return () => {
      cancelled = true;
    };
  }, [workspace, serverKey]);

  const onSelectTool = (name: string) => {
    setToolName(name);
    const tool = tools.find((item) => item.name === name);
    setArgs(emptyArgsFromSchema(tool?.inputSchema));
    setResult(null);
  };

  const runTool = async () => {
    if (!workspace || !serverKey || !selectedTool) return;
    setBusy("call");
    setError("");
    try {
      const arguments_ = preparedArguments(selectedTool.inputSchema, args);
      const outcome = await invoke<CallResult>("hub_mcp_client_call", {
        workspace,
        key: serverKey,
        tool: selectedTool.name,
        arguments: arguments_,
      });
      setResult(outcome);
    } catch (err) {
      setError(String(err));
      setResult(null);
    } finally {
      setBusy(null);
    }
  };

  if (!workspace) {
    return (
      <div style={nocPanelStyle}>
        <h3 style={{ margin: "0 0 0.75rem", fontSize: "1.05rem", fontWeight: 700, color: NOC_SURFACES.textPrimary }}>MCP tools</h3>
        <EmptyStateChip
          title="No workspace root configured"
          detail="Set a workspace root in Config, then enable a server under Settings → External MCP (Perplexity is the proof-of-concept)."
          tone="idle"
        />
      </div>
    );
  }

  return (
    <div className="fade-in" style={{ display: "grid", gap: "1.25rem" }}>
      <div style={nocPanelStyle}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: "0.5rem", marginBottom: "0.75rem" }}>
          <div>
            <h3 style={{ margin: 0, fontSize: "1.05rem", fontWeight: 700, color: NOC_SURFACES.textPrimary }}>MCP tools</h3>
            <p style={{ margin: "0.2rem 0 0", color: NOC_SURFACES.textMuted, fontSize: "0.82rem" }}>
              Run an enabled MCP tool directly without spending an agent turn. Configure servers in Settings → External MCP.
            </p>
          </div>
          {lastLoadedAt && (
            <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
              <StalenessDot lastUpdatedUnixSeconds={lastLoadedAt} pollIntervalSeconds={60} labelPrefix="live" />
              <button
                className="btn-secondary noc-focusable"
                style={{ fontSize: "0.78rem", padding: "0.25rem 0.6rem" }}
                onClick={() => void loadServers()}
                disabled={busy !== null}
              >
                {busy === "servers" ? "Loading…" : "Refresh"}
              </button>
            </div>
          )}
        </div>

        {servers.length === 0 ? (
          <div style={{ padding: "1rem 0" }}>
            <EmptyStateChip
              title="No enabled MCP servers for this workspace"
              detail="Enable external MCP servers or creative tools in Settings to invoke tools here."
              tone="idle"
            />
          </div>
        ) : (
          <div style={{ display: "grid", gap: "0.75rem", gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))", marginTop: "1rem" }}>
            <label style={{ display: "grid", gap: "0.35rem", fontSize: "0.82rem", color: NOC_SURFACES.textSecondary, fontWeight: 600 }}>
              Server
              <select
                className="noc-focusable"
                value={serverKey}
                onChange={(event) => setServerKey(event.target.value)}
                disabled={busy !== null}
                style={inputStyle}
              >
                {servers.map((server) => (
                  <option key={server.key} value={server.key}>
                    {server.displayName} ({server.kind})
                  </option>
                ))}
              </select>
            </label>
            <label style={{ display: "grid", gap: "0.35rem", fontSize: "0.82rem", color: NOC_SURFACES.textSecondary, fontWeight: 600 }}>
              Tool
              <select
                className="noc-focusable"
                value={toolName}
                onChange={(event) => onSelectTool(event.target.value)}
                disabled={busy !== null || tools.length === 0}
                style={inputStyle}
              >
                {tools.map((tool) => (
                  <option key={tool.name} value={tool.name}>
                    {tool.name}
                  </option>
                ))}
              </select>
            </label>
          </div>
        )}

        {selectedTool?.description ? (
          <p style={{ color: NOC_SURFACES.textMuted, fontSize: "0.85rem", marginTop: "0.75rem", marginBottom: 0 }}>
            {selectedTool.description}
          </p>
        ) : null}

        {selectedTool ? (
          <div style={{ marginTop: "1.25rem", borderTop: "1px solid rgba(148, 163, 184, 0.12)", paddingTop: "1rem" }}>
            <JsonSchemaForm schema={selectedTool.inputSchema} value={args} onChange={setArgs} />
            <button
              className="btn-primary noc-focusable"
              style={{ marginTop: "1rem" }}
              onClick={() => void runTool()}
              disabled={busy !== null}
            >
              {busy === "call" ? "Running…" : busy === "tools" ? "Loading tools…" : "Run tool"}
            </button>
          </div>
        ) : null}

        {error ? (
          <div style={{ color: NOC_COLORS.crit, marginTop: "0.75rem", fontSize: "0.85rem", background: "rgba(239, 68, 68, 0.1)", padding: "0.4rem 0.65rem", borderRadius: "6px" }}>
            {error}
          </div>
        ) : null}
      </div>

      {result ? (
        <div style={nocCardDenseStyle}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "0.5rem" }}>
            <h4 style={{ margin: 0, fontSize: "0.95rem", color: NOC_SURFACES.textPrimary }}>
              {result.is_error ? "Tool Error" : "Execution Result"}
            </h4>
            <span
              style={{
                fontSize: "0.72rem",
                fontWeight: 700,
                padding: "0.15rem 0.5rem",
                borderRadius: "6px",
                background: result.is_error ? "rgba(239, 68, 68, 0.15)" : "rgba(16, 185, 129, 0.15)",
                color: result.is_error ? NOC_COLORS.crit : NOC_COLORS.ok,
                border: `1px solid ${result.is_error ? "rgba(239, 68, 68, 0.35)" : "rgba(16, 185, 129, 0.35)"}`,
              }}
            >
              {result.is_error ? "ERR" : "OK"}
            </span>
          </div>
          <pre
            className="tabular-nums"
            style={{
              whiteSpace: "pre-wrap",
              color: result.is_error ? NOC_COLORS.crit : NOC_SURFACES.textPrimary,
              background: "rgba(2, 6, 23, 0.6)",
              border: "1px solid rgba(148, 163, 184, 0.15)",
              padding: "0.75rem 1rem",
              borderRadius: "8px",
              fontFamily: "var(--font-mono)",
              fontSize: "0.85rem",
              margin: 0,
              maxHeight: 360,
              overflowY: "auto",
            }}
          >
            {result.text}
          </pre>
        </div>
      ) : null}
    </div>
  );
}
