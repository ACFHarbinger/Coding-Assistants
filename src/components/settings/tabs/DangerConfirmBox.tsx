import { useState } from "react";
import { NOC_COLORS, NOC_RADIUS, NOC_SURFACES } from "../../common/telemetry/tokens";

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
  const isRed = tone === "red";
  const borderTone = isRed ? NOC_COLORS.crit : NOC_COLORS.amber;
  const matched = typed.trim() === targetName;

  return (
    <div
      style={{
        marginTop: "1rem",
        padding: "0.85rem 1rem",
        borderRadius: NOC_RADIUS.s,
        background: isRed ? "rgba(239, 68, 68, 0.10)" : "rgba(245, 158, 11, 0.10)",
        border: `1px solid ${borderTone}`,
        display: "grid",
        gap: "0.6rem",
      }}
    >
      <div style={{ fontSize: "0.82rem", fontWeight: 600, color: isRed ? "#fecaca" : "#fde68a" }}>{prompt}</div>
      <input
        style={{
          padding: "0.45rem 0.75rem",
          borderRadius: NOC_RADIUS.s,
          border: `1px solid ${borderTone}`,
          background: NOC_SURFACES.bgDeep,
          color: NOC_SURFACES.textPrimary,
          fontSize: "0.85rem",
        }}
        className="noc-focusable"
        placeholder={targetName}
        value={typed}
        onChange={(e) => setTyped(e.target.value)}
      />
      <div style={{ display: "flex", gap: "0.5rem", justifyContent: "flex-end", marginTop: "0.25rem" }}>
        <button
          type="button"
          autoFocus
          className="btn-secondary noc-focusable"
          style={{ marginTop: 0, padding: "0.35rem 0.85rem", fontSize: "0.78rem" }}
          onClick={onCancel}
        >
          {cancelLabel}
        </button>
        <button
          type="button"
          className="btn-primary noc-focusable"
          style={{
            marginTop: 0,
            padding: "0.35rem 0.85rem",
            fontSize: "0.78rem",
            background: isRed ? NOC_COLORS.crit : NOC_COLORS.amber,
            borderColor: borderTone,
            color: isRed ? "#ffffff" : "#0f172a",
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
