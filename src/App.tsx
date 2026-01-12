import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

interface ModelConfig {
  provider: string;
  model: string;
}

interface AgentConfig {
  planner: ModelConfig;
  developer: ModelConfig;
  reviewer: ModelConfig;
  work_dir: string;
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
  const [task, setTask] = useState("");
  const [output, setOutput] = useState("");
  const [loading, setLoading] = useState(false);

  const startTask = async () => {
    setLoading(true);
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
    <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
      <div>
        <label className="label">{label} Provider</label>
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
        <label className="label">{label} Model</label>
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
            <button className="btn-primary" onClick={startTask} disabled={loading || !task}>
              {loading ? "Running..." : "Launch Sequence"}
            </button>
          </div>
        </div>

        {output && (
          <div className="glass-card" style={{ borderLeft: '4px solid var(--primary)' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1rem' }}>
              <h2>Console Output</h2>
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
      </main>
    </div>
  );
}

export default App;
