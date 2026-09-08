import { evaluateProviderHealth, type ProviderHealth } from "./healthTypes";

const DISPLAY_NAMES: Record<string, string> = {
  grok: "Grok",
  chat: "Codex",
  codex: "Codex",
  claude: "Claude",
  gemini: "Gemini",
  agy: "Gemini",
  muse: "Muse",
  cursor: "Cursor",
};

interface ProviderHealthChipProps {
  providerId: string;
  health?: ProviderHealth | null;
}

export default function ProviderHealthChip({ providerId, health }: ProviderHealthChipProps) {
  const status = evaluateProviderHealth(health);
  const displayName = health?.harnessTitle || DISPLAY_NAMES[providerId.toLowerCase()] || providerId;

  return (
    <div
      title={`[${displayName}] ${status.detail}`}
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: "0.45rem",
        padding: "0.3rem 0.65rem",
        borderRadius: "9999px",
        background: "rgba(0, 0, 0, 0.35)",
        border: `1px solid ${status.dotColor}55`,
        fontSize: "0.78rem",
        color: "var(--text-main)",
        cursor: "help",
        boxShadow: "0 1px 2px rgba(0,0,0,0.2)",
      }}
    >
      <span
        style={{
          width: "7px",
          height: "7px",
          borderRadius: "50%",
          backgroundColor: status.dotColor,
          boxShadow: `0 0 6px ${status.dotColor}88`,
          flexShrink: 0,
        }}
      />
      <strong style={{ fontWeight: 600 }}>{displayName}</strong>
      <span
        style={{
          color: "var(--text-muted)",
          fontSize: "0.72rem",
          fontWeight: 500,
        }}
      >
        {status.label}
      </span>
    </div>
  );
}

export function ProviderHealthDot({ health, titlePrefix = "Health" }: { health?: ProviderHealth | null; titlePrefix?: string }) {
  const status = evaluateProviderHealth(health);
  return (
    <span
      title={`${titlePrefix}: ${status.label} (${status.detail})`}
      style={{
        display: "inline-block",
        width: "6px",
        height: "6px",
        borderRadius: "50%",
        backgroundColor: status.dotColor,
        opacity: 0.85,
        flexShrink: 0,
        boxShadow: `0 0 4px ${status.dotColor}66`,
      }}
    />
  );
}
