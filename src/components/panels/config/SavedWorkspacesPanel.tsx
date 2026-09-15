import { useCallback, useEffect, useState } from "react";
import { invoke, isTauriRuntime } from "../../../lib/tauri";
import type { SavedWorkspace } from "../../../app/hubState";

export interface SavedWorkspacesPanelProps {
  workDir: string;
  activeWorkSessionId?: string | null;
  onSelectPath: (path: string) => void;
  onSelectLinkedSession?: (sessionId: string) => void;
}

function basename(path: string): string {
  const trimmed = path.replace(/[\\/]+$/, "");
  const parts = trimmed.split(/[\\/]/);
  return parts[parts.length - 1] || trimmed || "workspace";
}

export default function SavedWorkspacesPanel({
  workDir,
  activeWorkSessionId,
  onSelectPath,
  onSelectLinkedSession,
}: SavedWorkspacesPanelProps) {
  const [rows, setRows] = useState<SavedWorkspace[]>([]);
  const [nameDraft, setNameDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameDraft, setRenameDraft] = useState("");

  const refresh = useCallback(async () => {
    if (!isTauriRuntime()) {
      setRows([]);
      return;
    }
    try {
      const listed = await invoke<SavedWorkspace[]>("hub_list_workspaces");
      setRows(Array.isArray(listed) ? listed : []);
      setError("");
    } catch (err) {
      setError(String(err));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const saveCurrent = async () => {
    const path = workDir.trim();
    if (!path) {
      alert("Set an absolute workspace path first.");
      return;
    }
    const name = nameDraft.trim() || basename(path);
    setBusy(true);
    try {
      await invoke<SavedWorkspace>("hub_save_workspace", {
        id: null,
        name,
        path,
        linkSessionId: activeWorkSessionId ?? null,
      });
      setNameDraft("");
      await refresh();
    } catch (err) {
      alert(`Could not save workspace: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setBusy(false);
    }
  };

  const renameRow = async (row: SavedWorkspace) => {
    const name = renameDraft.trim();
    if (!name) return;
    setBusy(true);
    try {
      await invoke("hub_save_workspace", {
        id: row.id,
        name,
        path: row.path,
        linkSessionId: null,
      });
      setRenamingId(null);
      await refresh();
    } catch (err) {
      alert(`Could not rename: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setBusy(false);
    }
  };

  const deleteRow = async (row: SavedWorkspace) => {
    if (!window.confirm(`Remove saved workspace “${row.name}”? The folder on disk is not deleted.`)) {
      return;
    }
    setBusy(true);
    try {
      await invoke("hub_delete_workspace", { id: row.id });
      await refresh();
    } catch (err) {
      alert(`Could not delete: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setBusy(false);
    }
  };

  const selectRow = (row: SavedWorkspace) => {
    onSelectPath(row.path);
    if (row.linked_session_id && onSelectLinkedSession) {
      onSelectLinkedSession(row.linked_session_id);
    }
  };

  return (
    <div style={{ marginBottom: "0.9rem" }} aria-label="Saved workspaces">
      <div style={{ fontSize: "0.78rem", color: "var(--text-muted)", marginBottom: "0.45rem" }}>
        Saved workspaces
      </div>
      {error && (
        <div style={{ color: "#fca5a5", fontSize: "0.78rem", marginBottom: "0.4rem" }}>{error}</div>
      )}
      {rows.length === 0 && (
        <div style={{ color: "var(--text-muted)", fontSize: "0.78rem", marginBottom: "0.5rem" }}>
          None yet. Save the current path to reopen it later with its team chat.
        </div>
      )}
      <ul style={{ listStyle: "none", margin: 0, padding: 0, display: "grid", gap: "0.35rem" }}>
        {rows.map((row) => {
          const active = row.path === workDir.trim() || `${row.path}/` === workDir.trim();
          return (
            <li
              key={row.id}
              style={{
                display: "flex",
                gap: "0.45rem",
                alignItems: "center",
                flexWrap: "wrap",
                padding: "0.4rem 0.55rem",
                borderRadius: "8px",
                border: active ? "1px solid rgba(16, 185, 129, 0.55)" : "1px solid var(--border-color)",
                background: active ? "rgba(16, 185, 129, 0.12)" : "rgba(0,0,0,0.2)",
              }}
            >
              {renamingId === row.id ? (
                <>
                  <input
                    aria-label={`Rename ${row.name}`}
                    value={renameDraft}
                    onChange={(event) => setRenameDraft(event.target.value)}
                    style={{ flex: "1 1 160px", padding: "0.35rem 0.5rem", borderRadius: "6px", background: "rgba(0,0,0,0.35)", color: "white", border: "1px solid var(--border-color)" }}
                  />
                  <button type="button" className="btn-primary" style={{ marginTop: 0, padding: "0.3rem 0.65rem", fontSize: "0.75rem" }} onClick={() => void renameRow(row)} disabled={busy}>
                    Save
                  </button>
                  <button type="button" className="btn-secondary" style={{ marginTop: 0, padding: "0.3rem 0.65rem", fontSize: "0.75rem" }} onClick={() => setRenamingId(null)}>
                    Cancel
                  </button>
                </>
              ) : (
                <>
                  <button
                    type="button"
                    onClick={() => selectRow(row)}
                    style={{ flex: "1 1 180px", textAlign: "left", background: "transparent", border: 0, color: "var(--text-main)", cursor: "pointer", padding: 0 }}
                  >
                    <strong style={{ display: "block", fontSize: "0.85rem" }}>{row.name}</strong>
                    <span style={{ display: "block", fontSize: "0.72rem", color: "var(--text-muted)" }}>{row.path}</span>
                    {row.linked_session_name && (
                      <span style={{ display: "block", fontSize: "0.72rem", color: "#67e8f9" }}>Team chat: {row.linked_session_name}</span>
                    )}
                  </button>
                  <button type="button" className="btn-secondary" style={{ marginTop: 0, padding: "0.3rem 0.65rem", fontSize: "0.75rem" }} onClick={() => { setRenamingId(row.id); setRenameDraft(row.name); }}>
                    Rename
                  </button>
                  <button type="button" className="btn-secondary" style={{ marginTop: 0, padding: "0.3rem 0.65rem", fontSize: "0.75rem" }} onClick={() => void deleteRow(row)} disabled={busy}>
                    Delete
                  </button>
                </>
              )}
            </li>
          );
        })}
      </ul>
      <div style={{ display: "flex", gap: "0.5rem", flexWrap: "wrap", marginTop: "0.55rem", alignItems: "center" }}>
        <input
          aria-label="New saved workspace name"
          placeholder="Name (defaults to folder)"
          value={nameDraft}
          onChange={(event) => setNameDraft(event.target.value)}
          style={{ flex: "1 1 180px", padding: "0.45rem 0.6rem", borderRadius: "8px", background: "rgba(0,0,0,0.3)", color: "white", border: "1px solid var(--border-color)" }}
        />
        <button type="button" className="btn-secondary" style={{ marginTop: 0 }} onClick={() => void saveCurrent()} disabled={busy}>
          Save current as…
        </button>
      </div>
    </div>
  );
}
