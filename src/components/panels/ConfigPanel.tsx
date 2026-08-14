import { open } from "@tauri-apps/plugin-dialog";
import { useState } from "react";
import { invoke } from "../../lib/tauri";
import { ModelSelect } from "./config/ModelSelect";
import WorkSessionSection from "./config/WorkSessionSection";
import type { AgentConfig, AgentResources, DetectedProcess, ModelConfig, RoleConfig, TeamMember, WorkSession } from "./config/types";
import { processTargetId } from "./config/types";
import HarnessReadinessPanel from "./harness/HarnessReadinessPanel";
import LiveTerminalsPanel from "./harness/LiveTerminalsPanel";

export type { AgentConfig, AgentResources, DetectedProcess, ModelConfig, RoleConfig, TeamMember, WorkSession } from "./config/types";

interface ConfigPanelProps {
  config: AgentConfig;
  setConfig: React.Dispatch<React.SetStateAction<AgentConfig>>;
  availableModels: Record<string, string[]>;
  resources: AgentResources;
  PROVIDERS: Record<string, string>;
  onPreview: (type: string, name?: string) => Promise<void>;
  teamMemberIds: string[];
  onAddAgent: (agent: TeamMember) => void;
  onRemoveAgent: (agent: TeamMember) => void;
  onCreateWorkSession: (name: string) => Promise<void>;
  workSessions?: WorkSession[];
  activeWorkSessionId?: string | null;
  onSelectWorkSession?: (sessionId: string | null) => void;
  onSwitchToChatView?: () => void;
  activeWorkSessionName: string | null;
}

export default function ConfigPanel({
  config,
  setConfig,
  availableModels,
  resources,
  PROVIDERS,
  onPreview,
  teamMemberIds,
  onAddAgent,
  onRemoveAgent,
  onCreateWorkSession,
  workSessions = [],
  activeWorkSessionId = null,
  onSelectWorkSession,
  onSwitchToChatView,
  activeWorkSessionName
}: ConfigPanelProps) {
  const [detectedProcesses, setDetectedProcesses] = useState<DetectedProcess[]>([]);
  const [detecting, setDetecting] = useState(false);
  const [detectError, setDetectError] = useState("");
  const [hasScanned, setHasScanned] = useState(false);
  const [addedPids, setAddedPids] = useState<number[]>([]);
  const [workSessionName, setWorkSessionName] = useState("");
  const [creatingWorkSession, setCreatingWorkSession] = useState(false);
  const [sessionError, setSessionError] = useState("");

  const createWorkSession = async () => {
    const trimmed = workSessionName.trim();
    if (!trimmed || creatingWorkSession) return;
    if (trimmed.length > 120) {
      setSessionError("Work session name must be between 1 and 120 characters.");
      return;
    }
    setCreatingWorkSession(true);
    setSessionError("");
    try {
      await onCreateWorkSession(trimmed);
      setWorkSessionName("");
      if (onSwitchToChatView) onSwitchToChatView();
    } catch (error) {
      const msg = error instanceof Error ? error.message : String(error);
      setSessionError(msg.replace(/^Error:\s*/, ""));
    } finally {
      setCreatingWorkSession(false);
    }
  };

  const loadWorkSession = (sessionId: string) => {
    if (!sessionId) return;
    setSessionError("");
    try {
      if (onSelectWorkSession) onSelectWorkSession(sessionId);
      if (onSwitchToChatView) onSwitchToChatView();
    } catch (error) {
      const msg = error instanceof Error ? error.message : String(error);
      setSessionError(msg.replace(/^Error:\s*/, ""));
    }
  };

  const detectProcesses = async () => {
    if (hasScanned) {
      setHasScanned(false);
      setDetectedProcesses([]);
      setDetectError("");
      return;
    }
    setDetecting(true);
    setDetectError("");
    try {
      setDetectedProcesses(await invoke<DetectedProcess[]>("detect_agent_processes"));
      setHasScanned(true);
    } catch (error) {
      setDetectError(String(error));
    } finally {
      setDetecting(false);
    }
  };

  const addDetectedProcess = (process: DetectedProcess) => {
    setConfig(prev => ({
      ...prev,
      roles: [...prev.roles, {
        name: `${process.agent} · PID ${process.pid}`,
        config: { provider: process.provider, model: process.model },
        origin: "existing",
        process_pid: process.pid
      }]
    }));
    setAddedPids(prev => [...prev, process.pid]);
    onAddAgent({
      id: `process:${process.pid}`,
      target_id: processTargetId(process),
      name: process.agent,
      provider: process.provider,
      model: process.model,
      origin: "existing"
    });
  };

  const removeDetectedProcess = (process: DetectedProcess) => {
    setConfig(prev => ({
      ...prev,
      roles: prev.roles.filter(role => role.process_pid !== process.pid)
    }));
    setAddedPids(prev => prev.filter(pid => pid !== process.pid));
    onRemoveAgent({
      id: `process:${process.pid}`,
      target_id: processTargetId(process),
      name: process.agent,
      provider: process.provider,
      model: process.model,
      origin: "existing"
    });
  };

  const spawnedRoleTeamMember = (index: number, role: RoleConfig): TeamMember => ({
    id: role.process_pid ? `process:${role.process_pid}` : `role:${index}`,
    target_id: role.process_pid ? processTargetId({ agent: role.name.split(" · ")[0], pid: role.process_pid }) : `role:${index}`,
    name: role.name,
    provider: role.config.provider,
    model: role.config.model,
    origin: role.origin || "spawned"
  });

  const addSpawnedRole = (index: number, role: RoleConfig) => {
    onAddAgent(spawnedRoleTeamMember(index, role));
  };

  const removeSpawnedRole = (index: number, role: RoleConfig) => {
    onRemoveAgent(spawnedRoleTeamMember(index, role));
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

  const updateRoleName = (index: number, name: string) => {
    setConfig(prev => {
      const newRoles = [...prev.roles];
      newRoles[index] = { ...newRoles[index], name };
      return { ...prev, roles: newRoles };
    });
  };

  const removeRole = (index: number) => {
    setConfig(prev => ({
      ...prev,
      roles: prev.roles.filter((_, i) => i !== index)
    }));
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

  return (
    <div className="glass-card fade-in" style={{ animationDelay: '0.1s' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', gap: '1rem', flexWrap: 'wrap', marginBottom: '1.25rem' }}>
        <h2 style={{ margin: 0 }}>Agent Team Configuration</h2>
        <button
          className={hasScanned ? "btn-primary" : "btn-secondary"}
          onClick={detectProcesses}
          disabled={detecting}
          style={hasScanned ? { background: 'rgba(168, 85, 247, 0.92)' } : undefined}
        >
          {detecting ? "Scanning this machine…" : hasScanned ? "Hide local processes" : "Detect local processes"}
        </button>
      </div>

      <section style={{ marginBottom: '1.5rem', padding: '1.25rem', border: '1px solid rgba(16, 185, 129, 0.32)', borderRadius: '12px', background: 'rgba(16, 185, 129, 0.06)' }}>
        <label className="label" style={{ fontWeight: 700, color: 'var(--text-primary)', marginBottom: '0.5rem', display: 'block' }}>Workspace Root</label>
        <div style={{ color: 'var(--text-muted)', fontSize: '0.82rem', marginBottom: '0.75rem' }}>All team sessions, harness capture, and task delivery use this absolute repository path.</div>
        <div style={{ display: 'flex', gap: '1rem', flexWrap: 'wrap' }}>
          <input
            style={{ flex: '1 1 420px', padding: '0.75rem', borderRadius: '8px', background: 'rgba(0,0,0,0.3)', color: 'white', border: '1px solid var(--border-color)', outline: 'none' }}
            placeholder="/absolute/path/to/workspace"
            value={config.work_dir}
            onChange={e => setConfig({ ...config, work_dir: e.target.value })}
          />
          <button
            className="btn-secondary"
            onClick={async () => {
              const selected = await open({ directory: true, multiple: false });
              if (selected) setConfig({ ...config, work_dir: selected as string });
            }}
          >
            Browse
          </button>
          <button
            className="btn-secondary"
            style={{ background: 'rgba(16, 185, 129, 0.1)', color: '#10b981', borderColor: 'rgba(16, 185, 129, 0.3)' }}
            onClick={async () => {
              try {
                await invoke("bootstrap_workspace", { workDir: config.work_dir });
                alert(`Successfully bootstrapped .agent/ in ${config.work_dir}`);
              } catch (err) {
                alert(`Failed to bootstrap: ${err}`);
              }
            }}
          >
            Initialize .agent/
          </button>
        </div>
      </section>

      <LiveTerminalsPanel workspace={config.work_dir} />
      <HarnessReadinessPanel workspace={config.work_dir} />

      <WorkSessionSection
        workSessions={workSessions}
        activeWorkSessionId={activeWorkSessionId}
        activeWorkSessionName={activeWorkSessionName}
        workSessionName={workSessionName}
        setWorkSessionName={setWorkSessionName}
        creatingWorkSession={creatingWorkSession}
        sessionError={sessionError}
        setSessionError={setSessionError}
        createWorkSession={() => void createWorkSession()}
        loadWorkSession={loadWorkSession}
        onSelectWorkSession={onSelectWorkSession}
      />

      {detectError && <div style={{ color: '#ef4444', fontSize: '0.85rem', marginBottom: '1rem' }}>{detectError}</div>}
      {hasScanned && <div style={{ display: 'flex', flexDirection: 'column', gap: '0.65rem', padding: '1rem', marginBottom: '1.5rem', border: '1px solid var(--border-color)', borderRadius: '10px', background: 'rgba(168, 85, 247, 0.06)' }}>
        <div style={{ color: 'var(--text-muted)', fontSize: '0.8rem' }}>External agent binaries currently running anywhere on this machine (not workspace-scoped, not a live Chat & Memory connection). Selecting one adds its identity to the team; it does not take ownership of or terminate the process. A task executes automatically only when that provider has a registered, supported active-session bridge.</div>
        {detectedProcesses.map(process => <div key={process.pid} style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '1rem', flexWrap: 'wrap', padding: '0.65rem 0.75rem', borderRadius: '8px', background: 'rgba(0,0,0,0.25)' }}>
          <div style={{ minWidth: 0 }}><strong style={{ color: 'var(--primary)' }}>{process.agent}</strong><span style={{ color: 'var(--text-muted)', marginLeft: '0.6rem', fontSize: '0.8rem' }}>PID {process.pid}</span><div style={{ color: 'var(--text-muted)', fontFamily: 'var(--font-mono)', fontSize: '0.75rem', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', maxWidth: 'min(65vw, 680px)' }}>{process.command}</div></div>
          <button
            className={addedPids.includes(process.pid) || teamMemberIds.includes(processTargetId(process)) ? "btn-secondary" : "btn-primary"}
            onClick={() => addedPids.includes(process.pid) || teamMemberIds.includes(processTargetId(process)) ? removeDetectedProcess(process) : addDetectedProcess(process)}
          >
            {addedPids.includes(process.pid) || teamMemberIds.includes(processTargetId(process)) ? "Remove from team" : "Add to team"}
          </button>
        </div>)}
        {detectedProcesses.length === 0 && <span style={{ color: 'var(--text-muted)', fontSize: '0.85rem' }}>No supported agent processes found.</span>}
      </div>}

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(320px, 1fr))', gap: '1.5rem' }}>
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
            onPreview={onPreview}
            resources={resources}
            PROVIDERS={PROVIDERS}
            onAddToTeam={() => addSpawnedRole(index, role)}
            onRemoveFromTeam={() => removeSpawnedRole(index, role)}
            isOnTeam={teamMemberIds.includes(spawnedRoleTeamMember(index, role).target_id)}
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
            borderRadius: '12px',
            background: 'rgba(255, 255, 255, 0.02)',
            cursor: 'pointer',
            transition: 'transform 0.2s ease, border-color 0.2s ease, background 0.2s ease',
            minHeight: '220px'
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.borderColor = 'var(--primary)';
            e.currentTarget.style.background = 'rgba(99, 102, 241, 0.05)';
            e.currentTarget.style.transform = 'scale(1.02)';
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.borderColor = 'var(--border-color)';
            e.currentTarget.style.background = 'rgba(255, 255, 255, 0.02)';
            e.currentTarget.style.transform = 'scale(1)';
          }}
        >
          <span style={{ fontSize: '2.5rem', color: 'var(--text-muted)', lineHeight: 1 }}>+</span>
          <span style={{ fontWeight: 600, color: 'var(--text-primary)' }}>Add New Role</span>
        </div>

        <div style={{ gridColumn: '1 / -1' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.75rem' }}>
            <label className="label" style={{ margin: 0, fontWeight: 600, color: 'var(--text-primary)' }}>MCP Configuration (JSON)</label>
            <button
              className="btn-secondary"
              style={{ fontSize: '0.85rem', padding: '0.4rem 0.8rem' }}
              onClick={async () => {
                try {
                  const selected = await open({ multiple: false });
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
              Load External Config...
            </button>
          </div>
          <textarea
            value={config.mcp_config}
            onChange={(e) => setConfig({ ...config, mcp_config: e.target.value })}
            placeholder="Paste mcp_servers.json content here..."
            style={{
              minHeight: '200px',
              fontFamily: 'var(--font-mono)',
              fontSize: '0.9rem',
              lineHeight: '1.5',
              backgroundColor: 'rgba(0, 0, 0, 0.4)',
              color: 'var(--text-primary)',
              border: '1px solid var(--border-color)',
              borderRadius: '8px',
              padding: '1rem',
              width: '100%',
              resize: 'vertical',
              outline: 'none',
              transition: 'border-color 0.2s'
            }}
            onFocus={e => e.target.style.borderColor = 'var(--primary)'}
            onBlur={e => e.target.style.borderColor = 'var(--border-color)'}
          />
        </div>
      </div>
    </div>
  );
}
