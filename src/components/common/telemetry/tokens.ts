/**
 * Paired NOC Telemetry Design Tokens (#372 / V1).
 *
 * These constants mirror the `:root` `--noc-*` custom properties in `src/index.css`.
 * Both files move together and are guarded by `src/components/common/telemetry/__tests__/tokens.test.ts`
 * for 1:1 CSS pairing and WCAG AA (>= 4.5:1) contrast against NOC panel surfaces.
 */

export const NOC_COLORS = {
  ok: "#10b981",
  warn: "#eab308",
  crit: "#ef4444",
  idle: "#94a3b8",
  cyan: "#38bdf8",
  teal: "#06b6d4",
  indigo: "#6366f1",
  purple: "#a855f7",
  pink: "#ec4899",
  amber: "#f59e0b",
  orange: "#f97316",
  emerald: "#22c55e",
} as const;

export const NOC_SURFACES = {
  bgPanel: "#0f172a",
  bgDeep: "#090e1a",
  textPrimary: "#f8fafc",
  textSecondary: "#e2e8f0",
  textMuted: "#94a3b8",
  border: "rgba(148, 163, 184, 0.14)",
  borderStrong: "rgba(148, 163, 184, 0.28)",
  gridline: "rgba(148, 163, 184, 0.14)",
  gridlineSubtle: "rgba(148, 163, 184, 0.08)",
  track: "rgba(148, 163, 184, 0.16)",
  focusRing: "0 0 0 2px #090e1a, 0 0 0 4px #38bdf8",
} as const;

export const NOC_TYPOGRAPHY = {
  kpi: "1.5rem",
  body: "0.85rem",
  micro: "0.72rem",
} as const;

export const NOC_SPACING = {
  space1: "4px",
  space2: "8px",
  space3: "12px",
  space4: "16px",
  space5: "24px",
  space6: "32px",
} as const;

export const NOC_ELEVATION = {
  elev1: "none",
  elev2: "0 8px 24px rgba(0, 0, 0, 0.35)",
  elev3: "0 20px 40px rgba(0, 0, 0, 0.55)",
} as const;

export const NOC_RADIUS = {
  s: "8px",
  m: "12px",
  l: "16px",
} as const;

export const NOC_MOTION = {
  durInstant: "100ms",
  durFast: "160ms",
  durSlow: "240ms",
  easeNoc: "cubic-bezier(0.2, 0, 0, 1)",
} as const;

export const NOC_CSS_VAR_MAP: Record<string, string> = {
  "--noc-ok": NOC_COLORS.ok,
  "--noc-warn": NOC_COLORS.warn,
  "--noc-crit": NOC_COLORS.crit,
  "--noc-idle": NOC_COLORS.idle,
  "--noc-cyan": NOC_COLORS.cyan,
  "--noc-teal": NOC_COLORS.teal,
  "--noc-indigo": NOC_COLORS.indigo,
  "--noc-purple": NOC_COLORS.purple,
  "--noc-pink": NOC_COLORS.pink,
  "--noc-amber": NOC_COLORS.amber,
  "--noc-orange": NOC_COLORS.orange,
  "--noc-emerald": NOC_COLORS.emerald,
  "--noc-bg-panel": NOC_SURFACES.bgPanel,
  "--noc-bg-deep": NOC_SURFACES.bgDeep,
  "--noc-text-primary": NOC_SURFACES.textPrimary,
  "--noc-text-secondary": NOC_SURFACES.textSecondary,
  "--noc-text-muted": NOC_SURFACES.textMuted,
  "--noc-border": NOC_SURFACES.border,
  "--noc-border-strong": NOC_SURFACES.borderStrong,
  "--noc-gridline": NOC_SURFACES.gridline,
  "--noc-gridline-subtle": NOC_SURFACES.gridlineSubtle,
  "--noc-track": NOC_SURFACES.track,
  "--noc-focus-ring": NOC_SURFACES.focusRing,
  "--noc-type-kpi": NOC_TYPOGRAPHY.kpi,
  "--noc-type-body": NOC_TYPOGRAPHY.body,
  "--noc-type-micro": NOC_TYPOGRAPHY.micro,
  "--space-1": NOC_SPACING.space1,
  "--space-2": NOC_SPACING.space2,
  "--space-3": NOC_SPACING.space3,
  "--space-4": NOC_SPACING.space4,
  "--space-5": NOC_SPACING.space5,
  "--space-6": NOC_SPACING.space6,
  "--elev-1": NOC_ELEVATION.elev1,
  "--elev-2": NOC_ELEVATION.elev2,
  "--elev-3": NOC_ELEVATION.elev3,
  "--radius-s": NOC_RADIUS.s,
  "--radius-m": NOC_RADIUS.m,
  "--radius-l": NOC_RADIUS.l,
  "--dur-instant": NOC_MOTION.durInstant,
  "--dur-fast": NOC_MOTION.durFast,
  "--dur-slow": NOC_MOTION.durSlow,
  "--ease-noc": NOC_MOTION.easeNoc,
};

export type NocTone = "ok" | "warn" | "crit" | "idle" | "cyan" | "indigo" | "purple" | "pink" | "amber";

export function resolveNocToneColor(tone: NocTone): string {
  return NOC_COLORS[tone];
}
