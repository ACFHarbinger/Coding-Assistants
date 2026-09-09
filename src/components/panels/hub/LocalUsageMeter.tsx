import type { ProviderQuotaLocalUsage } from "./types";

interface LocalUsageMeterProps {
  usage: ProviderQuotaLocalUsage;
  detail?: string | null;
}

function formatSince(timestamp?: number | null): string | null {
  if (!timestamp || timestamp <= 0) return null;
  try {
    return new Date(timestamp * 1000).toLocaleDateString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  } catch {
    return null;
  }
}

export function LocalUsageMeter({ usage, detail }: LocalUsageMeterProps) {
  const sinceStr = formatSince(usage.since);
  const totalTools =
    usage.tool_calls_succeeded +
    usage.tool_calls_failed +
    usage.tool_calls_rejected;

  return (
    <div
      style={{
        display: "grid",
        gap: "0.75rem",
        marginTop: "0.25rem",
      }}
      data-testid="local-usage-meter"
    >
      {/* Header with unmetered badge */}
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          flexWrap: "wrap",
          gap: "0.5rem",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
          <span style={{ fontWeight: 600, fontSize: "0.88rem", color: "var(--text-main)" }}>
            Local Session Activity
          </span>
          <span
            style={{
              fontSize: "0.68rem",
              padding: "0.15rem 0.45rem",
              borderRadius: "10px",
              background: "rgba(56, 189, 248, 0.12)",
              color: "#38bdf8",
              border: "1px solid rgba(56, 189, 248, 0.25)",
              fontWeight: 500,
            }}
          >
            Local · Unmetered
          </span>
        </div>
        {sinceStr && (
          <span style={{ fontSize: "0.72rem", color: "var(--text-muted)" }}>
            Tracking sessions since {sinceStr}
          </span>
        )}
      </div>

      {/* Raw counts grid: sessions, prompt tokens, completion tokens, cached tokens, tools */}
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fit, minmax(130px, 1fr))",
          gap: "0.5rem",
        }}
      >
        {/* Sessions */}
        <div
          style={{
            background: "rgba(0, 0, 0, 0.2)",
            border: "1px solid var(--border-color)",
            borderRadius: "8px",
            padding: "0.6rem 0.8rem",
            display: "grid",
            gap: "0.2rem",
          }}
        >
          <span style={{ fontSize: "0.72rem", color: "var(--text-muted)", textTransform: "uppercase", letterSpacing: "0.04em", fontWeight: 600 }}>
            Sessions
          </span>
          <span style={{ fontSize: "1.25rem", fontWeight: 700, color: "var(--text-main)" }}>
            {usage.sessions.toLocaleString()}
          </span>
          <span style={{ fontSize: "0.7rem", color: "var(--text-muted)" }}>
            Run workspaces
          </span>
        </div>

        {/* Prompt tokens */}
        <div
          style={{
            background: "rgba(0, 0, 0, 0.2)",
            border: "1px solid var(--border-color)",
            borderRadius: "8px",
            padding: "0.6rem 0.8rem",
            display: "grid",
            gap: "0.2rem",
          }}
        >
          <span style={{ fontSize: "0.72rem", color: "var(--text-muted)", textTransform: "uppercase", letterSpacing: "0.04em", fontWeight: 600 }}>
            Prompt Tokens
          </span>
          <span style={{ fontSize: "1.25rem", fontWeight: 700, color: "#38bdf8" }}>
            {usage.prompt_tokens.toLocaleString()}
          </span>
          <span style={{ fontSize: "0.7rem", color: "var(--text-muted)" }}>
            Input context
          </span>
        </div>

        {/* Completion tokens */}
        <div
          style={{
            background: "rgba(0, 0, 0, 0.2)",
            border: "1px solid var(--border-color)",
            borderRadius: "8px",
            padding: "0.6rem 0.8rem",
            display: "grid",
            gap: "0.2rem",
          }}
        >
          <span style={{ fontSize: "0.72rem", color: "var(--text-muted)", textTransform: "uppercase", letterSpacing: "0.04em", fontWeight: 600 }}>
            Completion Tokens
          </span>
          <span style={{ fontSize: "1.25rem", fontWeight: 700, color: "#a855f7" }}>
            {usage.completion_tokens.toLocaleString()}
          </span>
          <span style={{ fontSize: "0.7rem", color: "var(--text-muted)" }}>
            Output generated
          </span>
        </div>

        {/* Cached tokens */}
        <div
          style={{
            background: "rgba(0, 0, 0, 0.2)",
            border: "1px solid var(--border-color)",
            borderRadius: "8px",
            padding: "0.6rem 0.8rem",
            display: "grid",
            gap: "0.2rem",
          }}
        >
          <span style={{ fontSize: "0.72rem", color: "var(--text-muted)", textTransform: "uppercase", letterSpacing: "0.04em", fontWeight: 600 }}>
            Cached Tokens
          </span>
          <span style={{ fontSize: "1.25rem", fontWeight: 700, color: "#34d399" }}>
            {usage.cached_tokens.toLocaleString()}
          </span>
          <span style={{ fontSize: "0.7rem", color: "var(--text-muted)" }}>
            Prompt cache hits
          </span>
        </div>

        {/* Tool calls */}
        <div
          style={{
            background: "rgba(0, 0, 0, 0.2)",
            border: "1px solid var(--border-color)",
            borderRadius: "8px",
            padding: "0.6rem 0.8rem",
            display: "grid",
            gap: "0.2rem",
          }}
        >
          <span style={{ fontSize: "0.72rem", color: "var(--text-muted)", textTransform: "uppercase", letterSpacing: "0.04em", fontWeight: 600 }}>
            Tool Calls
          </span>
          <span style={{ fontSize: "1.25rem", fontWeight: 700, color: "var(--text-main)" }}>
            {totalTools.toLocaleString()}
          </span>
          <span style={{ fontSize: "0.7rem", color: "var(--text-muted)" }}>
            {usage.tool_calls_succeeded} ok · {usage.tool_calls_failed} err
            {usage.tool_calls_rejected > 0 ? ` · ${usage.tool_calls_rejected} rej` : ""}
          </span>
        </div>
      </div>

      {/* Detail advisory (e.g. admin key note) */}
      {detail && (
        <div
          style={{
            fontSize: "0.78rem",
            color: "var(--text-muted)",
            background: "rgba(255, 255, 255, 0.03)",
            border: "1px solid var(--border-color)",
            borderRadius: "6px",
            padding: "0.45rem 0.65rem",
          }}
        >
          {detail}
        </div>
      )}
    </div>
  );
}
