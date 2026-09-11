import { useCallback, useEffect, useState } from "react";
import { invoke } from "../../../lib/tauri";
import { cardStyle } from "./HubCharts";
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
      <div style={cardStyle}>
        <h3 style={{ marginTop: 0 }}>MCP tools</h3>
        <p style={{ color: "var(--text-muted)" }}>
          Set a workspace root in Config, then enable a server under Settings → External MCP
          (Perplexity is the proof-of-concept).
        </p>
      </div>
    );
  }

  return (
    <div style={{ display: "grid", gap: "1rem" }}>
      <div style={cardStyle}>
        <h3 style={{ marginTop: 0 }}>MCP tools</h3>
        <p style={{ color: "var(--text-muted)", fontSize: "0.85rem" }}>
          Run an enabled MCP tool without spending an agent turn. Enable servers in Settings →
          External MCP / Creative Tools.
        </p>
        {servers.length === 0 ? (
          <p style={{ color: "var(--text-muted)" }}>No enabled MCP servers for this workspace.</p>
        ) : (
          <div style={{ display: "grid", gap: "0.75rem", gridTemplateColumns: "1fr 1fr" }}>
            <label style={{ display: "grid", gap: "0.35rem" }}>
              Server
              <select
                value={serverKey}
                onChange={(event) => setServerKey(event.target.value)}
                disabled={busy !== null}
              >
                {servers.map((server) => (
                  <option key={server.key} value={server.key}>
                    {server.displayName} ({server.kind})
                  </option>
                ))}
              </select>
            </label>
            <label style={{ display: "grid", gap: "0.35rem" }}>
              Tool
              <select
                value={toolName}
                onChange={(event) => onSelectTool(event.target.value)}
                disabled={busy !== null || tools.length === 0}
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
          <p style={{ color: "var(--text-muted)", fontSize: "0.85rem" }}>{selectedTool.description}</p>
        ) : null}
        {selectedTool ? (
          <div style={{ marginTop: "1rem" }}>
            <JsonSchemaForm schema={selectedTool.inputSchema} value={args} onChange={setArgs} />
            <button
              className="btn-primary"
              style={{ marginTop: "1rem" }}
              onClick={() => void runTool()}
              disabled={busy !== null}
            >
              {busy === "call" ? "Running…" : busy === "tools" ? "Loading tools…" : "Run tool"}
            </button>
          </div>
        ) : null}
        {error ? (
          <p style={{ color: "#ef4444", marginTop: "0.75rem" }}>{error}</p>
        ) : null}
      </div>
      {result ? (
        <div style={cardStyle}>
          <h3 style={{ marginTop: 0 }}>{result.is_error ? "Tool error" : "Result"}</h3>
          <pre
            style={{
              whiteSpace: "pre-wrap",
              color: result.is_error ? "#ef4444" : "var(--text-main)",
              background: "rgba(0,0,0,0.35)",
              padding: "0.75rem",
              borderRadius: "8px",
            }}
          >
            {result.text}
          </pre>
        </div>
      ) : null}
    </div>
  );
}
