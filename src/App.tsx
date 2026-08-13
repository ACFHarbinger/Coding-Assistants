import { useState, useEffect, useRef } from "react";
import { invoke, isTauriRuntime } from "./lib/tauri";
import { listen } from "@tauri-apps/api/event";

import HubPanel from "./components/HubPanel";
import ConfigPanel, { AgentConfig, AgentResources, TeamMember } from "./components/panels/ConfigPanel";
import RemotePanel from "./components/panels/RemotePanel";
import SlackChatPanel from "./components/panels/SlackChatPanel";

const PROVIDERS = {
  "openai": "OpenAI",
  "anthropic": "Anthropic",
  "google": "Google",
  "xai": "Grok (xAI)",
  "ollama": "Ollama (Local)",
};

interface HubMessage {
  id: string;
  from_agent: string;
  to_agent: string;
  body: string;
  subject: string | null;
  kind: string;
  status: string;
  created_at: string;
}

interface HubAgent {
  id: string;
  display_name: string;
  team_member?: boolean;
}

export interface WorkSession {
  id: string;
  name: string;
  created_at: string;
  member_ids: string[];
}

function sameHubMessages(left: HubMessage[], right: HubMessage[]): boolean {
  if (left.length !== right.length) return false;
  return left.every((message, index) => {
    const other = right[index];
    return message.id === other.id
      && message.body === other.body
      && message.status === other.status
      && message.subject === other.subject;
  });
}

function sameHubAgents(left: HubAgent[], right: HubAgent[]): boolean {
  if (left.length !== right.length) return false;
  return left.every((agent, index) => {
    const other = right[index];
    return agent.id === other.id
      && agent.display_name === other.display_name
      && agent.team_member === other.team_member;
  });
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
  const [preview, setPreview] = useState<{ type: string, name: string, content: string } | null>(null);

  const [remoteStatus, setRemoteStatus] = useState<string>("Server not started");
  const [serverIP, setServerIP] = useState<string>("");
  const [remoteLogs, setRemoteLogs] = useState<string[]>([]);
  const [mainView, setMainView] = useState<"orchestrate" | "hub" | "slack">("slack");
  const [hubVisited, setHubVisited] = useState(false);
  const [availableModels, setAvailableModels] = useState<Record<string, string[]>>({});
  const [teamMembers, setTeamMembers] = useState<TeamMember[]>([]);
  const [hubMessages, setHubMessages] = useState<HubMessage[]>([]);
  const [hubAgents, setHubAgents] = useState<HubAgent[]>([]);
  const [workSessions, setWorkSessions] = useState<WorkSession[]>([]);
  const [activeWorkSessionId, setActiveWorkSessionId] = useState<string | null>(() => {
    try {
      return localStorage.getItem("ca.activeWorkSessionId");
    } catch {
      return null;
    }
  });
  const [chatFocusSessionId, setChatFocusSessionId] = useState<string | null>(() => {
    try {
      return localStorage.getItem("ca.activeWorkSessionId");
    } catch {
      return null;
    }
  });
  const [chatFocusToken, setChatFocusToken] = useState(0);
  const activeWorkSession = workSessions.find(session => session.id === activeWorkSessionId) ?? null;
  const workDirRef = useRef(config.work_dir);
  const sessionIdRef = useRef(activeWorkSessionId);
  workDirRef.current = config.work_dir;
  sessionIdRef.current = activeWorkSessionId;

  useEffect(() => {
    try {
      if (activeWorkSessionId) localStorage.setItem("ca.activeWorkSessionId", activeWorkSessionId);
      else localStorage.removeItem("ca.activeWorkSessionId");
    } catch {
      /* ignore quota / private mode */
    }
  }, [activeWorkSessionId]);

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

  const refreshHubChat = async () => {
    if (!isTauriRuntime()) return;
    try {
      const [messages, agents, sessions] = await Promise.all([
        invoke<HubMessage[]>("hub_list_messages", { to: null, status: null }),
        invoke<HubAgent[]>("hub_list_agents"),
        invoke<WorkSession[]>("hub_list_work_sessions"),
      ]);
      setHubMessages(prev => sameHubMessages(prev, messages) ? prev : messages);
      setHubAgents(prev => sameHubAgents(prev, agents) ? prev : agents);
      setWorkSessions(sessions);
      setActiveWorkSessionId(current =>
        current && sessions.some(session => session.id === current) ? current : current
      );
      const workspace = workDirRef.current;
      if (workspace) {
        const captures = await Promise.allSettled([
          invoke<{ captured?: unknown[] }>("hub_capture_grok_session", {
            workspace,
            grokSessionId: null,
            hubSessionId: sessionIdRef.current,
          }),
          invoke<{ captured?: unknown[] }>("hub_capture_claude_session", {
            workspace,
            claudeSessionId: null,
            hubSessionId: sessionIdRef.current,
          }),
          invoke<{ captured?: unknown[] }>("hub_capture_codex_session", {
            workspace,
            codexSessionId: null,
            hubSessionId: sessionIdRef.current,
          }),
          invoke<{ captured?: unknown[] }>("hub_capture_gemini_session", {
            workspace,
            geminiSessionId: null,
            hubSessionId: sessionIdRef.current,
          }),
        ]);
        const capturedNew = captures.some(result =>
          result.status === "fulfilled" && (result.value.captured?.length ?? 0) > 0
        );
        if (capturedNew) {
          const latest = await invoke<HubMessage[]>("hub_list_messages", { to: null, status: null });
          setHubMessages(prev => sameHubMessages(prev, latest) ? prev : latest);
        }
      }
    } catch (error) {
      console.error("Failed to refresh harness messages:", error);
    }
  };

  useEffect(() => {
    refreshHubChat();
    if (!isTauriRuntime()) return;
    const interval = window.setInterval(refreshHubChat, 1500);
    return () => window.clearInterval(interval);
  }, []);

  const addAgentToTeam = (agent: TeamMember) => {
    setTeamMembers(prev => {
      if (prev.some(member => member.id === agent.id)) return prev;
      return [...prev, agent];
    });
    const rosterId = agent.target_id;
    const persistable = rosterId === "chat"
      || rosterId === "claude"
      || rosterId === "gemini"
      || rosterId === "grok"
      || rosterId === "human";
    if (persistable && isTauriRuntime()) {
      void (async () => {
        try {
          await invoke("hub_set_team_member", { id: rosterId, enrolled: true });
          if (activeWorkSessionId) {
            await invoke("hub_add_work_session_member", {
              sessionId: activeWorkSessionId,
              agentId: rosterId,
            });
          }
          await refreshHubChat();
        } catch (error) {
          console.error("Failed to persist team/session enrollment:", error);
        }
      })();
    }
  };

  const selectWorkSession = (sessionId: string | null) => {
    setActiveWorkSessionId(sessionId);
    if (sessionId) {
      setChatFocusSessionId(sessionId);
      setChatFocusToken(token => token + 1);
    }
  };

  const createWorkSession = async (name: string) => {
    if (!isTauriRuntime()) throw new Error("Work sessions require the desktop app");
    const session = await invoke<WorkSession>("hub_create_work_session", { name });
    setWorkSessions(prev => [session, ...prev.filter(existing => existing.id !== session.id)]);
    selectWorkSession(session.id);
  };

  const removeAgentFromTeam = (agent: TeamMember) => {
    setTeamMembers(prev => prev.filter(member => member.id !== agent.id));
    const rosterId = agent.target_id;
    if (rosterId === "human") return;
    const persistable = rosterId === "chat"
      || rosterId === "claude"
      || rosterId === "gemini"
      || rosterId === "grok";
    if (persistable && isTauriRuntime()) {
      invoke("hub_set_team_member", { id: rosterId, enrolled: false }).catch(error => {
        console.error("Failed to persist team unenrollment:", error);
      });
    }
  };

  return (
    <div className="app-container" style={{ flexDirection: 'column' }}>
      <header style={{
        padding: '1.5rem 2.5rem',
        borderBottom: '1px solid var(--border-color)',
        background: 'rgba(2, 6, 23, 0.85)',
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
          <div title={config.work_dir || 'Workspace root is not set'} style={{ maxWidth: '260px', padding: '0.42rem 0.7rem', borderRadius: '8px', border: '1px solid rgba(16, 185, 129, 0.3)', background: 'rgba(16, 185, 129, 0.1)', color: '#a7f3d0', fontSize: '0.74rem', lineHeight: 1.25 }}>
            <strong style={{ display: 'block', color: '#6ee7b7' }}>Workspace root</strong>
            <span style={{ display: 'block', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{config.work_dir || 'Not set'}</span>
          </div>
          <div title={activeWorkSession?.name || 'No active team chat'} style={{ maxWidth: '210px', padding: '0.42rem 0.7rem', borderRadius: '8px', border: '1px solid rgba(6, 182, 212, 0.3)', background: 'rgba(6, 182, 212, 0.1)', color: '#cffafe', fontSize: '0.74rem', lineHeight: 1.25 }}>
            <strong style={{ display: 'block', color: '#67e8f9' }}>Active team chat</strong>
            <span style={{ display: 'block', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{activeWorkSession?.name || 'None selected'}</span>
          </div>
          <button
            className={mainView === "slack" ? "btn-primary" : "btn-secondary"}
            style={{ padding: '0.5rem 1rem', fontSize: '0.9rem', borderRadius: '8px' }}
            onClick={() => setMainView("slack")}
          >
            💬 Chat & Memory
          </button>
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
            onClick={() => { setHubVisited(true); setMainView("hub"); }}
          >
            Shared Hub
          </button>
          <div className="status-badge" style={{ marginLeft: '1rem', padding: '0.4rem 0.8rem', background: 'rgba(16, 185, 129, 0.15)', color: '#6ee7b7', borderRadius: '20px', fontSize: '0.75rem', fontWeight: 600, border: '1px solid rgba(16, 185, 129, 0.3)' }}>
            Local hub online
          </div>
        </div>
      </header>

      <main className="main-content">
        <div style={{ display: mainView === "slack" ? "contents" : "none" }}>
          <SlackChatPanel
            hubMessages={hubMessages}
            hubAgents={hubAgents}
            workSessions={workSessions}
            activeWorkSessionId={activeWorkSessionId}
            focusSessionId={chatFocusSessionId}
            focusSessionToken={chatFocusToken}
            workspacePath={config.work_dir}
            onSelectWorkSession={selectWorkSession}
            onRefresh={refreshHubChat}
          />
        </div>

        {(mainView === "hub" || hubVisited) && <div style={{ display: mainView === "hub" ? "contents" : "none" }}><HubPanel /></div>}

        <div style={{ display: mainView === "orchestrate" ? "contents" : "none" }}>
          <>
            <ConfigPanel
              config={config}
              setConfig={setConfig}
              availableModels={availableModels}
              resources={resources}
              PROVIDERS={PROVIDERS}
              onPreview={fetchPreview}
              teamMemberIds={teamMembers.map(member => member.id)}
              onAddAgent={addAgentToTeam}
              onRemoveAgent={removeAgentFromTeam}
              onCreateWorkSession={createWorkSession}
              workSessions={workSessions}
              activeWorkSessionId={activeWorkSessionId}
              onSelectWorkSession={selectWorkSession}
              onSwitchToChatView={() => setMainView("slack")}
              activeWorkSessionName={activeWorkSession?.name ?? null}
            />

            <RemotePanel
              remoteStatus={remoteStatus}
              serverIP={serverIP}
              startRemoteServer={startRemoteServer}
              stopRemoteServer={stopRemoteServer}
              remoteLogs={remoteLogs}
            />
          </>
        </div>

        {preview && (
          <div style={{
            position: 'fixed',
            top: 0, left: 0, right: 0, bottom: 0,
            background: 'rgba(2,6,23,0.92)',
            display: 'flex',
            justifyContent: 'center',
            alignItems: 'center',
            zIndex: 1000
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
      </main>
    </div>
  );
}

export default App;
