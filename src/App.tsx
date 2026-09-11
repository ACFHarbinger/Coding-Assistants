import { useState, useEffect, useRef } from "react";
import { invoke, isTauriRuntime } from "./lib/tauri";
import { listen } from "@tauri-apps/api/event";

import { PROVIDERS, HubAgent, HubMessage, HubRefreshOptions, WorkSession, loadWorkspaceRoot, sameHubAgents, sameHubMessages } from "./app/hubState";
import { addTeamMemberUnique, rosterAgentToTeamMember } from "./app/team";
import HubPanel from "./components/panels/HubPanel";
import ConfigPanel, { AgentConfig, AgentResources, TeamMember } from "./components/panels/ConfigPanel";
import RemotePanel from "./components/panels/RemotePanel";
import MessagerPanel from "./components/panels/MessagerPanel";
import { openSettingsWindow } from "./lib/settingsWindow";
import { defaultMcpConfig, loadPersistedRoles, savePersistedRoles } from "./app/rolesConfig";
import HarnessTerminalGrid from "./components/panels/terminal/HarnessTerminalGrid";

function App() {
  const [config, setConfig] = useState<AgentConfig>(() => {
    const ws = loadWorkspaceRoot();
    return {
      roles: loadPersistedRoles(),
      work_dir: ws,
      mcp_config: defaultMcpConfig(ws),
    };
  });

  const [resources, setResources] = useState<AgentResources>({ prompts: [], rules: [], workflows: [] });
  const [preview, setPreview] = useState<{ type: string, name: string, content: string } | null>(null);

  const [remoteStatus, setRemoteStatus] = useState<string>("Server not started");
  const [serverIP, setServerIP] = useState<string>("");
  const [remoteLogs, setRemoteLogs] = useState<string[]>([]);
  const [mainView, setMainView] = useState<"orchestrate" | "hub" | "messager">("messager");
  const [orchestrateSubView, setOrchestrateSubView] = useState<"setup" | "terminals">("setup");
  const [requestedTerminalHarness, setRequestedTerminalHarness] = useState<string | null>(null);
  const [hubVisited, setHubVisited] = useState(false);
  const [availableModels, setAvailableModels] = useState<Record<string, string[]>>({});
  const [teamMembers, setTeamMembers] = useState<TeamMember[]>([]);
  const [teamError, setTeamError] = useState<string | null>(null);
  const [hubMessages, setHubMessages] = useState<HubMessage[]>([]);
  const [hubAgents, setHubAgents] = useState<HubAgent[]>([]);
  const [workSessions, setWorkSessions] = useState<WorkSession[]>([]);
  const getSavedWorkSessionId = () => { try { return localStorage.getItem("ca.activeWorkSessionId"); } catch { return null; } };
  const [activeWorkSessionId, setActiveWorkSessionId] = useState<string | null>(getSavedWorkSessionId);
  const [chatFocusSessionId, setChatFocusSessionId] = useState<string | null>(getSavedWorkSessionId);
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
    try {
      if (config.work_dir) localStorage.setItem("ca.workspaceRoot", config.work_dir);
      else localStorage.removeItem("ca.workspaceRoot");
    } catch {
      /* ignore quota / private mode */
    }
  }, [config.work_dir]);

  useEffect(() => {
    savePersistedRoles(config.roles);
  }, [config.roles]);

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

  const refreshHubChat = async (options?: HubRefreshOptions) => {
    if (!isTauriRuntime()) return;
    try {
      const [messages, agents, sessions] = await Promise.all([
        invoke<HubMessage[]>("hub_list_messages", { to: null, status: null }),
        invoke<HubAgent[]>("hub_list_agents"),
        invoke<WorkSession[]>("hub_list_work_sessions"),
      ]);
      setHubMessages(prev => sameHubMessages(prev, messages) ? prev : messages);
      setHubAgents(prev => sameHubAgents(prev, agents) ? prev : agents);
      setTeamMembers(previous => {
        const persisted = agents
          .filter(agent => agent.team_member)
          .map(rosterAgentToTeamMember);
        const persistedIds = new Set(persisted.map(agent => agent.target_id));
        const transient = previous.filter(agent =>
          !persistedIds.has(agent.target_id)
          && (agent.target_id.startsWith("role:") || agent.target_id.startsWith("process:"))
        );
        const next = [...persisted, ...transient];
        const unchanged = next.length === previous.length
          && next.every((agent, index) => agent.id === previous[index]?.id && agent.target_id === previous[index]?.target_id);
        return unchanged ? previous : next;
      });
      setWorkSessions(sessions);
      setActiveWorkSessionId(current =>
        current && sessions.some(session => session.id === current) ? current : current
      );
      const workspace = workDirRef.current;
      if (workspace && options?.includeCapture !== false) {
        const harnessCaptureCmds = [
          ["hub_capture_grok_session", "grokSessionId"],
          ["hub_capture_claude_session", "claudeSessionId"],
          ["hub_capture_codex_session", "codexSessionId"],
          ["hub_capture_gemini_session", "geminiSessionId"],
          ["hub_capture_cursor_session", "cursorSessionId"],
          ["hub_capture_muse_session", "museSessionId"],
          ["hub_capture_qwen_session", "qwenSessionId"],
          ["hub_capture_vibe_session", "vibeSessionId"],
          ["hub_capture_kimi_session", "kimiSessionId"],
        ] as const;
        const captures = await Promise.allSettled(
          harnessCaptureCmds.map(([cmd, paramKey]) =>
            invoke<{ captured?: unknown[] }>(cmd, {
              workspace,
              [paramKey]: null,
              hubSessionId: sessionIdRef.current,
            })
          )
        );
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
    let unlisten: (() => void) | undefined;
    listen("hub:agents-changed", () => void refreshHubChat()).then((u) => {
      unlisten = u;
    });
    return () => {
      window.clearInterval(interval);
      unlisten?.();
    };
  }, []);

  const addAgentToTeam = async (agent: TeamMember): Promise<void> => {
    setTeamError(null);
    const rosterId = agent.target_id;
    // No allowlist: any roster identity persists the same way, with
    // `hub_set_team_member` as the source of truth (U22 / #225). A backend
    // NotFound (unknown id) surfaces in the banner below instead of console.
    if (!isTauriRuntime()) {
      setTeamMembers(prev => addTeamMemberUnique(prev, agent));
      return;
    }
    try {
      await invoke("hub_set_team_member", { id: rosterId, enrolled: true });
      if (activeWorkSessionId) {
        await invoke("hub_add_work_session_member", {
          sessionId: activeWorkSessionId,
          agentId: rosterId,
        });
      }
      setTeamMembers(prev => addTeamMemberUnique(prev, agent));
      await refreshHubChat();
    } catch (error) {
      setTeamError(`Could not enroll ${rosterId} on the persisted team: ${error}`);
      throw error;
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
    const trimmed = name.trim();
    if (!trimmed || trimmed.length > 120) {
      throw new Error("Work session name must be between 1 and 120 characters");
    }
    const session = await invoke<WorkSession>("hub_create_work_session", { name: trimmed });
    setWorkSessions(prev => [session, ...prev.filter(existing => existing.id !== session.id)]);
    selectWorkSession(session.id);
  };

  const removeAgentFromTeam = async (agent: TeamMember): Promise<void> => {
    setTeamError(null);
    const rosterId = agent.target_id;
    if (rosterId === "human") return;
    // No allowlist (U22 / #225): unenroll persists for every other identity.
    if (!isTauriRuntime()) {
      setTeamMembers(prev => prev.filter(member => member.id !== agent.id));
      return;
    }
    try {
      await invoke("hub_set_team_member", { id: rosterId, enrolled: false });
      setTeamMembers(prev => prev.filter(member => member.id !== agent.id));
      await refreshHubChat();
    } catch (error) {
      setTeamError(`Could not remove ${rosterId} from the persisted team: ${error}`);
      throw error;
    }
  };

  const teamMemberIds = [...new Set([
    ...teamMembers.flatMap(member => [member.id, member.target_id]),
    ...hubAgents.filter(agent => agent.team_member).map(agent => agent.id),
  ])];

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
          <div
            title={config.work_dir || 'Workspace root is not set (Click to configure)'}
            onClick={() => setMainView("orchestrate")}
            style={{ maxWidth: '260px', padding: '0.42rem 0.7rem', borderRadius: '8px', border: '1px solid rgba(16, 185, 129, 0.3)', background: 'rgba(16, 185, 129, 0.1)', color: '#a7f3d0', fontSize: '0.74rem', lineHeight: 1.25, cursor: 'pointer' }}
          >
            <strong style={{ display: 'block', color: '#6ee7b7' }}>Workspace root</strong>
            <span style={{ display: 'block', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{config.work_dir || 'Not set'}</span>
          </div>
          <div
            title={activeWorkSession?.name ? `Active: ${activeWorkSession.name} (Click to open Chat)` : 'No active team chat (Click to open Chat)'}
            onClick={() => setMainView("messager")}
            style={{ maxWidth: '210px', padding: '0.42rem 0.7rem', borderRadius: '8px', border: '1px solid rgba(6, 182, 212, 0.3)', background: 'rgba(6, 182, 212, 0.1)', color: '#cffafe', fontSize: '0.74rem', lineHeight: 1.25, cursor: 'pointer' }}
          >
            <strong style={{ display: 'block', color: '#67e8f9' }}>Active team chat</strong>
            <span style={{ display: 'block', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{activeWorkSession?.name || 'None selected'}</span>
          </div>
          <button
            className={mainView === "messager" ? "btn-primary" : "btn-secondary"}
            style={{ padding: '0.5rem 1rem', fontSize: '0.9rem', borderRadius: '8px' }}
            onClick={() => setMainView("messager")}
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
          <button
            type="button"
            className="btn-secondary"
            style={{ marginTop: 0, padding: "0.5rem 0.9rem", fontSize: "0.9rem", borderRadius: "8px", display: "inline-flex", alignItems: "center", gap: "0.45rem" }}
            onClick={() => { void openSettingsWindow(); }}
            aria-haspopup="dialog"
            title="Open Settings in its own window"
          >
            <svg aria-hidden="true" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <circle cx="12" cy="12" r="3" />
              <path d="M19.4 15a1.7 1.7 0 0 0 .34 1.88l.06.06-2.12 2.12-.06-.06a1.7 1.7 0 0 0-1.88-.34 1.7 1.7 0 0 0-1.04 1.56V20.3h-3v-.08A1.7 1.7 0 0 0 10.66 18.7a1.7 1.7 0 0 0-1.88.34l-.06.06-2.12-2.12.06-.06A1.7 1.7 0 0 0 7 15.04a1.7 1.7 0 0 0-1.56-1.04h-.08v-3h.08A1.7 1.7 0 0 0 7 9.96a1.7 1.7 0 0 0-.34-1.88L6.6 8.02 8.72 5.9l.06.06a1.7 1.7 0 0 0 1.88.34A1.7 1.7 0 0 0 11.7 4.74v-.08h3v.08a1.7 1.7 0 0 0 1.04 1.56 1.7 1.7 0 0 0 1.88-.34l.06-.06 2.12 2.12-.06.06a1.7 1.7 0 0 0-.34 1.88A1.7 1.7 0 0 0 20.96 11h.08v3h-.08A1.7 1.7 0 0 0 19.4 15Z" />
            </svg>
            Settings
          </button>
          <div className="status-badge" style={{ marginLeft: '1rem', padding: '0.4rem 0.8rem', background: 'rgba(16, 185, 129, 0.15)', color: '#6ee7b7', borderRadius: '20px', fontSize: '0.75rem', fontWeight: 600, border: '1px solid rgba(16, 185, 129, 0.3)' }}>
            Local hub online
          </div>
        </div>
      </header>

      <main className="main-content">
        {teamError && (
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "1rem", padding: "0.6rem 0.9rem", borderRadius: "8px", background: "rgba(239, 68, 68, 0.12)", border: "1px solid rgba(248, 113, 113, 0.45)", color: "#fca5a5", fontSize: "0.82rem", marginBottom: "1rem" }}>
            <span>{teamError}</span>
            <button type="button" className="btn-secondary" style={{ marginTop: 0, padding: "0.25rem 0.7rem", fontSize: "0.75rem" }} onClick={() => setTeamError(null)}>
              Dismiss
            </button>
          </div>
        )}
        <div style={{ display: mainView === "messager" ? "contents" : "none" }}>
          <MessagerPanel
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

        {(mainView === "hub" || hubVisited) && <div style={{ display: mainView === "hub" ? "contents" : "none" }}><HubPanel teamMemberIds={teamMemberIds} onAddAgent={addAgentToTeam} onRemoveAgent={removeAgentFromTeam} /></div>}

        <div style={{ display: mainView === "orchestrate" ? "contents" : "none" }}>
          <div style={{ display: "flex", gap: "0.5rem", background: "rgba(0,0,0,0.2)", padding: "0.25rem", borderRadius: "10px", width: "fit-content", marginBottom: "1rem" }}>
            <button type="button" className={orchestrateSubView === "setup" ? "btn-primary" : "btn-secondary"} style={{ marginTop: 0, padding: "0.5rem 1rem", fontSize: "0.9rem", borderRadius: "8px", transition: "opacity 0.15s ease, transform 0.15s ease" }} onClick={() => setOrchestrateSubView("setup")}>
              ⚙️ Setup &amp; Config
            </button>
            <button type="button" className={orchestrateSubView === "terminals" ? "btn-primary" : "btn-secondary"} style={{ marginTop: 0, padding: "0.5rem 1rem", fontSize: "0.9rem", borderRadius: "8px", transition: "opacity 0.15s ease, transform 0.15s ease" }} onClick={() => setOrchestrateSubView("terminals")}>
              🖥️ Terminal Grid
            </button>
          </div>

          <div style={{ display: orchestrateSubView === "terminals" ? "block" : "none" }}>
            <HarnessTerminalGrid
              workspace={config.work_dir}
              onOpenSetup={() => setOrchestrateSubView("setup")}
              requestedHarness={requestedTerminalHarness}
              onHarnessRequestHandled={() => setRequestedTerminalHarness(null)}
            />
          </div>

          <div style={{ display: orchestrateSubView === "setup" ? "contents" : "none" }}>
            <ConfigPanel
              config={config}
              setConfig={setConfig}
              availableModels={availableModels}
              resources={resources}
              PROVIDERS={PROVIDERS}
              onPreview={fetchPreview}
              teamMemberIds={teamMemberIds}
              onAddAgent={addAgentToTeam}
              onRemoveAgent={removeAgentFromTeam}
              onCreateWorkSession={createWorkSession}
              workSessions={workSessions}
              activeWorkSessionId={activeWorkSessionId}
              onSelectWorkSession={selectWorkSession}
              onSwitchToChatView={() => setMainView("messager")}
              activeWorkSessionName={activeWorkSession?.name ?? null}
              onOpenTerminalGrid={(harness) => {
                if (harness) setRequestedTerminalHarness(harness);
                setOrchestrateSubView("terminals");
              }}
            />

            <RemotePanel
              remoteStatus={remoteStatus}
              serverIP={serverIP}
              startRemoteServer={startRemoteServer}
              stopRemoteServer={stopRemoteServer}
              remoteLogs={remoteLogs}
            />
          </div>
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
