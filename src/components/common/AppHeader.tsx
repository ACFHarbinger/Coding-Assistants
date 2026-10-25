import { workspaceBasename } from "../../app/hubState";
import { HudStatusPill } from "./telemetry/status";
import { NOC_COLORS, NOC_MOTION, NOC_RADIUS, NOC_SURFACES } from "./telemetry/tokens";

export type MainView = "orchestrate" | "messager" | "operations" | "work";

export interface AppHeaderProps {
  mainView: MainView;
  onSelectView: (view: MainView) => void;
  workspacePath: string;
  activeWorkSessionName: string | null;
  onOpenSettings: () => void;
  hubStatus?: string;
  pendingGatesWakes?: number;
}

const NAV_ITEMS: { id: MainView; label: string }[] = [
  { id: "messager", label: "💬 Chat & Memory" },
  { id: "orchestrate", label: "Orchestrate" },
  { id: "operations", label: "Operations" },
  { id: "work", label: "Work" },
];

/**
 * AppHeader — Deep-Space NOC Cockpit Header (V3 / #374).
 *
 * Provides:
 * 1. Segmented HUD mode switcher (Chat & Memory, Orchestrate, Operations, Work) with active glow indicator.
 * 2. Standardized <HudStatusPill> badges for Workspace Root (basename visible, full path in title & focus),
 *    Active Team Session, and Pending Gates/Wakes.
 * 3. Settings modal trigger button.
 * 4. Local Hub online telemetry badge.
 * 5. Responsive 2-row wrapping at <= 1024px without page-level scroll (CODEX-UI-2).
 * 6. Zero inline backdrop-filter to preserve #213 containing-block performance safety.
 */
export function AppHeader({
  mainView,
  onSelectView,
  workspacePath,
  activeWorkSessionName,
  onOpenSettings,
  hubStatus = "Local hub online",
  pendingGatesWakes,
}: AppHeaderProps) {
  const baseName = workspacePath ? workspaceBasename(workspacePath) : "Not set";
  const workspaceTitle = workspacePath
    ? `${workspacePath} (Click to configure)`
    : "Workspace root is not set (Click to configure)";
  const sessionTitle = activeWorkSessionName
    ? `Active: ${activeWorkSessionName} (Click to open Chat)`
    : "No active team chat (Click to open Chat)";

  return (
    <header
      role="banner"
      style={{
        padding: "0.85rem 1.75rem",
        borderBottom: `1px solid ${NOC_SURFACES.border}`,
        background: "rgba(2, 6, 23, 0.88)",
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        flexWrap: "wrap",
        gap: "0.75rem",
        zIndex: 10,
        minWidth: 0,
      }}
    >
      {/* Brand title + Context HUD pills */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: "1rem",
          flexWrap: "wrap",
          minWidth: 0,
        }}
      >
        <h1
          style={{
            margin: 0,
            fontSize: "1.4rem",
            fontWeight: 800,
            letterSpacing: "-0.025em",
            background: "linear-gradient(to right, #fff, var(--text-muted))",
            WebkitBackgroundClip: "text",
            WebkitTextFillColor: "transparent",
            whiteSpace: "nowrap",
          }}
        >
          Coding Assistants
        </h1>

        <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", flexWrap: "wrap", minWidth: 0 }}>
          <HudStatusPill
            label="Workspace root"
            value={baseName}
            title={workspaceTitle}
            tone={workspacePath ? "ok" : "idle"}
            onClick={() => onSelectView("orchestrate")}
            maxWidth={240}
            testId="hud-pill-workspace"
          />

          <HudStatusPill
            label="Active team chat"
            value={activeWorkSessionName || "None selected"}
            title={sessionTitle}
            tone={activeWorkSessionName ? "cyan" : "idle"}
            onClick={() => onSelectView("messager")}
            maxWidth={200}
            testId="hud-pill-session"
          />

          {typeof pendingGatesWakes === "number" && pendingGatesWakes > 0 && (
            <HudStatusPill
              label="Pending Actions"
              value={`${pendingGatesWakes} gate${pendingGatesWakes === 1 ? "" : "s"}`}
              title={`${pendingGatesWakes} pending gate/wake approval`}
              tone="warn"
              onClick={() => onSelectView("operations")}
              maxWidth={180}
              testId="hud-pill-gates"
            />
          )}
        </div>
      </div>

      {/* Mode switcher + Settings + Hub status */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: "0.6rem",
          flexWrap: "wrap",
        }}
      >
        <nav
          role="tablist"
          aria-label="Main application view"
          style={{
            display: "flex",
            alignItems: "center",
            gap: "0.35rem",
            background: "rgba(9, 14, 26, 0.72)",
            border: `1px solid ${NOC_SURFACES.border}`,
            padding: "0.22rem",
            borderRadius: NOC_RADIUS.m,
            flexWrap: "wrap",
          }}
        >
          {NAV_ITEMS.map((item) => {
            const isActive = mainView === item.id;
            return (
              <button
                key={item.id}
                type="button"
                role="tab"
                aria-selected={isActive}
                className={`${isActive ? "btn-primary" : "btn-secondary"} noc-focusable`}
                style={{
                  marginTop: 0,
                  padding: "0.45rem 0.95rem",
                  fontSize: "0.88rem",
                  borderRadius: NOC_RADIUS.s,
                  position: "relative",
                  display: "inline-flex",
                  alignItems: "center",
                  gap: "0.35rem",
                  boxShadow: isActive ? `0 0 14px rgba(56, 189, 248, 0.22)` : "none",
                  transition: `opacity ${NOC_MOTION.durFast} ${NOC_MOTION.easeNoc}, transform ${NOC_MOTION.durFast} ${NOC_MOTION.easeNoc}`,
                }}
                onClick={() => onSelectView(item.id)}
              >
                <span>{item.label}</span>
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
        </nav>

        <button
          type="button"
          className="btn-secondary noc-focusable"
          style={{
            marginTop: 0,
            padding: "0.48rem 0.9rem",
            fontSize: "0.88rem",
            borderRadius: NOC_RADIUS.s,
            display: "inline-flex",
            alignItems: "center",
            gap: "0.45rem",
          }}
          onClick={onOpenSettings}
          aria-haspopup="dialog"
          title="Open Settings in its own window"
        >
          <svg
            aria-hidden="true"
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <circle cx="12" cy="12" r="3" />
            <path d="M19.4 15a1.7 1.7 0 0 0 .34 1.88l.06.06-2.12 2.12-.06-.06a1.7 1.7 0 0 0-1.88-.34 1.7 1.7 0 0 0-1.04 1.56V20.3h-3v-.08A1.7 1.7 0 0 0 10.66 18.7a1.7 1.7 0 0 0-1.88.34l-.06.06-2.12-2.12.06-.06A1.7 1.7 0 0 0 7 15.04a1.7 1.7 0 0 0-1.56-1.04h-.08v-3h.08A1.7 1.7 0 0 0 7 9.96a1.7 1.7 0 0 0-.34-1.88L6.6 8.02 8.72 5.9l.06.06a1.7 1.7 0 0 0 1.88.34A1.7 1.7 0 0 0 11.7 4.74v-.08h3v.08a1.7 1.7 0 0 0 1.04 1.56 1.7 1.7 0 0 0 1.88-.34l.06-.06 2.12 2.12-.06.06a1.7 1.7 0 0 0-.34 1.88A1.7 1.7 0 0 0 20.96 11h.08v3h-.08A1.7 1.7 0 0 0 19.4 15Z" />
          </svg>
          <span>Settings</span>
        </button>

        <div
          className="status-badge"
          data-testid="hud-status-hub"
          style={{
            marginLeft: "0.25rem",
            padding: "0.4rem 0.8rem",
            background: "rgba(16, 185, 129, 0.15)",
            color: "#6ee7b7",
            borderRadius: "20px",
            fontSize: "0.74rem",
            fontWeight: 600,
            border: "1px solid rgba(16, 185, 129, 0.3)",
            whiteSpace: "nowrap",
          }}
        >
          {hubStatus}
        </div>
      </div>
    </header>
  );
}
export default AppHeader;
