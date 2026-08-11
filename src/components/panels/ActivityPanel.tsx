
import { TeamMember } from "./ConfigPanel";

interface AgentEvent {
  source: string;
  event_type: string;
  content: string;
}

interface ActivityPanelProps {
  task: string;
  setTask: (t: string) => void;
  loading: boolean;
  startTask: () => Promise<void>;
  events: AgentEvent[];
  setEvents: (events: AgentEvent[]) => void;
  output: string;
  setOutput: (out: string) => void;
  teamMembers: TeamMember[];
}

export default function ActivityPanel({
  task,
  setTask,
  loading,
  startTask,
  events,
  setEvents,
  output,
  setOutput,
  teamMembers
}: ActivityPanelProps) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '2rem' }}>
      <div className="glass-card fade-in" style={{ animationDelay: '0.2s' }}>
        <h2>Write Message</h2>
        <textarea
          rows={5}
          placeholder="Write a message to the agent team…"
          value={task}
          onChange={e => setTask(e.target.value)}
          style={{
            width: '100%',
            background: 'rgba(0,0,0,0.3)',
            border: '1px solid var(--border-color)',
            borderRadius: '8px',
            padding: '1rem',
            color: 'var(--text-primary)',
            fontFamily: 'var(--font-sans)',
            fontSize: '1rem',
            resize: 'vertical',
            outline: 'none',
            transition: 'border-color 0.2s'
          }}
          onFocus={e => e.target.style.borderColor = 'var(--primary)'}
          onBlur={e => e.target.style.borderColor = 'var(--border-color)'}
        />
        <div style={{ marginTop: '1.5rem', display: 'flex', justifyContent: 'flex-end', alignItems: 'center', gap: '1.5rem' }}>
          {loading && (
            <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
              <div className="pulse-indicator" style={{ backgroundColor: 'var(--primary)' }} />
              <span style={{ color: 'var(--primary)', fontSize: '0.9rem', fontWeight: 500 }}>Sending message to the agent team...</span>
            </div>
          )}
          <button className={loading ? "btn-secondary" : "btn-primary"} onClick={startTask} disabled={!task && !loading}>
            {loading ? "Cancel Message" : "Send Message"}
          </button>
        </div>
      </div>

      <div className="glass-card fade-in" style={{ animationDelay: '0.25s' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1rem' }}>
          <h2 style={{ margin: 0 }}>Team Chat</h2>
          <span style={{ color: 'var(--text-muted)', fontSize: '0.8rem' }}>{teamMembers.length} participant{teamMembers.length === 1 ? "" : "s"}</span>
        </div>
        {teamMembers.length === 0 ? (
          <p style={{ color: 'var(--text-muted)', margin: 0 }}>Add an agent to the team to start a shared conversation.</p>
        ) : (
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: '0.5rem' }}>
            {teamMembers.map(member => (
              <span key={member.id} style={{ padding: '0.35rem 0.65rem', borderRadius: '999px', background: 'rgba(99, 102, 241, 0.14)', color: 'var(--primary)', fontSize: '0.8rem' }}>
                {member.name}
              </span>
            ))}
          </div>
        )}
      </div>

      {events.length > 0 && (
        <div className="glass-card fade-in" style={{ animationDelay: '0.3s' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1.5rem' }}>
            <h2>Messages</h2>
            <button
              onClick={() => setEvents([])}
              style={{ background: 'transparent', border: '1px solid var(--border-color)', borderRadius: '6px', padding: '0.4rem 0.8rem', color: 'var(--text-muted)', cursor: 'pointer', fontSize: '0.8rem', transition: 'border-color 0.15s ease, opacity 0.15s ease' }}
              onMouseEnter={e => { e.currentTarget.style.color = 'var(--text-main)'; e.currentTarget.style.background = 'rgba(255,255,255,0.05)'; }}
              onMouseLeave={e => { e.currentTarget.style.color = 'var(--text-muted)'; e.currentTarget.style.background = 'transparent'; }}
            >
              Clear Feed
            </button>
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: '1.25rem' }}>
            {events.map((ev, i) => (
              <div key={i} style={{ border: '1px solid var(--border-color)', borderRadius: '12px', overflow: 'hidden', background: ev.source === 'Harbinger' || ev.source === 'Remote' ? 'rgba(99, 102, 241, 0.12)' : 'rgba(0,0,0,0.2)', transition: 'transform 0.2s', boxShadow: '0 4px 6px rgba(0,0,0,0.1)' }}>
                <div style={{
                  padding: '0.75rem 1.25rem',
                  background: 'rgba(255, 255, 255, 0.04)',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  borderBottom: '1px solid var(--border-color)'
                }}>
                  <div style={{ display: 'flex', gap: '0.75rem', alignItems: 'center' }}>
                    {(() => {
                      const colors = {
                        Planner: { bg: 'rgba(56, 189, 248, 0.2)', text: '#38bdf8' },
                        Developer: { bg: 'rgba(168, 85, 247, 0.2)', text: '#a855f7' },
                        Reviewer: { bg: 'rgba(234, 179, 8, 0.2)', text: '#eab308' },
                        System: { bg: 'rgba(255, 255, 255, 0.1)', text: 'var(--text-muted)' },
                        User: { bg: 'rgba(34, 197, 94, 0.2)', text: '#22c55e' },
                      }[ev.source] || { bg: 'rgba(168, 85, 247, 0.15)', text: 'var(--primary)' };

                      return (
                        <span style={{
                          background: colors.bg,
                          color: colors.text,
                          padding: '0.25rem 0.6rem',
                          borderRadius: '6px',
                          fontSize: '0.75rem',
                          fontWeight: 700,
                          textTransform: 'uppercase',
                          letterSpacing: '0.05em'
                        }}>
                          {ev.source}
                        </span>
                      );
                    })()}
                    <span style={{ fontSize: '0.9rem', color: 'var(--text-muted)' }}>
                      {ev.event_type === "thought" ? "is thinking..." : ev.event_type === "team" ? "joined the conversation" : ev.event_type === "message" ? "sent a message" : "responded"}
                    </span>
                  </div>
                  <span style={{ fontSize: '0.75rem', color: 'var(--text-muted)', fontFamily: 'var(--font-mono)' }}>#{i + 1}</span>
                </div>
                <div style={{ padding: '1.25rem', maxHeight: '400px', overflowY: 'auto' }}>
                  <pre style={{
                    margin: 0,
                    whiteSpace: 'pre-wrap',
                    fontSize: '0.85rem',
                    fontFamily: 'var(--font-mono)',
                    color: 'var(--text-main)'
                  }}>
                    {ev.content}
                  </pre>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {output && (
        <div className="glass-card fade-in" style={{ borderLeft: '4px solid var(--primary)', animationDelay: '0.4s' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1.5rem' }}>
            <h2 style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', margin: 0 }}>
              <span style={{ color: 'var(--primary)' }}>✓</span> Final Output
            </h2>
            <button
              onClick={() => setOutput("")}
              style={{ background: 'transparent', border: '1px solid var(--border-color)', borderRadius: '6px', padding: '0.4rem 0.8rem', color: 'var(--text-muted)', cursor: 'pointer', fontSize: '0.8rem' }}
            >
              Clear Output
            </button>
          </div>
          <pre style={{
            whiteSpace: 'pre-wrap',
            background: 'rgba(0,0,0,0.3)',
            padding: '1.25rem',
            borderRadius: '8px',
            fontFamily: 'var(--font-mono)',
            fontSize: '0.9rem',
            lineHeight: 1.5,
            color: 'var(--text-main)',
            border: '1px solid var(--border-color)'
          }}>
            {output}
          </pre>
        </div>
      )}
    </div>
  );
}
