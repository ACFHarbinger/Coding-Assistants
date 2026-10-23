import { NOC_COLORS, NOC_MOTION, NOC_RADIUS, NOC_SURFACES } from "./tokens";

export type SubtabItem<T extends string> = {
  id: T;
  label: string;
  badgeCount?: number;
};

/**
 * Shared NOC subtab switcher with accessible selected-state semantics,
 * `:focus-visible` ring, and an active indicator (`MISTRAL-UI-3` / `CODEX-UI-4`).
 */
export function SubtabBar<T extends string>({
  tabs,
  activeId,
  onSelect,
  ariaLabel = "Panel subtabs",
}: {
  tabs: SubtabItem<T>[];
  activeId: T;
  onSelect: (id: T) => void;
  ariaLabel?: string;
}) {
  return (
    <div
      role="group"
      aria-label={ariaLabel}
      style={{
        display: "flex",
        gap: "0.45rem",
        background: "rgba(9, 14, 26, 0.72)",
        border: `1px solid ${NOC_SURFACES.border}`,
        padding: "0.28rem",
        borderRadius: NOC_RADIUS.m,
        flexWrap: "wrap",
      }}
    >
      {tabs.map((tab) => {
        const isActive = tab.id === activeId;
        return (
          <button
            key={tab.id}
            type="button"
            aria-pressed={isActive}
            className={`${isActive ? "btn-primary" : "btn-secondary"} noc-focusable`}
            style={{
              marginTop: 0,
              padding: "0.48rem 0.95rem",
              fontSize: "0.88rem",
              borderRadius: NOC_RADIUS.s,
              position: "relative",
              display: "inline-flex",
              alignItems: "center",
              gap: "0.4rem",
              transition: `opacity ${NOC_MOTION.durFast} ${NOC_MOTION.easeNoc}, transform ${NOC_MOTION.durFast} ${NOC_MOTION.easeNoc}`,
            }}
            onClick={() => onSelect(tab.id)}
          >
            <span>{tab.label}</span>
            {typeof tab.badgeCount === "number" && tab.badgeCount > 0 && (
              <span
                className="tabular-nums"
                style={{
                  fontSize: "0.68rem",
                  fontWeight: 700,
                  padding: "0.08rem 0.38rem",
                  borderRadius: "9999px",
                  background: isActive ? "rgba(9, 14, 26, 0.45)" : "rgba(56, 189, 248, 0.16)",
                  color: isActive ? NOC_SURFACES.textPrimary : NOC_COLORS.cyan,
                }}
              >
                {tab.badgeCount}
              </span>
            )}
            {isActive && (
              <span
                className="noc-subtab-indicator"
                aria-hidden="true"
                style={{
                  position: "absolute",
                  left: "18%",
                  right: "18%",
                  bottom: 2,
                  height: 2,
                  borderRadius: 2,
                  background: NOC_COLORS.cyan,
                }}
              />
            )}
          </button>
        );
      })}
    </div>
  );
}
