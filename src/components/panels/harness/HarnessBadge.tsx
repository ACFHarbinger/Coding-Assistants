import { harnessTone, type HarnessSessionMode, type HarnessSessionState } from "./types";

export default function HarnessBadge({ mode, state }: { mode: HarnessSessionMode; state: HarnessSessionState }) {
  const tone = harnessTone(mode, state);
  return (
    <span
      role="status"
      aria-label={`${mode} ${state}`}
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: "0.35rem",
        padding: "0.12rem 0.45rem",
        borderRadius: "999px",
        fontSize: "0.7rem",
        fontWeight: 700,
        letterSpacing: "0.03em",
        textTransform: "uppercase",
        color: tone.color,
        border: `1px solid ${tone.border}`,
        background: tone.bg,
      }}
    >
      <span>{mode}</span>
      <span aria-hidden="true">·</span>
      <span>{state}</span>
    </span>
  );
}
