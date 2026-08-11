import { open } from "@tauri-apps/plugin-dialog";
import { useState } from "react";
import { invoke } from "../../lib/tauri";

export interface ModelConfig {
  provider: string;
  model: string;
  endpoint?: string;
  prompt_file?: string;
  rule_file?: string;
  workflow_file?: string;
}

export interface RoleConfig {
  name: string;
  config: ModelConfig;
  origin?: "spawned" | "existing";
  process_pid?: number;
}

export interface AgentConfig {
  roles: RoleConfig[];
  work_dir: string;
  mcp_config: string;
}

export interface AgentResources {
  prompts: string[];
  rules: string[];
  workflows: string[];
}

export interface TeamMember {
  id: string;
  name: string;
  provider: string;
  model: string;
  origin: "spawned" | "existing";
}

interface ConfigPanelProps {
  config: AgentConfig;
  setConfig: React.Dispatch<React.SetStateAction<AgentConfig>>;
  availableModels: Record<string, string[]>;
  resources: AgentResources;
  PROVIDERS: Record<string, string>;
  onPreview: (type: string, name?: string) => Promise<void>;
  teamMemberIds: string[];
  onAddAgent: (agent: TeamMember) => void;
}

export interface DetectedProcess {
  pid: number;
  agent: string;
  provider: string;
  model: string;
  command: string;
}

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
  PROVIDERS,
  onAddToTeam,
  isOnTeam
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
  onAddToTeam: () => void;
  isOnTeam: boolean;
}) => {
  const roleConfig = role.config;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '1.25rem', padding: '1.5rem', border: '1px solid var(--border-color)', borderRadius: '12px', background: 'rgba(255, 255, 255, 0.02)', position: 'relative', transition: 'transform 0.2s ease, box-shadow 0.2s ease, background 0.2s ease' }}
         onMouseEnter={(e) => {
           e.currentTarget.style.transform = 'translateY(-2px)';
           e.currentTarget.style.boxShadow = '0 10px 30px -10px rgba(0,0,0,0.5)';
           e.currentTarget.style.background = 'rgba(255,255,255,0.04)';
         }}
         onMouseLeave={(e) => {
           e.currentTarget.style.transform = 'translateY(0)';
           e.currentTarget.style.boxShadow = 'none';
           e.currentTarget.style.background = 'rgba(255,255,255,0.02)';
         }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <input
          value={role.name}
          onChange={(e) => onNameChange(index, e.target.value)}
          style={{
            background: 'transparent',
            border: 'none',
            borderBottom: '2px solid transparent',
            fontSize: '1.2rem',
            fontWeight: 700,
            color: 'var(--primary)',
            padding: '2px 0',
            outline: 'none',
            width: '100%',
            transition: 'border-color 0.2s ease'
          }}
          onFocus={(e) => e.target.style.borderBottom = '2px solid var(--primary)'}
          onBlur={(e) => e.target.style.borderBottom = '2px solid transparent'}
        />
        <button
          onClick={() => onRemove(index)}
          style={{
            padding: '0.35rem 0.75rem',
            fontSize: '0.8rem',
            background: 'rgba(239, 68, 68, 0.1)',
            color: '#ef4444',
            border: '1px solid rgba(239, 68, 68, 0.2)',
            borderRadius: '6px',
            cursor: 'pointer',
            transition: 'background 0.15s ease'
          }}
          onMouseEnter={e => e.currentTarget.style.background = 'rgba(239, 68, 68, 0.2)'}
          onMouseLeave={e => e.currentTarget.style.background = 'rgba(239, 68, 68, 0.1)'}
        >
          Remove
        </button>
        <button
          className={isOnTeam ? "btn-secondary" : "btn-primary"}
          onClick={onAddToTeam}
          disabled={isOnTeam}
          style={{ marginLeft: "0.5rem" }}
        >
          {isOnTeam ? "In team" : "Add to team"}
        </button>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
        <label className="label">Provider</label>
        <select
          value={roleConfig.provider}
          onChange={(e) => onProviderChange(index, e.target.value)}
          style={{ padding: '0.5rem', borderRadius: '6px', background: 'rgba(0,0,0,0.3)', color: 'white', border: '1px solid var(--border-color)', outline: 'none' }}
        >
          {Object.entries(PROVIDERS).map(([id, name]) => (
            <option key={id} value={id}>{name}</option>
          ))}
        </select>
      </div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
        <label className="label">Model</label>
        <select
          value={roleConfig.model}
          onChange={(e) => onConfigChange(index, 'model', e.target.value)}
          style={{ padding: '0.5rem', borderRadius: '6px', background: 'rgba(0,0,0,0.3)', color: 'white', border: '1px solid var(--border-color)', outline: 'none' }}
        >
          {(availableModels[roleConfig.provider] || []).map(model => (
            <option key={model} value={model}>{model}</option>
          ))}
        </select>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
        <label className="label">Existing process endpoint <span style={{ color: 'var(--text-muted)', fontWeight: 400 }}>(optional)</span></label>
        <input
          value={roleConfig.endpoint || ""}
          onChange={(e) => onConfigChange(index, 'endpoint', e.target.value)}
          placeholder="http://127.0.0.1:1234/v1"
          style={{ padding: '0.5rem', borderRadius: '6px', background: 'rgba(0,0,0,0.3)', color: 'white', border: '1px solid var(--border-color)', outline: 'none' }}
        />
        <span style={{ color: 'var(--text-muted)', fontSize: '0.75rem' }}>Uses an OpenAI-compatible service already running. Leave blank to let Coding Assistants start the provider.</span>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: '0.75rem', marginTop: '0.5rem' }}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: '0.25rem' }}>
          <label
            className="label"
            style={{ fontSize: '0.8rem', cursor: 'pointer', color: 'var(--text-muted)', transition: 'color 0.2s' }}
            onMouseEnter={e => e.currentTarget.style.color = 'var(--text-main)'}
            onMouseLeave={e => e.currentTarget.style.color = 'var(--text-muted)'}
            onClick={() => onPreview('prompt', roleConfig.prompt_file)}
            title="Click to preview selected prompt"
          >
            Prompt 🔍
          </label>
          <select
            value={roleConfig.prompt_file || ""}
            onChange={(e) => onConfigChange(index, 'prompt_file', e.target.value)}
            style={{ fontSize: '0.85rem', padding: '0.4rem', borderRadius: '6px', background: 'rgba(0,0,0,0.3)', color: 'white', border: '1px solid var(--border-color)', outline: 'none' }}
          >
            <option value="">Default</option>
            {resources.prompts.map(f => <option key={f} value={f}>{f.split('/').pop()}</option>)}
          </select>
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: '0.25rem' }}>
          <label
            className="label"
            style={{ fontSize: '0.8rem', cursor: 'pointer', color: 'var(--text-muted)', transition: 'color 0.2s' }}
            onMouseEnter={e => e.currentTarget.style.color = 'var(--text-main)'}
            onMouseLeave={e => e.currentTarget.style.color = 'var(--text-muted)'}
            onClick={() => onPreview('rule', roleConfig.rule_file)}
            title="Click to preview selected rule"
          >
            Rule 🔍
          </label>
          <select
            value={roleConfig.rule_file || ""}
            onChange={(e) => onConfigChange(index, 'rule_file', e.target.value)}
            style={{ fontSize: '0.85rem', padding: '0.4rem', borderRadius: '6px', background: 'rgba(0,0,0,0.3)', color: 'white', border: '1px solid var(--border-color)', outline: 'none' }}
          >
            <option value="">None</option>
            {resources.rules.map(f => <option key={f} value={f}>{f.split('/').pop()}</option>)}
          </select>
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: '0.25rem' }}>
          <label
            className="label"
            style={{ fontSize: '0.8rem', cursor: 'pointer', color: 'var(--text-muted)', transition: 'color 0.2s' }}
            onMouseEnter={e => e.currentTarget.style.color = 'var(--text-main)'}
            onMouseLeave={e => e.currentTarget.style.color = 'var(--text-muted)'}
            onClick={() => onPreview('workflow', roleConfig.workflow_file)}
            title="Click to preview selected workflow"
          >
            Workflow 🔍
          </label>
          <select
            value={roleConfig.workflow_file || ""}
            onChange={(e) => onConfigChange(index, 'workflow_file', e.target.value)}
            style={{ fontSize: '0.85rem', padding: '0.4rem', borderRadius: '6px', background: 'rgba(0,0,0,0.3)', color: 'white', border: '1px solid var(--border-color)', outline: 'none' }}
          >
            <option value="">None</option>
            {resources.workflows.map(f => <option key={f} value={f}>{f.split('/').pop()}</option>)}
          </select>
        </div>
      </div>
    </div>
  );
};

export default function ConfigPanel({ config, setConfig, availableModels, resources, PROVIDERS, onPreview, teamMemberIds, onAddAgent }: ConfigPanelProps) {
  const [detectedProcesses, setDetectedProcesses] = useState<DetectedProcess[]>([]);
  const [detecting, setDetecting] = useState(false);
  const [detectError, setDetectError] = useState("");
  const [hasScanned, setHasScanned] = useState(false);
  const [addedPids, setAddedPids] = useState<number[]>([]);

  const detectProcesses = async () => {
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
      name: process.agent,
      provider: process.provider,
      model: process.model,
      origin: "existing"
    });
  };

  const addSpawnedRole = (index: number, role: RoleConfig) => {
    onAddAgent({
      id: role.process_pid ? `process:${role.process_pid}` : `role:${index}`,
      name: role.name,
      provider: role.config.provider,
      model: role.config.model,
      origin: role.origin || "spawned"
    });
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
        <button className="btn-secondary" onClick={detectProcesses} disabled={detecting}>
          {detecting ? "Scanning processes…" : "Detect running agents"}
        </button>
      </div>

      {detectError && <div style={{ color: '#ef4444', fontSize: '0.85rem', marginBottom: '1rem' }}>{detectError}</div>}
      {hasScanned && <div style={{ display: 'flex', flexDirection: 'column', gap: '0.65rem', padding: '1rem', marginBottom: '1.5rem', border: '1px solid var(--border-color)', borderRadius: '10px', background: 'rgba(168, 85, 247, 0.06)' }}>
        <div style={{ color: 'var(--text-muted)', fontSize: '0.8rem' }}>Detected local agent processes. Selecting one adds its identity to the team; it does not take ownership of or terminate the process.</div>
        {detectedProcesses.map(process => <div key={process.pid} style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '1rem', flexWrap: 'wrap', padding: '0.65rem 0.75rem', borderRadius: '8px', background: 'rgba(0,0,0,0.25)' }}>
          <div style={{ minWidth: 0 }}><strong style={{ color: 'var(--primary)' }}>{process.agent}</strong><span style={{ color: 'var(--text-muted)', marginLeft: '0.6rem', fontSize: '0.8rem' }}>PID {process.pid}</span><div style={{ color: 'var(--text-muted)', fontFamily: 'var(--font-mono)', fontSize: '0.75rem', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', maxWidth: 'min(65vw, 680px)' }}>{process.command}</div></div>
          <button className={addedPids.includes(process.pid) ? "btn-secondary" : "btn-primary"} onClick={() => addDetectedProcess(process)} disabled={addedPids.includes(process.pid)}>{addedPids.includes(process.pid) ? "Added" : "Add to team"}</button>
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
            isOnTeam={teamMemberIds.includes(role.process_pid ? `process:${role.process_pid}` : `role:${index}`)}
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

        <div style={{ gridColumn: '1 / -1', marginTop: '1rem' }}>
          <label className="label" style={{ fontWeight: 600, color: 'var(--text-primary)', marginBottom: '0.5rem', display: 'block' }}>Workspace Root</label>
          <div style={{ display: 'flex', gap: '1rem' }}>
            <input
              style={{ flex: 1, padding: '0.75rem', borderRadius: '8px', background: 'rgba(0,0,0,0.3)', color: 'white', border: '1px solid var(--border-color)', outline: 'none' }}
              placeholder="./workspace"
              value={config.work_dir}
              onChange={e => setConfig({ ...config, work_dir: e.target.value })}
            />
            <button
              className="btn-secondary"
              onClick={async () => {
                const selected = await open({ directory: true, multiple: false });
                if (selected) {
                  setConfig({ ...config, work_dir: selected as string });
                }
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
