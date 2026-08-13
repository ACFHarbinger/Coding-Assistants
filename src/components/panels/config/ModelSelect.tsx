import { ModelConfig, RoleConfig, AgentResources } from "./types";

export const ModelSelect = ({
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
  onRemoveFromTeam,
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
  onRemoveFromTeam: () => void;
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
          onClick={isOnTeam ? onRemoveFromTeam : onAddToTeam}
          style={{ marginLeft: "0.5rem" }}
        >
          {isOnTeam ? "Remove from team" : "Add to team"}
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
