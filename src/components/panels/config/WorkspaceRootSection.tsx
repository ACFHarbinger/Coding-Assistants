import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState, useCallback } from "react";
import { invoke, isTauriRuntime } from "../../../lib/tauri";
import type { WorkspaceValidation } from "./types";
import SavedWorkspacesPanel from "./SavedWorkspacesPanel";

export interface WorkspaceRootSectionProps {
  workDir: string;
  onWorkDirChange: (newPath: string) => void;
  onWorkspaceApplied: (newPath: string) => void;
  workspaceNotice?: string;
  setWorkspaceNotice?: (notice: string) => void;
  activeWorkSessionId?: string | null;
  onSelectWorkSession?: (sessionId: string | null) => void;
  onSwitchToChatView?: () => void;
}

export default function WorkspaceRootSection({
  workDir,
  onWorkDirChange,
  onWorkspaceApplied,
  workspaceNotice,
  setWorkspaceNotice,
  activeWorkSessionId,
  onSelectWorkSession,
  onSwitchToChatView,
}: WorkspaceRootSectionProps) {
  const [validation, setValidation] = useState<WorkspaceValidation | null>(null);
  const [validating, setValidating] = useState(false);
  const [bootstrapping, setBootstrapping] = useState(false);
  const [localNotice, setLocalNotice] = useState("");

  const displayNotice = workspaceNotice ?? localNotice;
  const updateNotice = useCallback(
    (notice: string) => {
      if (setWorkspaceNotice) {
        setWorkspaceNotice(notice);
      } else {
        setLocalNotice(notice);
      }
      setTimeout(() => {
        if (setWorkspaceNotice) setWorkspaceNotice("");
        else setLocalNotice("");
      }, 4000);
    },
    [setWorkspaceNotice]
  );

  const validatePath = useCallback(async (pathToCheck: string) => {
    const trimmed = pathToCheck.trim();
    if (!trimmed) {
      setValidation(null);
      return;
    }
    if (!isTauriRuntime()) {
      // Fallback for browser tests / environments without Tauri internals
      const isAbs = trimmed.startsWith("/") || trimmed.startsWith("~") || /^[a-zA-Z]:[/\\]/.test(trimmed);
      setValidation({
        path: trimmed,
        valid: isAbs,
        exists: true,
        is_dir: true,
        is_bootstrapped: true,
        parent_exists: true,
        is_system_dir: false,
        error: isAbs ? null : "Workspace root must be an absolute path",
      });
      return;
    }

    setValidating(true);
    try {
      const res = await invoke<WorkspaceValidation>("validate_workspace_path", { path: trimmed });
      setValidation(res);
    } catch (err) {
      setValidation({
        path: trimmed,
        valid: false,
        exists: false,
        is_dir: false,
        is_bootstrapped: false,
        parent_exists: false,
        is_system_dir: false,
        error: String(err),
      });
    } finally {
      setValidating(false);
    }
  }, []);

  useEffect(() => {
    const timer = setTimeout(() => {
      void validatePath(workDir);
    }, 250);
    return () => clearTimeout(timer);
  }, [workDir, validatePath]);

  const handleApply = async () => {
    const trimmed = workDir.trim();
    if (!trimmed) {
      alert("Set an absolute workspace path first.");
      return;
    }

    let currentVal = validation;
    if (!currentVal || currentVal.path !== trimmed) {
      if (isTauriRuntime()) {
        try {
          currentVal = await invoke<WorkspaceValidation>("validate_workspace_path", { path: trimmed });
          setValidation(currentVal);
        } catch {
          // ignore
        }
      }
    }

    if (currentVal?.is_system_dir) {
      alert(`Cannot switch to system directory: ${trimmed}`);
      return;
    }
    if (currentVal && !currentVal.exists) {
      const proceed = window.confirm(
        `Directory '${trimmed}' does not exist on disk yet. Switch to this path anyway?`
      );
      if (!proceed) return;
    }
    onWorkspaceApplied(trimmed);
  };

  const handleBrowse = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        defaultPath: workDir.trim() || undefined,
      });
      if (selected && typeof selected === "string") {
        onWorkDirChange(selected);
        onWorkspaceApplied(selected);
      }
    } catch (err) {
      console.error("Failed to open directory picker:", err);
    }
  };

  const handleInitialize = async () => {
    const trimmed = workDir.trim();
    if (!trimmed) {
      alert("Set an absolute workspace path first.");
      return;
    }
    if (bootstrapping) return;

    let currentVal = validation;
    if (!currentVal || currentVal.path !== trimmed) {
      if (isTauriRuntime()) {
        try {
          currentVal = await invoke<WorkspaceValidation>("validate_workspace_path", { path: trimmed });
          setValidation(currentVal);
        } catch {
          // ignore
        }
      }
    }

    if (currentVal?.is_system_dir) {
      alert(`Cannot bootstrap in system directory: ${trimmed}`);
      return;
    }
    if (currentVal?.is_bootstrapped) {
      alert("Workspace is already bootstrapped (.agent/ directory exists).");
      return;
    }

    let createDir = false;
    if (currentVal && !currentVal.exists) {
      if (!currentVal.parent_exists) {
        alert(
          `Cannot create workspace '${trimmed}': parent directory does not exist. Please create parent directories first.`
        );
        return;
      }
      const confirmed = window.confirm(
        `Directory '${trimmed}' does not exist. Create this directory and initialize .agent/ in it?`
      );
      if (!confirmed) return;
      createDir = true;
    }

    setBootstrapping(true);
    try {
      await invoke("bootstrap_workspace", { workDir: trimmed, createDir });
      updateNotice(`Successfully bootstrapped .agent/ in ${trimmed}`);
      await validatePath(trimmed);
    } catch (err) {
      alert(`Failed to bootstrap: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setBootstrapping(false);
    }
  };

  return (
    <section
      style={{
        marginBottom: "1.5rem",
        padding: "1.25rem",
        border: "1px solid rgba(16, 185, 129, 0.32)",
        borderRadius: "12px",
        background: "rgba(16, 185, 129, 0.06)",
      }}
      aria-label="Workspace Root Section"
    >
      <label
        className="label"
        htmlFor="workspace-root-input"
        style={{
          fontWeight: 700,
          color: "var(--text-primary)",
          marginBottom: "0.5rem",
          display: "block",
        }}
      >
        Workspace Root
      </label>
      <div style={{ color: "var(--text-muted)", fontSize: "0.82rem", marginBottom: "0.75rem" }}>
        All team sessions, harness capture, and task delivery use this absolute repository path. Direct path entry and ~ expansion supported.
      </div>
      <SavedWorkspacesPanel
        workDir={workDir}
        activeWorkSessionId={activeWorkSessionId}
        onSelectPath={(path) => {
          onWorkDirChange(path);
          onWorkspaceApplied(path);
        }}
        onSelectLinkedSession={onSelectWorkSession ? (sessionId) => {
          onSelectWorkSession(sessionId);
          onSwitchToChatView?.();
        } : undefined}
      />
      <div style={{ display: "flex", gap: "0.75rem", flexWrap: "wrap", alignItems: "center" }}>
        <input
          id="workspace-root-input"
          style={{
            flex: "1 1 360px",
            padding: "0.75rem",
            borderRadius: "8px",
            background: "rgba(0,0,0,0.3)",
            color: "white",
            border: validation && !validation.valid
              ? "1px solid rgba(239, 68, 68, 0.6)"
              : "1px solid var(--border-color)",
            outline: "none",
          }}
          placeholder="/absolute/path/to/workspace or ~/repo"
          value={workDir}
          onChange={(e) => onWorkDirChange(e.target.value)}
        />
        <button
          className="btn-primary"
          style={{ marginTop: 0 }}
          onClick={handleApply}
          title="Switch active workspace to this directory"
        >
          Switch Workspace
        </button>
        <button
          className="btn-secondary"
          style={{ marginTop: 0 }}
          onClick={() => void handleBrowse()}
        >
          Browse
        </button>
        <button
          className="btn-secondary"
          style={{
            marginTop: 0,
            background: "rgba(16, 185, 129, 0.1)",
            color: "#10b981",
            borderColor: "rgba(16, 185, 129, 0.3)",
          }}
          onClick={() => void handleInitialize()}
          disabled={bootstrapping}
        >
          {bootstrapping ? "Initializing…" : "Initialize .agent/"}
        </button>
      </div>

      {/* Validation status badge */}
      {workDir.trim() && (
        <div style={{ marginTop: "0.6rem", fontSize: "0.82rem", display: "flex", alignItems: "center", gap: "0.5rem", flexWrap: "wrap" }}>
          {validating && (
            <span style={{ color: "var(--text-muted)" }}>Checking path…</span>
          )}
          {!validating && validation && validation.is_system_dir && (
            <span style={{ color: "#ef4444", fontWeight: 600 }}>
              ⚠ Forbidden: System directory cannot be used as workspace root.
            </span>
          )}
          {!validating && validation && !validation.is_system_dir && !validation.exists && (
            <span style={{ color: "#f59e0b" }}>
              Directory does not exist on disk {!validation.parent_exists ? "(parent also missing)" : "(ready to create)"}.
            </span>
          )}
          {!validating && validation && validation.exists && !validation.is_dir && (
            <span style={{ color: "#ef4444" }}>
              Path points to a file, not a directory.
            </span>
          )}
          {!validating && validation && validation.exists && validation.is_dir && validation.is_bootstrapped && (
            <span style={{ color: "#10b981" }}>
              ✓ Ready: .agent/ initialized
            </span>
          )}
          {!validating && validation && validation.exists && validation.is_dir && !validation.is_bootstrapped && (
            <span style={{ color: "#38bdf8" }}>
              Directory exists; .agent/ not initialized. Click "Initialize .agent/" to scaffold.
            </span>
          )}
          {validation && validation.error && !validation.is_system_dir && !validation.exists && (
            <span style={{ color: "var(--text-muted)", fontSize: "0.78rem" }}>
              ({validation.error})
            </span>
          )}
        </div>
      )}

      {displayNotice && (
        <div
          style={{
            marginTop: "0.75rem",
            padding: "0.5rem 0.75rem",
            borderRadius: "8px",
            background: "rgba(16, 185, 129, 0.15)",
            border: "1px solid rgba(16, 185, 129, 0.4)",
            color: "#34d399",
            fontSize: "0.85rem",
            display: "flex",
            alignItems: "center",
            gap: "0.4rem",
          }}
        >
          <span>✓</span> {displayNotice}
        </div>
      )}
    </section>
  );
}
