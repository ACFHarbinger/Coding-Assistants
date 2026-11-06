import type React from "react";
import type { SettingsFieldStatus } from "../types";
import { NOC_COLORS, NOC_RADIUS, NOC_SURFACES } from "../../common/telemetry/tokens";
import { isTauriRuntime } from "../../../lib/tauri";

export const MIN_BACKUP_RETENTION = 1;
export const MAX_BACKUP_RETENTION = 20;
export const MIN_MEMORY_RECALL_LIMIT = 1;
export const DEFAULT_MEMORY_RECALL_LIMIT = 5;
export const MAX_MEMORY_RECALL_LIMIT = 20;

export const inputStyle: React.CSSProperties = {
  flex: "1 1 260px",
  minWidth: "200px",
  padding: "0.5rem 0.75rem",
  borderRadius: NOC_RADIUS.s,
  border: `1px solid ${NOC_SURFACES.border}`,
  background: NOC_SURFACES.bgDeep,
  color: NOC_SURFACES.textPrimary,
  fontSize: "0.85rem",
};

export function readWorkspaceRoot(): string | null {
  try {
    return localStorage.getItem("ca.workspaceRoot");
  } catch {
    return null;
  }
}

export async function closeSettingsWindow(): Promise<void> {
  if (!isTauriRuntime()) {
    window.close();
    return;
  }
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  try {
    await getCurrentWindow().close();
  } catch (error) {
    console.error("Failed to close the Settings window:", error);
  }
}

export function shortenPath(path: string, max = 42): string {
  if (path.length <= max) return path;
  return `…${path.slice(path.length - max + 1)}`;
}

export function StatusPill({ status }: { status: SettingsFieldStatus }) {
  const isOverride = status === "override";
  return (
    <span
      style={{
        fontSize: "0.7rem",
        fontWeight: 600,
        padding: "0.15rem 0.55rem",
        borderRadius: "999px",
        border: `1px solid ${isOverride ? "rgba(99, 102, 241, 0.45)" : NOC_SURFACES.border}`,
        color: isOverride ? NOC_COLORS.indigo : NOC_SURFACES.textMuted,
        background: isOverride ? "rgba(99, 102, 241, 0.12)" : "rgba(15, 23, 42, 0.6)",
        whiteSpace: "nowrap",
      }}
    >
      {isOverride ? "Workspace Override" : "Inherited"}
    </span>
  );
}

export function FieldRow({ label, hint, pill, children }: { label: string; hint?: string; pill?: React.ReactNode; children: React.ReactNode }) {
  return (
    <div style={{ marginBottom: "1.25rem" }}>
      <div style={{ display: "flex", alignItems: "center", gap: "0.6rem", marginBottom: "0.35rem" }}>
        <label style={{ fontWeight: 600, fontSize: "0.9rem", color: NOC_SURFACES.textSecondary }}>{label}</label>
        {pill}
      </div>
      {hint && <p style={{ margin: "0 0 0.5rem", color: NOC_SURFACES.textMuted, fontSize: "0.8rem", lineHeight: 1.5 }}>{hint}</p>}
      <div style={{ display: "flex", flexWrap: "wrap", gap: "0.5rem", alignItems: "center" }}>{children}</div>
    </div>
  );
}

export function ToggleRow({
  label,
  hint,
  checked,
  onToggle,
  disabled,
  pill,
  resetButton,
}: {
  label: string;
  hint?: string;
  checked: boolean;
  onToggle: () => void;
  disabled?: boolean;
  pill?: React.ReactNode;
  resetButton?: React.ReactNode;
}) {
  return (
    <div style={{ marginBottom: "1.1rem" }}>
      <div style={{ display: "flex", alignItems: "center", gap: "0.6rem", flexWrap: "wrap" }}>
        <button
          type="button"
          role="switch"
          aria-checked={checked}
          onClick={onToggle}
          disabled={disabled}
          className={`${checked ? "btn-primary" : "btn-secondary"} noc-focusable`}
          style={{ marginTop: 0, padding: "0.3rem 0.7rem", fontSize: "0.78rem", minWidth: "3.2rem" }}
        >
          {checked ? "On" : "Off"}
        </button>
        <label style={{ fontWeight: 600, fontSize: "0.9rem", color: NOC_SURFACES.textSecondary }}>{label}</label>
        {pill}
        {resetButton}
      </div>
      {hint && <p style={{ margin: "0.3rem 0 0", color: NOC_SURFACES.textMuted, fontSize: "0.8rem", lineHeight: 1.5 }}>{hint}</p>}
    </div>
  );
}
