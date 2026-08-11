
interface RemotePanelProps {
  remoteStatus: string;
  serverIP: string;
  startRemoteServer: () => Promise<void>;
  stopRemoteServer: () => Promise<void>;
  remoteLogs: string[];
}

export default function RemotePanel({
  remoteStatus,
  serverIP,
  startRemoteServer,
  stopRemoteServer,
  remoteLogs
}: RemotePanelProps) {
  const isListening = remoteStatus.includes("listening");

  return (
    <div className="glass-card fade-in" style={{ animationDelay: '0.5s' }}>
      <h2>Remote Control</h2>
      
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1.5rem', background: 'rgba(0,0,0,0.2)', padding: '1.5rem', borderRadius: '12px', border: '1px solid var(--border-color)' }}>
        <div>
          <p style={{ margin: 0, fontWeight: 600, fontSize: '1.1rem', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            <span style={{ 
              display: 'inline-block',
              width: '10px',
              height: '10px',
              borderRadius: '50%',
              backgroundColor: isListening ? '#22c55e' : 'var(--text-muted)',
              boxShadow: isListening ? '0 0 10px #22c55e' : 'none'
            }} />
            Status: <span style={{ color: isListening ? '#22c55e' : 'var(--text-muted)' }}>{remoteStatus}</span>
          </p>
          <p style={{ margin: '0.5rem 0 0 0', fontSize: '0.85rem', color: 'var(--text-muted)' }}>
            Control from Android app via WiFi
          </p>
          {serverIP && (
            <div style={{ marginTop: '1rem', background: 'rgba(168, 85, 247, 0.1)', padding: '0.75rem 1rem', borderRadius: '8px', border: '1px solid rgba(168, 85, 247, 0.2)' }}>
              <p style={{ margin: 0, fontSize: '0.85rem', color: 'var(--text-muted)' }}>Connect your mobile app to:</p>
              <p style={{ margin: '0.25rem 0 0 0', fontSize: '1.1rem', fontWeight: 600, color: 'var(--primary)', fontFamily: 'var(--font-mono)', letterSpacing: '1px' }}>
                {serverIP}
              </p>
            </div>
          )}
        </div>
        
        <button
          className={isListening ? "btn-secondary" : "btn-primary"}
          onClick={isListening ? stopRemoteServer : startRemoteServer}
          style={isListening ? { borderColor: 'rgba(239, 68, 68, 0.5)', color: '#ef4444' } : { padding: '1rem 2rem' }}
        >
          {isListening ? "Stop Server" : "Start Server"}
        </button>
      </div>

      {remoteLogs.length > 0 && (
        <div style={{ background: 'rgba(0,0,0,0.3)', padding: '1.25rem', borderRadius: '12px', border: '1px solid var(--border-color)' }}>
          <p style={{ margin: '0 0 1rem 0', fontSize: '0.85rem', fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.05em', color: 'var(--text-muted)' }}>
            Remote Connection Logs
          </p>
          <ul style={{ margin: 0, paddingLeft: '1.2rem', fontSize: '0.85rem', color: 'var(--text-main)', display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
            {remoteLogs.map((log, i) => (
              <li key={i} style={{ fontFamily: 'var(--font-mono)' }}>{log}</li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
