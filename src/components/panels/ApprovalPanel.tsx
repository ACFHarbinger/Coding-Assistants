
interface ApprovalPanelProps {
  authorizationRequest: { role: string, question: string } | null;
  respondToAuthorization: (approved: boolean) => Promise<void>;
  currentQuestion: string | null;
  userInput: string;
  setUserInput: (val: string) => void;
  submitAnswer: () => Promise<void>;
}

export default function ApprovalPanel({
  authorizationRequest,
  respondToAuthorization,
  currentQuestion,
  userInput,
  setUserInput,
  submitAnswer
}: ApprovalPanelProps) {
  if (!authorizationRequest && !currentQuestion) return null;

  return (
    <>
      {authorizationRequest && (
        <div style={{
          position: 'fixed',
          top: 0, left: 0, right: 0, bottom: 0,
          background: 'rgba(2,6,23,0.92)',
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
          zIndex: 2000
        }}>
          <div className="fade-in" style={{
            background: 'var(--bg-card)',
            border: '1px solid var(--primary)',
            borderRadius: '16px',
            padding: '2.5rem',
            maxWidth: '550px',
            width: '90%',
            boxShadow: '0 25px 50px -12px rgba(99, 102, 241, 0.25), 0 0 0 1px rgba(99, 102, 241, 0.1)'
          }}>
            <h2 style={{ marginTop: 0, color: 'var(--text-main)', fontSize: '1.5rem', display: 'flex', alignItems: 'center', gap: '0.75rem', fontWeight: 700 }}>
              <span style={{ fontSize: '1.75rem' }}>🛡️</span> Authorization Required
            </h2>
            <div style={{ margin: '2rem 0', fontSize: '1.05rem', lineHeight: '1.6' }}>
              <p style={{ color: 'var(--text-muted)' }}>An agent wants to ask a question to the <strong style={{ color: 'var(--primary)', fontWeight: 600 }}>{authorizationRequest.role}</strong>:</p>
              <div style={{ 
                background: 'rgba(0,0,0,0.3)', 
                padding: '1.25rem', 
                borderRadius: '12px', 
                fontStyle: 'italic',
                marginTop: '1rem',
                borderLeft: '4px solid var(--accent)',
                color: 'var(--text-main)'
              }}>
                "{authorizationRequest.question}"
              </div>
            </div>
            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '1rem' }}>
              <button 
                className="btn-secondary" 
                style={{ borderColor: 'rgba(239, 68, 68, 0.5)', color: '#ef4444', padding: '0.75rem 1.5rem' }} 
                onClick={() => respondToAuthorization(false)}
              >
                Deny Request
              </button>
              <button 
                className="btn-primary" 
                style={{ padding: '0.75rem 1.5rem' }}
                onClick={() => respondToAuthorization(true)}
              >
                Approve Request
              </button>
            </div>
          </div>
        </div>
      )}

      {currentQuestion && (
        <div style={{
          position: 'fixed',
          top: 0, left: 0, right: 0, bottom: 0,
          background: 'rgba(2,6,23,0.92)',
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
          zIndex: 2000
        }}>
          <div className="fade-in" style={{
            background: 'var(--bg-card)',
            border: '1px solid var(--accent)',
            borderRadius: '16px',
            padding: '2.5rem',
            maxWidth: '650px',
            width: '90%',
            boxShadow: '0 25px 50px -12px rgba(168, 85, 247, 0.25), 0 0 0 1px rgba(168, 85, 247, 0.1)'
          }}>
            <h2 style={{ marginTop: 0, color: 'var(--text-main)', fontSize: '1.5rem', display: 'flex', alignItems: 'center', gap: '0.75rem', fontWeight: 700 }}>
              <span style={{ fontSize: '1.75rem' }}>❓</span> Agent Needs Input
            </h2>
            <p style={{ fontSize: '1.1rem', lineHeight: '1.6', margin: '2rem 0', color: 'var(--text-muted)' }}>
              {currentQuestion}
            </p>
            <textarea
              value={userInput}
              onChange={e => setUserInput(e.target.value)}
              placeholder="Type your answer here..."
              rows={5}
              style={{
                width: '100%',
                background: 'rgba(0,0,0,0.4)',
                border: '1px solid var(--border-color)',
                borderRadius: '12px',
                padding: '1.25rem',
                color: 'var(--text-main)',
                fontFamily: 'var(--font-sans)',
                fontSize: '1rem',
                marginBottom: '1.5rem',
                resize: 'vertical',
                outline: 'none',
                transition: 'border-color 0.2s',
                boxShadow: 'inset 0 2px 4px rgba(0,0,0,0.1)'
              }}
              onFocus={e => e.target.style.borderColor = 'var(--accent)'}
              onBlur={e => e.target.style.borderColor = 'var(--border-color)'}
              autoFocus
            />
            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '1rem' }}>
              <button 
                className="btn-primary" 
                style={{ background: 'linear-gradient(135deg, var(--accent), var(--primary))', padding: '0.875rem 2rem' }}
                onClick={submitAnswer}
              >
                Submit Answer
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
