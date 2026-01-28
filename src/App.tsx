import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

interface AgentEvent {
  source: string;
  event_type: string;
  content: string;
}

interface ModelConfig {
  provider: string;
  model: string;
  prompt_file?: string;
  rule_file?: string;
  workflow_file?: string;
}

interface RoleConfig {
  name: string;
  config: ModelConfig;
}

interface AgentConfig {
  roles: RoleConfig[];
  work_dir: string;
  mcp_config: string;
}

interface AgentResources {
  prompts: string[];
  rules: string[];
  workflows: string[];
}

const PROVIDERS = {
  opencode: "OpenCode Zen",
  google: "Google",
  anthropic: "Anthropic",
  openai: "OpenAI",
  github_copilot: "GitHub Copilot",
  ollama: "Ollama",
  lm_studio: "LM Studio"
};

// MODELS was here, removed.

const ModelSelect = ({
  index,
  role,
  availableModels,
  onProviderChange,
  onConfigChange,
  onNameChange,
  onRemove,
  onPreview,
  resources,
  PROVIDERS
}: {
  index: number;
  role: RoleConfig;
  availableModels: Record<string, string[]>;
  onProviderChange: (index: number, provider: string) => void;
  onConfigChange: (index: number, field: keyof ModelConfig, value: any) => void;
  onNameChange: (index: number, name: string) => void;
  onRemove: (index: number) => void;
  onPreview: (type: string, name?: string) => Promise<void>;
  resources: AgentResources;
  PROVIDERS: Record<string, string>;
}) => {
  const roleConfig = role.config;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem', padding: '1rem', border: '1px solid var(--border-color)', borderRadius: '0.5rem', background: 'rgba(255, 255, 255, 0.05)', position: 'relative' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <input
          value={role.name}
          onChange={(e) => onNameChange(index, e.target.value)}
          style={{
            background: 'transparent',
            border: 'none',
            borderBottom: '1px solid transparent',
            fontSize: '1rem',
            fontWeight: 600,
            color: 'var(--primary)',
            padding: '2px 0',
            outline: 'none',
            width: 'auto',
            minWidth: '100px'
          }}
          onFocus={(e) => e.target.style.borderBottom = '1px solid var(--primary)'}
          onBlur={(e) => e.target.style.borderBottom = '1px solid transparent'}
        />
        <button
          onClick={() => onRemove(index)}
          style={{
            padding: '0.25rem 0.5rem',
            fontSize: '0.8rem',
            background: 'rgba(239, 68, 68, 0.1)',
            color: '#ef4444',
            border: '1px solid rgba(239, 68, 68, 0.2)',
            borderRadius: '0.25rem',
            cursor: 'pointer'
          }}
        >
          Remove
        </button>
      </div>

      <div>
        <label className="label">Provider</label>
        <select
          value={roleConfig.provider}
          onChange={(e) => onProviderChange(index, e.target.value)}
        >
          {Object.entries(PROVIDERS).map(([id, name]) => (
            <option key={id} value={id}>{name}</option>
          ))}
        </select>
      </div>
      <div>
        <label className="label">Model</label>
        <select
          value={roleConfig.model}
          onChange={(e) => onConfigChange(index, 'model', e.target.value)}
        >
          {(availableModels[roleConfig.provider] || []).map(model => (
            <option key={model} value={model}>{model}</option>
          ))}
        </select>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: '0.5rem' }}>
        <div>
          <label
            className="label"
            style={{ fontSize: '0.8rem', cursor: 'pointer', textDecoration: 'underline' }}
            onClick={() => onPreview('prompt', roleConfig.prompt_file)}
            title="Click to preview selected prompt"
          >
            Prompt
          </label>
          <select
            value={roleConfig.prompt_file || ""}
            onChange={(e) => onConfigChange(index, 'prompt_file', e.target.value)}
            style={{ fontSize: '0.85rem', padding: '0.4rem' }}
          >
            <option value="">Default</option>
            {resources.prompts.map(f => <option key={f} value={f}>{f.split('/').pop()}</option>)}
          </select>
        </div>
        <div>
          <label
            className="label"
            style={{ fontSize: '0.8rem', cursor: 'pointer', textDecoration: 'underline' }}
            onClick={() => onPreview('rule', roleConfig.rule_file)}
            title="Click to preview selected rule"
          >
            Rule
          </label>
          <select
            value={roleConfig.rule_file || ""}
            onChange={(e) => onConfigChange(index, 'rule_file', e.target.value)}
            style={{ fontSize: '0.85rem', padding: '0.4rem' }}
          >
            <option value="">None</option>
            {resources.rules.map(f => <option key={f} value={f}>{f.split('/').pop()}</option>)}
          </select>
        </div>
        <div>
          <label
            className="label"
            style={{ fontSize: '0.8rem', cursor: 'pointer', textDecoration: 'underline' }}
            onClick={() => onPreview('workflow', roleConfig.workflow_file)}
            title="Click to preview selected workflow"
          >
            Workflow
          </label>
          <select
            value={roleConfig.workflow_file || ""}
            onChange={(e) => onConfigChange(index, 'workflow_file', e.target.value)}
            style={{ fontSize: '0.85rem', padding: '0.4rem' }}
          >
            <option value="">None</option>
            {resources.workflows.map(f => <option key={f} value={f}>{f.split('/').pop()}</option>)}
          </select>
        </div>
      </div>
    </div>
  );
};

function App() {
  const [config, setConfig] = useState<AgentConfig>({
    roles: [
      { name: "Planner", config: { provider: "openai", model: "gpt-4o" } },
      { name: "Developer", config: { provider: "openai", model: "gpt-4o-mini" } },
      { name: "Reviewer", config: { provider: "openai", model: "gpt-4o" } },
    ],
    work_dir: "./workspace",
    mcp_config: `{
  "mcpServers": {
    "sequential-thinking": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-sequential-thinking"],
      "env": {}
    },
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/home/pkhunter/Repositories/Coding-Assistants"],
      "disabledTools": ["read_file"]
    },
    "memory": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-memory"]
    }
  }
}`
  });
  const [resources, setResources] = useState<AgentResources>({ prompts: [], rules: [], workflows: [] });
  const [task, setTask] = useState("");
  const [output, setOutput] = useState("");
  const [loading, setLoading] = useState(false);
  const [events, setEvents] = useState<AgentEvent[]>([]);
  const [preview, setPreview] = useState<{ type: string, name: string, content: string } | null>(null);
  const [currentQuestion, setCurrentQuestion] = useState<string | null>(null);
  const [authorizationRequest, setAuthorizationRequest] = useState<{ role: string, question: string } | null>(null);
  const [userInput, setUserInput] = useState("");
  const [remoteStatus, setRemoteStatus] = useState<string>("Server not started");
  const [serverIP, setServerIP] = useState<string>("");
  const [remoteLogs, setRemoteLogs] = useState<string[]>([]);

  const startRemoteServer = async () => {
    try {
      setRemoteStatus("Starting server...");
      const address = await invoke<string>("start_tcp_server");
      setServerIP(address);
      setRemoteStatus(`Server listening on ${address}`);
    } catch (e) {
      setRemoteStatus(`Error: ${e}`);
    }
  };

  const stopRemoteServer = async () => {
    try {
      setRemoteStatus("Stopping server...");
      await invoke("stop_tcp_server");
      setRemoteStatus("Server stopped");
      setServerIP("");
    } catch (e) {
      setRemoteStatus(`Error stopping server: ${e}`);
    }
  };

  useEffect(() => {
    const unlisten = listen<string>("remote-status", (event) => {
      setRemoteLogs(prev => [event.payload, ...prev].slice(0, 10));
    });
    return () => {
      unlisten.then(f => f());
    };
  }, []);

  const fetchPreview = async (type: string, name?: string) => {
    if (!config.work_dir) return;
    if (!name) {
      alert(`Please select a custom ${type} file to preview it.`);
      return;
    }
    console.log(`Fetching preview for ${type}: ${name}`);
    try {
      const path = name; // Name is now the full relative path
      const content = await invoke<string>("get_resource_content", { workDir: config.work_dir, path });
      setPreview({ type, name, content });
    } catch (e) {
      console.error(`Failed to fetch ${type} preview:`, e);
      alert(`Failed to load preview: ${e}`);
    }
  };


  useEffect(() => {
    async function fetchResources() {
      if (!config.work_dir) return;
      try {
        const resources = await invoke<AgentResources>("get_agent_resources", { workDir: config.work_dir });
        setResources(resources);
      } catch (e) {
        console.error("Failed to fetch resources:", e);
      }
    }
    fetchResources();
  }, [config.work_dir]);

  useEffect(() => {
    const unlisten = listen<AgentEvent>("agent-event", (event) => {
      setEvents((prev) => {
        const last = prev[prev.length - 1];
        if (event.payload.event_type === "stream") {
          // If the last event was a response from the same source, append to it
          if (last && last.source === event.payload.source && last.event_type === "response") {
            const newLast = { ...last, content: last.content + event.payload.content };
            return [...prev.slice(0, -1), newLast];
          }
          // Otherwise, start a new response block
          return [...prev, { ...event.payload, event_type: "response" }];
        }
        if (event.payload.event_type === "question") {
          setCurrentQuestion(event.payload.content);
        }
        if (event.payload.event_type === "authorization") {
          try {
            const content = JSON.parse(event.payload.content);
            setAuthorizationRequest(content);
          } catch (e) {
            console.error("Failed to parse authorization request", e);
          }
        }
        // For standard events (thought, etc.)
        return [...prev, event.payload];
      });
    });
    return () => {
      unlisten.then(f => f());
    };
  }, []);

  useEffect(() => {
    const unlistenTask = listen<{ config: AgentConfig, task: string }>("android-task-request", (event) => {
      console.log("Received remote task request:", event.payload);
      setConfig(event.payload.config);
      setTask(event.payload.task);
      startTaskRemote(event.payload.config, event.payload.task);
    });

    const unlistenCancel = listen("android-cancel-request", () => {
      console.log("Received remote cancel request");
      invoke("cancel_task").catch(err => console.error("Remote cancel failed:", err));
    });

    const unlistenInput = listen<string>("android-input-submit", (event) => {
      console.log("Received remote input submit:", event.payload);
      setUserInput(event.payload);
      invoke("submit_user_input", { input: event.payload }).catch(err => console.error("Remote input submittal failed:", err));
      setCurrentQuestion(null);
    });

    return () => {
      unlistenTask.then(f => f());
      unlistenCancel.then(f => f());
      unlistenInput.then(f => f());
    };
  }, []);

  const [availableModels, setAvailableModels] = useState<Record<string, string[]>>({});

  useEffect(() => {
    async function loadModels() {
      try {
        const models = await invoke<Record<string, string[]>>("get_available_models");
        setAvailableModels(models);
        console.log("Loaded models:", models);
      } catch (err) {
        console.error("Failed to load models:", err);
      }
    }
    loadModels();
  }, []);

  const submitAnswer = async () => {
    if (!userInput.trim()) return;
    try {
      await invoke("submit_user_input", { input: userInput });
      setCurrentQuestion(null);
      setUserInput("");
    } catch (e) {
      console.error("Failed to submit answer:", e);
      alert("Failed to submit answer: " + e);
    }
  };

  const respondToAuthorization = async (approved: boolean) => {
    try {
      await invoke("submit_user_input", { input: approved ? "APPROVED" : "DENIED" });
      setAuthorizationRequest(null);
    } catch (e) {
      console.error("Failed to submit authorization response:", e);
      alert("Failed to submit response: " + e);
    }
  };

  const startTask = async () => {
    if (loading) {
      // Cancel logic
      try {
        await invoke("cancel_task");
        setOutput(prev => prev + "\n[Cancelling task...]");
      } catch (error) {
        console.error("Failed to cancel task:", error);
      }
      return;
    }

    setLoading(true);
    setEvents([]);
    setOutput("");
    try {
      const result = await invoke<string>("run_agent_task", { config, task });
      setOutput(result);
    } catch (error) {
      setOutput(`Error: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  const startTaskRemote = async (remoteConfig: AgentConfig, remoteTask: string) => {
    setLoading(true);
    setEvents([]);
    setOutput("");
    try {
      const result = await invoke<string>("run_agent_task", { config: remoteConfig, task: remoteTask });
      setOutput(result);
    } catch (error) {
      setOutput(`Error: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  const handleProviderChange = (index: number, provider: string) => {
    setConfig(prev => {
      const newRoles = [...prev.roles];
      const models = availableModels[provider] || [];
      newRoles[index] = {
        ...newRoles[index],
        config: {
          ...newRoles[index].config,
          provider,
          model: models[0] || ""
        }
      };
      return { ...prev, roles: newRoles };
    });
  };

  const addRole = () => {
    setConfig(prev => ({
      ...prev,
      roles: [...prev.roles, {
        name: `New Role ${prev.roles.length + 1}`,
        config: { provider: "openai", model: "gpt-4o-mini" }
      }]
    }));
  };

  const removeRole = (index: number) => {
    setConfig(prev => ({
      ...prev,
      roles: prev.roles.filter((_, i) => i !== index)
    }));
  };

  const updateRoleName = (index: number, name: string) => {
    setConfig(prev => {
      const newRoles = [...prev.roles];
      newRoles[index] = { ...newRoles[index], name };
      return { ...prev, roles: newRoles };
    });
  };

  const updateRoleConfig = (index: number, field: keyof ModelConfig, value: any) => {
    setConfig(prev => {
      const newRoles = [...prev.roles];
      newRoles[index] = {
        ...newRoles[index],
        config: { ...newRoles[index].config, [field]: value || undefined }
      };
      return { ...prev, roles: newRoles };
    });
  };

  return (
    <div className="app-container" style={{ flexDirection: 'column' }}>
      <header style={{
        padding: '1.5rem 2.5rem',
        borderBottom: '1px solid var(--border-color)',
        background: 'rgba(2, 6, 23, 0.3)',
        backdropFilter: 'var(--glass-blur)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between'
      }}>
        <h1 style={{ fontSize: '1.4rem', fontWeight: 800, letterSpacing: '-0.025em' }}>
          Coding Assistants
        </h1>
        <div className="status-badge">Powered by OpenCode</div>
      </header>

      <main className="main-content">
        <div className="glass-card">
          <h2>Configuration</h2>

          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(300px, 1fr))', gap: '2rem' }}>
            {config.roles.map((role, index) => (
              <ModelSelect
                key={index}
                index={index}
                role={role}
                availableModels={availableModels}
                onProviderChange={handleProviderChange}
                onConfigChange={updateRoleConfig}
                onNameChange={updateRoleName}
                onRemove={removeRole}
                onPreview={fetchPreview}
                resources={resources}
                PROVIDERS={PROVIDERS}
              />
            ))}
            <div
              onClick={addRole}
              style={{
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                justifyContent: 'center',
                gap: '1rem',
                padding: '2rem',
                border: '2px dashed var(--border-color)',
                borderRadius: '0.5rem',
                background: 'rgba(255, 255, 255, 0.02)',
                cursor: 'pointer',
                transition: 'all 0.2s ease',
                minHeight: '200px'
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.borderColor = 'var(--primary)';
                e.currentTarget.style.background = 'rgba(168, 85, 247, 0.05)';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.borderColor = 'var(--border-color)';
                e.currentTarget.style.background = 'rgba(255, 255, 255, 0.02)';
              }}
            >
              <span style={{ fontSize: '2rem', color: 'var(--text-muted)' }}>+</span>
              <span style={{ fontWeight: 600, color: 'var(--text-primary)' }}>Add New Role</span>
            </div>

            <div style={{ gridColumn: '1 / -1' }}>
              <label className="label">Workspace Root</label>
              <div style={{ display: 'flex', gap: '1rem' }}>
                <input
                  style={{ flex: 1 }}
                  placeholder="./workspace"
                  value={config.work_dir}
                  onChange={e => setConfig({ ...config, work_dir: e.target.value })}
                />
                <button
                  className="btn-secondary"
                  onClick={async () => {
                    const selected = await open({
                      directory: true,
                      multiple: false,
                    });
                    if (selected) {
                      setConfig({ ...config, work_dir: selected as string });
                    }
                  }}
                >
                  Browse
                </button>
              </div>
            </div>

            <div style={{ gridColumn: '1 / -1' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.5rem' }}>
                <label className="label" style={{ margin: 0 }}>MCP Configuration (JSON)</label>
                <button
                  className="btn-secondary"
                  style={{ fontSize: '0.8rem', padding: '0.2rem 0.5rem' }}
                  onClick={async () => {
                    try {
                      const selected = await open({
                        multiple: false
                      });
                      if (selected) {
                        const content = await invoke<string>("read_file_absolute", { path: selected as string });
                        setConfig({ ...config, mcp_config: content });
                      }
                    } catch (err) {
                      console.error("Failed to load config", err);
                      alert("Failed to load config: " + err);
                    }
                  }}
                >
                  Load Config...
                </button>
              </div>
              <textarea
                value={config.mcp_config}
                onChange={(e) => setConfig({ ...config, mcp_config: e.target.value })}
                placeholder="Paste mcp_servers.json content here..."
                style={{
                  minHeight: '150px',
                  fontFamily: 'monospace',
                  fontSize: '0.9rem',
                  lineHeight: '1.4',
                  backgroundColor: 'rgba(0, 0, 0, 0.2)',
                  color: 'var(--text-primary)',
                  border: '1px solid var(--border-color)',
                  borderRadius: '4px',
                  padding: '0.75rem',
                  width: '100%',
                  resize: 'vertical'
                }}
              />
            </div>
          </div>
        </div>

        <div className="glass-card">
          <h2>Execute Task</h2>
          <textarea
            rows={5}
            placeholder="What should the agent team build today?"
            value={task}
            onChange={e => setTask(e.target.value)}
          />
          <div style={{ marginTop: '1.5rem', display: 'flex', justifyContent: 'flex-end', alignItems: 'center', gap: '1.5rem' }}>
            {loading && <span style={{ color: 'var(--text-muted)', fontSize: '0.9rem', fontStyle: 'italic' }}>Orchestrating agents via OpenCode...</span>}
            <button className={loading ? "btn-secondary" : "btn-primary"} onClick={startTask} disabled={!task && !loading}>
              {loading ? "Cancel" : "Launch Sequence"}
            </button>
          </div>
        </div>

        <div className="glass-card">
          <h2>Remote Control</h2>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1rem' }}>
            <div>
              <p style={{ margin: 0, fontWeight: 600 }}>Status: <span style={{ color: remoteStatus.includes("listening") ? "#22c55e" : "var(--text-muted)" }}>{remoteStatus}</span></p>
              <p style={{ margin: '4px 0 0 0', fontSize: '0.8rem', color: 'var(--text-muted)' }}>Control from Android app via WiFi</p>
              {serverIP && (
                <p style={{ margin: '8px 0 0 0', fontSize: '0.9rem', fontWeight: 600, color: 'var(--primary)' }}>
                  Connect to: <span style={{ fontFamily: 'monospace', background: 'rgba(168, 85, 247, 0.1)', padding: '4px 8px', borderRadius: '4px' }}>{serverIP}</span>
                </p>
              )}
            </div>
            <button
              className="btn-secondary"
              onClick={remoteStatus.includes("listening") ? stopRemoteServer : startRemoteServer}
              style={remoteStatus.includes("listening") ? { borderColor: '#ef4444', color: '#ef4444' } : {}}
            >
              {remoteStatus.includes("listening") ? "Stop Server" : "Start Server"}
            </button>
          </div>
          {remoteLogs.length > 0 && (
            <div style={{ background: 'rgba(0,0,0,0.2)', padding: '0.75rem', borderRadius: '0.5rem' }}>
              <p style={{ margin: '0 0 0.5rem 0', fontSize: '0.8rem', fontWeight: 600, color: 'var(--text-muted)' }}>Remote Logs:</p>
              <ul style={{ margin: 0, paddingLeft: '1.2rem', fontSize: '0.85rem' }}>
                {remoteLogs.map((log, i) => <li key={i}>{log}</li>)}
              </ul>
            </div>
          )}
        </div>

        {events.length > 0 && (
          <div className="glass-card">
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1rem' }}>
              <h2>Agent Activity</h2>
              <button
                onClick={() => setEvents([])}
                style={{ background: 'transparent', border: 'none', color: 'var(--text-muted)', cursor: 'pointer', fontSize: '0.8rem' }}
              >
                Clear Events
              </button>
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
              {events.map((ev, i) => (
                <div key={i} style={{ border: '1px solid var(--border-color)', borderRadius: '0.5rem', overflow: 'hidden' }}>
                  <div style={{
                    padding: '0.75rem 1rem',
                    background: 'rgba(255, 255, 255, 0.03)',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    borderBottom: '1px solid var(--border-color)'
                  }}>
                    <div style={{ display: 'flex', gap: '0.5rem', alignItems: 'center' }}>
                      {(() => {
                        const colors = {
                          Planner: { bg: 'rgba(56, 189, 248, 0.2)', text: '#38bdf8' },
                          Developer: { bg: 'rgba(168, 85, 247, 0.2)', text: '#a855f7' },
                          Reviewer: { bg: 'rgba(234, 179, 8, 0.2)', text: '#eab308' },
                          System: { bg: 'rgba(255, 255, 255, 0.1)', text: 'var(--text-muted)' },
                          User: { bg: 'rgba(34, 197, 94, 0.2)', text: '#22c55e' },
                        }[ev.source] || { bg: 'rgba(168, 85, 247, 0.15)', text: 'var(--primary)' };

                        return (
                          <span className={`badge ${ev.source.toLowerCase()}`} style={{
                            background: colors.bg,
                            color: colors.text,
                            padding: '0.2rem 0.5rem',
                            borderRadius: '0.25rem',
                            fontSize: '0.75rem',
                            fontWeight: 600
                          }}>
                            {ev.source}
                          </span>
                        );
                      })()}
                      <span style={{ fontSize: '0.9rem', color: 'var(--text-muted)' }}>
                        {ev.event_type === "thought" ? "is thinking..." : "responded"}
                      </span>
                    </div>
                    <span style={{ fontSize: '0.8rem', color: 'var(--text-muted)' }}>Event #{i + 1}</span>
                  </div>
                  <pre style={{
                    margin: 0,
                    padding: '1rem',
                    whiteSpace: 'pre-wrap',
                    fontSize: '0.85rem',
                    maxHeight: '300px',
                    overflowY: 'auto',
                    background: 'rgba(0,0,0,0.2)'
                  }}>
                    {ev.content}
                  </pre>
                </div>
              ))}
            </div>
          </div>
        )}

        {output && (
          <div className="glass-card" style={{ borderLeft: '4px solid var(--primary)' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1rem' }}>
              <h2>Final Output</h2>
              <button
                onClick={() => setOutput("")}
                style={{ background: 'transparent', border: 'none', color: 'var(--text-muted)', cursor: 'pointer', fontSize: '0.8rem' }}
              >
                Clear
              </button>
            </div>
            <pre style={{ whiteSpace: 'pre-wrap' }}>
              {output}
            </pre>
          </div>
        )}

        {preview && (
          <div style={{
            position: 'fixed',
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            background: 'rgba(0,0,0,0.8)',
            display: 'flex',
            justifyContent: 'center',
            alignItems: 'center',
            zIndex: 1000,
            backdropFilter: 'blur(5px)'
          }} onClick={() => setPreview(null)}>
            <div
              style={{
                background: 'var(--card-bg)',
                border: '1px solid var(--border-color)',
                borderRadius: '1rem',
                padding: '2rem',
                maxWidth: '800px',
                width: '90%',
                maxHeight: '80vh',
                overflow: 'auto',
                boxShadow: 'var(--shadow-lg)'
              }}
              onClick={e => e.stopPropagation()}
            >
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1rem' }}>
                <h2 style={{ textTransform: 'capitalize' }}>{preview.type} Preview: {preview.name}</h2>
                <button onClick={() => setPreview(null)} className="btn-secondary">Close</button>
              </div>
              <pre style={{ whiteSpace: 'pre-wrap', background: 'rgba(0,0,0,0.3)', padding: '1rem', borderRadius: '0.5rem' }}>
                {preview.content}
              </pre>
            </div>
          </div>
        )}

        {
          authorizationRequest && (
            <div style={{
              position: 'fixed',
              top: 0,
              left: 0,
              right: 0,
              bottom: 0,
              background: 'rgba(0,0,0,0.85)',
              display: 'flex',
              justifyContent: 'center',
              alignItems: 'center',
              zIndex: 2000,
              backdropFilter: 'blur(8px)'
            }}>
              <div style={{
                background: 'var(--card-bg)',
                border: '1px solid var(--primary)',
                borderRadius: '1rem',
                padding: '2rem',
                maxWidth: '500px',
                width: '90%',
                boxShadow: '0 0 50px rgba(168, 85, 247, 0.2)'
              }}>
                <h2 style={{ marginTop: 0, color: 'var(--text-primary)', fontSize: '1.2rem', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                  🛡️ Authorization Required
                </h2>
                <div style={{ margin: '1.5rem 0', fontSize: '1rem', lineHeight: '1.6' }}>
                  <p>An agent wants to ask a question to the <strong style={{ color: 'var(--primary)' }}>{authorizationRequest.role}</strong>:</p>
                  <div style={{ background: 'rgba(0,0,0,0.2)', padding: '1rem', borderRadius: '0.5rem', fontStyle: 'italic' }}>
                    "{authorizationRequest.question}"
                  </div>
                </div>
                <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '1rem' }}>
                  <button className="btn-secondary" style={{ borderColor: '#ef4444', color: '#ef4444' }} onClick={() => respondToAuthorization(false)}>
                    Deny
                  </button>
                  <button className="btn-primary" onClick={() => respondToAuthorization(true)}>
                    Approve
                  </button>
                </div>
              </div>
            </div>
          )
        }

        {
          currentQuestion && (
            <div style={{
              position: 'fixed',
              top: 0,
              left: 0,
              right: 0,
              bottom: 0,
              background: 'rgba(0,0,0,0.85)',
              display: 'flex',
              justifyContent: 'center',
              alignItems: 'center',
              zIndex: 2000,
              backdropFilter: 'blur(8px)'
            }}>
              <div style={{
                background: 'var(--card-bg)',
                border: '1px solid var(--primary)',
                borderRadius: '1rem',
                padding: '2rem',
                maxWidth: '600px',
                width: '90%',
                boxShadow: '0 0 50px rgba(56, 189, 248, 0.2)'
              }}>
                <h2 style={{ marginTop: 0, color: 'var(--primary)', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                  <span>❓</span> Agent Needs Input
                </h2>
                <p style={{ fontSize: '1.1rem', lineHeight: '1.6', margin: '1.5rem 0' }}>
                  {currentQuestion}
                </p>
                <textarea
                  value={userInput}
                  onChange={e => setUserInput(e.target.value)}
                  placeholder="Type your answer here..."
                  rows={4}
                  style={{
                    width: '100%',
                    background: 'rgba(0,0,0,0.3)',
                    border: '1px solid var(--border-color)',
                    borderRadius: '0.5rem',
                    padding: '1rem',
                    color: 'var(--text-primary)',
                    marginBottom: '1rem',
                    resize: 'vertical'
                  }}
                  autoFocus
                />
                <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '1rem' }}>
                  <button className="btn-primary" onClick={submitAnswer}>
                    Submit Answer
                  </button>
                </div>
              </div>
            </div>
          )
        }
      </main >
    </div >
  );
}

export default App;
