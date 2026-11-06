const AGENTS = [
  { id: "grok", label: "Grok", cx: 200, cy: 36 },
  { id: "claude", label: "Claude", cx: 364, cy: 200 },
  { id: "codex", label: "Codex", cx: 200, cy: 364 },
  { id: "gemini", label: "Gemini", cx: 36, cy: 200 },
] as const;

/** Abstract Hub with four harness nodes. Not a desktop screenshot. */
export function ArchitectureGraphic({ className = "" }: { className?: string }) {
  return (
    <figure className={className} aria-labelledby="arch-graphic-caption">
      <svg
        viewBox="0 0 400 400"
        className="h-full w-full"
        role="img"
        aria-label="Hub at the center connected to Grok, Claude, Codex, and Gemini"
      >
        <defs>
          <radialGradient id="hub-glow" cx="50%" cy="50%" r="50%">
            <stop offset="0%" stopColor="#6366f1" stopOpacity="0.35" />
            <stop offset="55%" stopColor="#a855f7" stopOpacity="0.12" />
            <stop offset="100%" stopColor="#020617" stopOpacity="0" />
          </radialGradient>
        </defs>
        <circle cx="200" cy="200" r="170" fill="url(#hub-glow)" className="motion-safe:animate-pulse" />
        {AGENTS.map((agent) => (
          <line
            key={`${agent.id}-link`}
            x1="200"
            y1="200"
            x2={agent.cx}
            y2={agent.cy}
            stroke="rgba(165, 85, 247, 0.45)"
            strokeWidth="2"
            strokeDasharray="5 6"
          />
        ))}
        <circle cx="200" cy="200" r="42" fill="var(--glass-bg)" stroke="#6366f1" strokeWidth="2.5" />
        <text
          x="200"
          y="204"
          textAnchor="middle"
          fill="var(--text-primary)"
          fontSize="14"
          fontWeight="700"
          fontFamily="Inter, system-ui, sans-serif"
        >
          Hub
        </text>
        {AGENTS.map((agent) => (
          <g key={agent.id}>
            <circle cx={agent.cx} cy={agent.cy} r="28" fill="var(--glass-bg)" stroke="#a855f7" strokeWidth="2" />
            <text
              x={agent.cx}
              y={agent.cy + 4}
              textAnchor="middle"
              fill="var(--text-primary)"
              fontSize="11"
              fontWeight="600"
              fontFamily="Inter, system-ui, sans-serif"
            >
              {agent.label}
            </text>
          </g>
        ))}
      </svg>
      <figcaption id="arch-graphic-caption" className="sr-only">
        Local hub in the center, with Grok, Claude, Codex, and Gemini as connected agents.
      </figcaption>
    </figure>
  );
}
