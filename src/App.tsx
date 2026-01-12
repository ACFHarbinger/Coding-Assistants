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

interface AgentConfig {
  planner: ModelConfig;
  developer: ModelConfig;
  reviewer: ModelConfig;
  work_dir: string;
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
  github_copilot: "GitHub Copilot"
};

const MODELS: Record<string, string[]> = {
  opencode: ["big-pickle", "grok-code-fast-1", "minimax-m2.1", "glm-4.7"],
  google: [
    "gemini-flash-latest",
    "gemini-flash-lite-latest",
    "gemini-3-pro-preview",
    "gemini-3-flash-preview",
    "gemini-2.5-pro",
    "gemini-2.5-pro-preview-05-06",
    "gemini-2.5-pro-preview-06-05",
    "gemini-2.5-pro-preview-tts",
    "gemini-2.5-flash",
    "gemini-2.5-flash-preview-04-17",
    "gemini-2.5-flash-preview-05-20",
    "gemini-2.5-flash-preview-09-25",
    "gemini-2.5-flash-preview-tts",
    "gemini-2.5-flash-image",
    "gemini-2.5-flash-image-preview",
    "gemini-2.5-flash-lite",
    "gemini-2.5-flash-lite-preview-05-25",
    "gemini-2.5-flash-lite-preview-06-17",
    "gemini-2.0-flash",
    "gemini-2.0-flash-lite",
    "gemini-1.5-pro",
    "gemini-1.5-flash",
    "gemini-1.5-flash-8b",
    "gemini-live-2.5-flash",
    "gemini-live-2.5-flash-preview-native-audio",
    "gemini-embedding-001"
  ],
  anthropic: [
    "claude-sonnet-4.5-latest",
    "claude-sonnet-4.5",
    "claude-sonnet-4-latest",
    "claude-sonnet-4",
    "claude-sonnet-3.7-latest",
    "claude-sonnet-3.7",
    "claude-sonnet-3.5-v2",
    "claude-sonnet-3.5",
    "claude-sonnet-3",
    "claude-opus-4.5-latest",
    "claude-opus-4.5",
    "claude-opus-4.1-latest",
    "claude-opus-4.1",
    "claude-opus-4-latest",
    "claude-opus-4",
    "claude-opus-3",
    "claude-haiku-4.5-latest",
    "claude-haiku-4.5",
    "claude-haiku-3.5-latest",
    "claude-haiku-3.5",
    "claude-haiku-3"
  ],
  openai: ["gpt-5.1-codex-max", "gpt-5.1-codex-mini", "gpt-5.2-codex", "gpt-5.2"],
  github_copilot: [
    "claude-haiku-4.5",
    "claude-opus-4.1",
    "claude-opus-4.5",
    "claude-sonnet-4",
    "claude-sonnet-4.5",
    "gpt-4.1",
    "gpt-4o",
    "gpt-5",
    "gpt-5-codex",
    "gpt-5-mini",
    "gpt-5.1",
    "gpt-5.1-codex",
    "gpt-5.1-codex-max",
    "gpt-5.1-codex-mini",
    "gpt-5.2",
    "gemini-2.5-pro",
    "gemini-3-flash",
    "gemini-3-pro-preview",
    "grok-code-fast-1",
    "raptor-mini-preview"
  ]
};

function App() {
  const [config, setConfig] = useState<AgentConfig>({
    planner: { provider: "openai", model: "gpt-4o" },
    developer: { provider: "openai", model: "gpt-4o-mini" },
    reviewer: { provider: "openai", model: "gpt-4o" },
    work_dir: "./workspace",
  });
  const [resources, setResources] = useState<AgentResources>({ prompts: [], rules: [], workflows: [] });
  const [task, setTask] = useState("");
  const [output, setOutput] = useState("");
  const [loading, setLoading] = useState(false);
  const [events, setEvents] = useState<AgentEvent[]>([]);
  const [preview, setPreview] = useState<{ type: string, name: string, content: string } | null>(null);

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
        // For standard events (thought, etc.)
        return [...prev, event.payload];
      });
    });
    return () => {
      unlisten.then(f => f());
    };
  }, []);

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

  const handleProviderChange = (key: 'planner' | 'developer' | 'reviewer', provider: string) => {
    setConfig({
      ...config,
      [key]: {
        ...config[key],
        provider,
        model: MODELS[provider][0]
      }
    });
  };

  const ModelSelect = ({
    label,
    configKey
  }: {
    label: string,
    configKey: 'planner' | 'developer' | 'reviewer'
  }) => (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem', padding: '1rem', border: '1px solid var(--border-color)', borderRadius: '0.5rem', background: 'rgba(255, 255, 255, 0.05)' }}>
      <h3 style={{ margin: 0, fontSize: '1rem', color: 'var(--primary)' }}>{label}</h3>
      <div>
        <label className="label">Provider</label>
        <select
          value={config[configKey].provider}
          onChange={(e) => handleProviderChange(configKey, e.target.value)}
        >
          {Object.entries(PROVIDERS).map(([id, name]) => (
            <option key={id} value={id}>{name}</option>
          ))}
        </select>
      </div>
      <div>
        <label className="label">Model</label>
        <select
          value={config[configKey].model}
          onChange={(e) => setConfig({
            ...config,
            [configKey]: { ...config[configKey], model: e.target.value }
          })}
        >
          {MODELS[config[configKey].provider].map(model => (
            <option key={model} value={model}>{model}</option>
          ))}
        </select>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: '0.5rem' }}>
        <div>
          <label
            className="label"
            style={{ fontSize: '0.8rem', cursor: 'pointer', textDecoration: 'underline' }}
            onClick={() => fetchPreview('prompt', config[configKey].prompt_file)}
            title="Click to preview selected prompt"
          >
            Prompt
          </label>
          <select
            value={config[configKey].prompt_file || ""}
            onChange={(e) => setConfig({
              ...config,
              [configKey]: { ...config[configKey], prompt_file: e.target.value || undefined }
            })}
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
            onClick={() => fetchPreview('rule', config[configKey].rule_file)}
            title="Click to preview selected rule"
          >
            Rule
          </label>
          <select
            value={config[configKey].rule_file || ""}
            onChange={(e) => setConfig({
              ...config,
              [configKey]: { ...config[configKey], rule_file: e.target.value || undefined }
            })}
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
            onClick={() => fetchPreview('workflow', config[configKey].workflow_file)}
            title="Click to preview selected workflow"
          >
            Workflow
          </label>
          <select
            value={config[configKey].workflow_file || ""}
            onChange={(e) => setConfig({
              ...config,
              [configKey]: { ...config[configKey], workflow_file: e.target.value || undefined }
            })}
            style={{ fontSize: '0.85rem', padding: '0.4rem' }}
          >
            <option value="">None</option>
            {resources.workflows.map(f => <option key={f} value={f}>{f.split('/').pop()}</option>)}
          </select>
        </div>
      </div>
    </div>
  );

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

          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(300px, 1fr))', gap: '2rem' }}>
            <ModelSelect label="Planner" configKey="planner" />
            <ModelSelect label="Developer" configKey="developer" />
            <ModelSelect label="Reviewer" configKey="reviewer" />

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
                      <span className={`badge ${ev.source.toLowerCase()}`} style={{
                        background: ev.source === 'Planner' ? 'rgba(56, 189, 248, 0.2)' :
                          ev.source === 'Developer' ? 'rgba(168, 85, 247, 0.2)' :
                            'rgba(234, 179, 8, 0.2)',
                        color: ev.source === 'Planner' ? '#38bdf8' :
                          ev.source === 'Developer' ? '#a855f7' :
                            '#eab308',
                        padding: '0.2rem 0.5rem',
                        borderRadius: '0.25rem',
                        fontSize: '0.75rem',
                        fontWeight: 600
                      }}>
                        {ev.source}
                      </span>
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
      </main>
    </div>
  );
}

export default App;
