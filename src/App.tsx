import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ModelConfig {
  provider: string;
  model: string;
  api_key?: string;
  base_url?: string;
}

interface AgentConfig {
  planner: ModelConfig;
  developer: ModelConfig;
  reviewer: ModelConfig;
  work_dir: string;
}

function App() {
  const [config, setConfig] = useState<AgentConfig>({
    planner: { provider: "OpenAI", model: "gpt-4o" },
    developer: { provider: "Ollama", model: "llama3.1", base_url: "http://localhost:11434/v1" },
    reviewer: { provider: "OpenAI", model: "gpt-4o" },
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
          Antigravity
        </h1>
        <div className="status-badge">System Active</div>
      </header>

      <main className="main-content">
        <div className="glass-card">
          <h2>Configuration</h2>

          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(300px, 1fr))', gap: '2rem' }}>
            <div>
              <label className="label">Planner (Architect)</label>
              <input
                placeholder="e.g. gpt-4o"
                value={config.planner.model}
                onChange={e => setConfig({ ...config, planner: { ...config.planner, model: e.target.value } })}
              />

              <label className="label" style={{ marginTop: '1.5rem' }}>Planner API Key</label>
              <input
                type="password"
                placeholder="sk-..."
                value={config.planner.api_key}
                onChange={e => setConfig({ ...config, planner: { ...config.planner, api_key: e.target.value } })}
              />
            </div>

            <div>
              <label className="label">Developer (Worker)</label>
              <input
                placeholder="e.g. llama3.1"
                value={config.developer.model}
                onChange={e => setConfig({ ...config, developer: { ...config.developer, model: e.target.value } })}
              />

              <label className="label" style={{ marginTop: '1.5rem' }}>Workspace Root</label>
              <input
                placeholder="./workspace"
                value={config.work_dir}
                onChange={e => setConfig({ ...config, work_dir: e.target.value })}
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
            {loading && <span style={{ color: 'var(--text-muted)', fontSize: '0.9rem', fontStyle: 'italic' }}>Orchestrating agents...</span>}
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
            <pre>
              {output}
            </pre>
          </div>
        )}
      </main>
    </div>
  );
}

export default App;
