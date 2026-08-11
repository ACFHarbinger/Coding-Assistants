import { useState, useEffect } from "react";
import { invoke, isTauriRuntime } from "./lib/tauri";
import { listen } from "@tauri-apps/api/event";

import HubPanel from "./components/HubPanel";
import ConfigPanel, { AgentConfig, AgentResources } from "./components/panels/ConfigPanel";
import ActivityPanel from "./components/panels/ActivityPanel";
import RemotePanel from "./components/panels/RemotePanel";
import ApprovalPanel from "./components/panels/ApprovalPanel";

const PROVIDERS = {
  "openai": "OpenAI",
  "anthropic": "Anthropic",
  "google": "Google",
  "ollama": "Ollama (Local)",
};

interface AgentEvent {
  source: string;
  event_type: string;
  content: string;
}

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
  const [mainView, setMainView] = useState<"orchestrate" | "hub">("orchestrate");
  const [availableModels, setAvailableModels] = useState<Record<string, string[]>>({});

  useEffect(() => {
    async function loadModels() {
      try {
        const models = await invoke<Record<string, string[]>>("get_available_models");
        setAvailableModels(models);
      } catch (err) {
        console.error("Failed to load models:", err);
      }
    }
    loadModels();
  }, []);

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
    if (!isTauriRuntime()) return;
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
    try {
      const content = await invoke<string>("get_resource_content", { workDir: config.work_dir, path: name });
      setPreview({ type, name, content });
    } catch (e) {
      alert(`Failed to load preview: ${e}`);
    }
  };

  useEffect(() => {
    async function fetchResources() {
      if (!config.work_dir) return;
      try {
        const res = await invoke<AgentResources>("get_agent_resources", { workDir: config.work_dir });
        setResources(res);
      } catch (e) {
        console.error("Failed to fetch resources:", e);
      }
    }
    fetchResources();
  }, [config.work_dir]);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    const unlisten = listen<AgentEvent>("agent-event", (event) => {
      setEvents((prev) => {
        const last = prev[prev.length - 1];
        if (event.payload.event_type === "stream") {
          if (last && last.source === event.payload.source && last.event_type === "response") {
            const newLast = { ...last, content: last.content + event.payload.content };
            return [...prev.slice(0, -1), newLast];
          }
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
        return [...prev, event.payload];
      });
    });
    return () => {
      unlisten.then(f => f());
    };
  }, []);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    const unlistenTask = listen<{ config: AgentConfig, task: string }>("android-task-request", (event) => {
      setConfig(event.payload.config);
      setTask(event.payload.task);
      startTaskRemote(event.payload.config, event.payload.task);
    });

    const unlistenCancel = listen("android-cancel-request", () => {
      invoke("cancel_task").catch(err => console.error("Remote cancel failed:", err));
    });

    const unlistenInput = listen<string>("android-input-submit", (event) => {
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

  const submitAnswer = async () => {
    if (!userInput.trim()) return;
    try {
      await invoke("submit_user_input", { input: userInput });
      setCurrentQuestion(null);
      setUserInput("");
    } catch (e) {
      alert("Failed to submit answer: " + e);
    }
  };

  const respondToAuthorization = async (approved: boolean) => {
    try {
      await invoke("submit_user_input", { input: approved ? "APPROVED" : "DENIED" });
      setAuthorizationRequest(null);
    } catch (e) {
      alert("Failed to submit response: " + e);
    }
  };

  const startTask = async () => {
    if (loading) {
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

  return (
    <div className="app-container" style={{ flexDirection: 'column' }}>
      <header style={{
        padding: '1.5rem 2.5rem',
        borderBottom: '1px solid var(--border-color)',
        background: 'rgba(2, 6, 23, 0.5)',
        backdropFilter: 'var(--glass-blur)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        zIndex: 10
      }}>
        <h1 style={{ fontSize: '1.5rem', fontWeight: 800, letterSpacing: '-0.025em', background: 'linear-gradient(to right, #fff, var(--text-muted))', WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent' }}>
          Coding Assistants
        </h1>
        <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
          <button
            className={mainView === "orchestrate" ? "btn-primary" : "btn-secondary"}
            style={{ padding: '0.5rem 1rem', fontSize: '0.9rem', borderRadius: '8px' }}
            onClick={() => setMainView("orchestrate")}
          >
            Orchestrate
          </button>
          <button
            className={mainView === "hub" ? "btn-primary" : "btn-secondary"}
            style={{ padding: '0.5rem 1rem', fontSize: '0.9rem', borderRadius: '8px' }}
            onClick={() => setMainView("hub")}
          >
            Shared Hub
          </button>
          <div className="status-badge" style={{ marginLeft: '1rem', padding: '0.4rem 0.8rem', background: 'rgba(168, 85, 247, 0.2)', color: 'var(--accent)', borderRadius: '20px', fontSize: '0.75rem', fontWeight: 600, border: '1px solid rgba(168, 85, 247, 0.3)' }}>
            Powered by OpenCode
          </div>
        </div>
      </header>

      <main className="main-content">
        {mainView === "hub" && <HubPanel />}

        {mainView === "orchestrate" && (
          <>
            <ConfigPanel 
              config={config} 
              setConfig={setConfig} 
              availableModels={availableModels} 
              resources={resources} 
              PROVIDERS={PROVIDERS} 
              onPreview={fetchPreview} 
            />
            
            <ActivityPanel 
              task={task} 
              setTask={setTask} 
              loading={loading} 
              startTask={startTask} 
              events={events} 
              setEvents={setEvents} 
              output={output} 
              setOutput={setOutput} 
            />

            <RemotePanel 
              remoteStatus={remoteStatus} 
              serverIP={serverIP} 
              startRemoteServer={startRemoteServer} 
              stopRemoteServer={stopRemoteServer} 
              remoteLogs={remoteLogs} 
            />
          </>
        )}

        {preview && (
          <div style={{
            position: 'fixed',
            top: 0, left: 0, right: 0, bottom: 0,
            background: 'rgba(2,6,23,0.85)',
            display: 'flex',
            justifyContent: 'center',
            alignItems: 'center',
            zIndex: 1000,
            backdropFilter: 'blur(8px)'
          }} onClick={() => setPreview(null)}>
            <div
              className="fade-in"
              style={{
                background: 'var(--bg-card)',
                border: '1px solid var(--border-color)',
                borderRadius: '16px',
                padding: '2.5rem',
                maxWidth: '850px',
                width: '90%',
                maxHeight: '85vh',
                overflow: 'auto',
                boxShadow: '0 25px 50px -12px rgba(0, 0, 0, 0.5)'
              }}
              onClick={e => e.stopPropagation()}
            >
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1.5rem' }}>
                <h2 style={{ textTransform: 'capitalize', color: 'var(--text-main)', margin: 0, fontWeight: 700 }}>
                  {preview.type} Preview: <span style={{ color: 'var(--primary)' }}>{preview.name}</span>
                </h2>
                <button onClick={() => setPreview(null)} className="btn-secondary" style={{ padding: '0.5rem 1rem' }}>Close</button>
              </div>
              <pre style={{ 
                whiteSpace: 'pre-wrap', 
                background: 'rgba(0,0,0,0.4)', 
                padding: '1.5rem', 
                borderRadius: '12px',
                fontFamily: 'var(--font-mono)',
                fontSize: '0.9rem',
                color: 'var(--text-main)',
                border: '1px solid var(--border-color)',
                lineHeight: 1.5
              }}>
                {preview.content}
              </pre>
            </div>
          </div>
        )}

        <ApprovalPanel 
          authorizationRequest={authorizationRequest}
          respondToAuthorization={respondToAuthorization}
          currentQuestion={currentQuestion}
          userInput={userInput}
          setUserInput={setUserInput}
          submitAnswer={submitAnswer}
        />
      </main>
    </div>
  );
}

export default App;
