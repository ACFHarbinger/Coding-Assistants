import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface TaskRecord {
  id: string;
  title: string;
  workspace_path: string | null;
  status: string;
  step_index: number;
  steps: any[];
  created_at: string;
  updated_at: string;
  last_message_id: string | null;
  attempts: Record<string, number>;
  open_agents: string[];
  pending_agents: string[];
  max_parallel: number;
}

export interface MessageRecord {
  id: string;
  from_agent: string;
  to_agent: string;
  workspace_path: string | null;
  task_id: string | null;
  kind: string;
  status: string;
  subject: string | null;
  body: string;
  created_at: string;
  acked_at: string | null;
}

export default function TaskTab() {
  const [tasks, setTasks] = useState<TaskRecord[]>([]);
  const [selectedTask, setSelectedTask] = useState<TaskRecord | null>(null);
  const [taskMessages, setTaskMessages] = useState<MessageRecord[]>([]);
  
  const refreshTasks = useCallback(async () => {
    try {
      const res = await invoke<TaskRecord[]>("hub_list_tasks", { status: null });
      setTasks(res);
    } catch (e) {
      console.error(e);
    }
  }, []);

  const selectTask = async (task: TaskRecord) => {
    setSelectedTask(task);
    try {
      const msgs = await invoke<MessageRecord[]>("hub_list_messages", { to: null, status: null });
      setTaskMessages(msgs.filter(m => m.task_id === task.id).sort((a, b) => a.created_at.localeCompare(b.created_at)));
    } catch (e) {
      console.error(e);
    }
  };

  useEffect(() => {
    refreshTasks();
  }, [refreshTasks]);

  const cardStyle = {
    background: "rgba(0, 0, 0, 0.4)",
    borderRadius: "12px",
    padding: "1.25rem",
    border: "1px solid var(--border-color)",
    transition: "all 0.2s ease"
  };

  if (selectedTask) {
    return (
      <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1.5rem" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <h3 style={{ margin: 0, fontSize: "1.25rem", color: "var(--text-main)" }}>Task: {selectedTask.title}</h3>
          <button className="btn-secondary" onClick={() => setSelectedTask(null)}>Back to Tasks</button>
        </div>
        <div style={{ display: "flex", gap: "1rem", flexWrap: "wrap", fontSize: "0.85rem", color: "var(--text-muted)" }}>
          <span>ID: <code style={{ color: "var(--primary)" }}>{selectedTask.id.slice(0, 8)}</code></span>
          <span>Status: <strong style={{ color: selectedTask.status === "completed" ? "#10b981" : "#38bdf8" }}>{selectedTask.status}</strong></span>
          {selectedTask.workspace_path && <span>Workspace: {selectedTask.workspace_path}</span>}
          <span>Stage: {selectedTask.step_index} / {selectedTask.steps.length}</span>
        </div>
        
        <h4 style={{ margin: "1rem 0 0", color: "var(--text-main)" }}>Transcript & Handoffs</h4>
        <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
          {taskMessages.length === 0 ? (
            <p style={{ color: "var(--text-muted)", fontStyle: "italic" }}>No messages or handoffs recorded for this task.</p>
          ) : (
            taskMessages.map(m => (
              <div key={m.id} style={cardStyle}>
                <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "0.5rem" }}>
                  <div>
                    <span style={{ color: "var(--primary)", fontWeight: 600 }}>{m.from_agent}</span>
                    <span style={{ color: "var(--text-muted)", margin: "0 0.5rem" }}>→</span>
                    <span style={{ color: "var(--accent)", fontWeight: 600 }}>{m.to_agent}</span>
                  </div>
                  <div style={{ display: "flex", gap: "0.5rem", fontSize: "0.75rem" }}>
                    <span style={{ background: "rgba(255,255,255,0.1)", padding: "0.1rem 0.4rem", borderRadius: "4px" }}>{m.kind}</span>
                    <span style={{ color: "var(--text-muted)" }}>{new Date(m.created_at).toLocaleTimeString()}</span>
                  </div>
                </div>
                {m.subject && <div style={{ fontWeight: 600, marginBottom: "0.5rem", color: "var(--text-main)" }}>{m.subject}</div>}
                <pre style={{ margin: 0, whiteSpace: "pre-wrap", fontFamily: "var(--font-sans)", fontSize: "0.9rem", color: "var(--text-main)" }}>
                  {m.body}
                </pre>
              </div>
            ))
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "0.5rem" }}>
        <h3 style={{ margin: 0, fontSize: "1.1rem", color: "var(--text-main)" }}>Task Browser</h3>
        <button className="btn-secondary" onClick={refreshTasks}>Refresh</button>
      </div>
      
      {tasks.length === 0 ? (
        <div style={{ padding: "3rem", textAlign: "center", background: "rgba(0,0,0,0.2)", borderRadius: "12px", border: "1px dashed var(--border-color)" }}>
          <p style={{ color: "var(--text-muted)", fontSize: "0.95rem", margin: 0 }}>No tasks found in the hub.</p>
        </div>
      ) : (
        tasks.map(t => (
          <div key={t.id} style={{ ...cardStyle, cursor: "pointer" }} onClick={() => selectTask(t)} onMouseEnter={e => e.currentTarget.style.borderColor = 'var(--primary)'} onMouseLeave={e => e.currentTarget.style.borderColor = 'var(--border-color)'}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "0.5rem" }}>
              <strong style={{ fontSize: "1.1rem", color: "var(--primary)" }}>{t.title}</strong>
              <span style={{ fontSize: "0.8rem", color: t.status === "completed" ? "#10b981" : "#38bdf8", background: "rgba(255,255,255,0.1)", padding: "0.2rem 0.5rem", borderRadius: "4px" }}>
                {t.status.toUpperCase()}
              </span>
            </div>
            <div style={{ display: "flex", gap: "1rem", fontSize: "0.8rem", color: "var(--text-muted)" }}>
              <span>ID: {t.id.slice(0, 8)}</span>
              {t.workspace_path && <span>Workspace: {t.workspace_path.split('/').pop()}</span>}
              <span>Created: {new Date(t.created_at).toLocaleString()}</span>
            </div>
          </div>
        ))
      )}
    </div>
  );
}
