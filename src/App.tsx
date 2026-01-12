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
    <div className="app-container">
      <div className="sidebar">
        <h1 style={{ fontSize: '1.25rem', marginBottom: '2rem' }}>Antigravity Assistant</h1>
        <div className="status-badge">Tauri V2 + React</div>
      </div>

      <div className="main-content">
        <div className="glass-card">
          <h2>Configuration</h2>

          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1rem' }}>
            <div>
              <label className="label">Planner Model</label>
              <input value={config.planner.model} onChange={e => setConfig({ ...config, planner: { ...config.planner, model: e.target.value } })} />

              <label className="label">OpenAI API Key</label>
              <input type="password" value={config.planner.api_key} onChange={e => setConfig({ ...config, planner: { ...config.planner, api_key: e.target.value } })} />
            </div>

            <div>
              <label className="label">Developer Model</label>
              <input value={config.developer.model} onChange={e => setConfig({ ...config, developer: { ...config.developer, model: e.target.value } })} />

              <label className="label">Work Directory</label>
              <input value={config.work_dir} onChange={e => setConfig({ ...config, work_dir: e.target.value })} />
            </div>
          </div>
        </div>

        <div className="glass-card">
          <h2>New Task</h2>
          <textarea
            rows={4}
            placeholder="Describe the task for the agent team..."
            value={task}
            onChange={e => setTask(e.target.value)}
          />
          <div style={{ marginTop: '1rem', display: 'flex', justifyContent: 'flex-end' }}>
            <button className="btn-primary" onClick={startTask} disabled={loading}>
              {loading ? "Running Agent Team..." : "Start Sequence"}
            </button>
          </div>
        </div>

        {output && (
          <div className="glass-card">
            <h2>Output</h2>
            <pre style={{ whiteSpace: 'pre-wrap', fontSize: '0.875rem', marginTop: '1rem', color: '#cbd5e1' }}>
              {output}
            </pre>
          </div>
        )}
      </div>
    </div>
  );
}

export default App;
