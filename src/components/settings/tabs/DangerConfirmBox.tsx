import { useState } from "react";

export type DangerTone = "red" | "amber";

export interface DangerConfirmBoxProps {
  /** Exact string the user must type. Comparison trims surrounding space. */
  targetName: string;
  /** Instruction line rendered above the input; name the target. */
  prompt: string;
  /** Error shown when the typed text does not match. */
  mismatchError: string;
  confirmLabel: string;
  cancelLabel: string;
  tone?: DangerTone;
  busy: boolean;
  onCancel: () => void;
  onConfirm: () => void;
  onError: (message: string) => void;
}

/**
 * Shared typed-target confirmation box for the danger tab (S6 / #132).
 * Every destructive section reuses this one pattern: red/amber treatment,
 * Cancel-first focus (`autoFocus` sits on Cancel), and a confirm button
 * that stays disabled until the typed text matches. Cancellation only ever
 * calls `onCancel` — it never touches data.
 */
export default function DangerConfirmBox({
  targetName,
  prompt,
  mismatchError,
  confirmLabel,
  cancelLabel,
  tone = "red",
  busy,
  onCancel,
  onConfirm,
  onError,
}: DangerConfirmBoxProps) {
  const [typed, setTyped] = useState("");
  const accent = tone === "red" ? "248, 113, 113" : "251, 191, 36";
  const matched = typed.trim() === targetName;

  return (
    <div
      style={{
        marginTop: "1rem",
        padding: "0.85rem",
        borderRadius: "8px",
        background: `rgba(${tone === "red" ? "239, 68, 68" : "245, 158, 11"}, 0.08)`,
        border: `1px solid rgba(${accent}, 0.5)`,
        display: "grid",
        gap: "0.6rem",
      }}
    >
      <div style={{ fontSize: "0.82rem", color: tone === "red" ? "#fecaca" : "#fde68a" }}>{prompt}</div>
      <input
        style={{
          padding: "0.45rem 0.7rem",
          borderRadius: "8px",
          border: `1px solid rgba(${accent}, 0.6)`,
          background: "rgba(0,0,0,0.4)",
          color: "white",
          fontSize: "0.85rem",
        }}
        placeholder={targetName}
        value={typed}
        onChange={(e) => setTyped(e.target.value)}
      />
      <div style={{ display: "flex", gap: "0.5rem", justifyContent: "flex-end", marginTop: "0.25rem" }}>
        <button
          type="button"
          autoFocus
          className="btn-secondary"
          style={{ marginTop: 0, padding: "0.35rem 0.8rem", fontSize: "0.78rem" }}
          onClick={onCancel}
        >
          {cancelLabel}
        </button>
        <button
          type="button"
          className="btn-primary"
          style={{
            marginTop: 0,
            padding: "0.35rem 0.8rem",
            fontSize: "0.78rem",
            background: tone === "red" ? "#dc2626" : "#b45309",
            borderColor: tone === "red" ? "#ef4444" : "#f59e0b",
          }}
          disabled={busy || !matched}
          onClick={() => {
            if (!matched) {
              onError(mismatchError);
              return;
            }
            onConfirm();
          }}
        >
          {confirmLabel}
        </button>
      </div>
    </div>
  );
}
