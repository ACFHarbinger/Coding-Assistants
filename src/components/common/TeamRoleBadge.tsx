import React from "react";

export interface TeamRoleBadgeProps {
  role?: string | null;
  size?: "small" | "normal";
  style?: React.CSSProperties;
  className?: string;
  title?: string;
  onClear?: () => void;
}

export const ROLE_PRESETS = [
  { value: "lead", label: "Team Lead" },
  { value: "reviewer", label: "Reviewer" },
  { value: "implementer", label: "Implementer" },
  { value: "observer", label: "Observer" },
] as const;

interface RoleVisual {
  label: string;
  color: string;
  bg: string;
  border: string;
  icon: string;
}

export function getRoleVisual(rawRole: string): RoleVisual {
  const normalized = rawRole.trim().toLowerCase();

  if (normalized === "lead" || normalized === "team lead" || normalized === "team_lead") {
    return {
      label: rawRole.trim().toLowerCase() === "lead" ? "Team Lead" : rawRole.trim(),
      color: "#fef08a",
      bg: "rgba(234, 179, 8, 0.15)",
      border: "rgba(234, 179, 8, 0.45)",
      icon: "★",
    };
  }

  if (normalized === "reviewer" || normalized === "review lead" || normalized === "review_lead" || normalized === "review") {
    return {
      label: rawRole.trim().toLowerCase() === "reviewer" ? "Reviewer" : rawRole.trim(),
      color: "#e9d5ff",
      bg: "rgba(168, 85, 247, 0.15)",
      border: "rgba(168, 85, 247, 0.45)",
      icon: "✓",
    };
  }

  if (normalized === "implementer" || normalized === "developer" || normalized === "coder") {
    return {
      label: rawRole.trim().toLowerCase() === "implementer" ? "Implementer" : rawRole.trim(),
      color: "#a7f3d0",
      bg: "rgba(16, 185, 129, 0.15)",
      border: "rgba(16, 185, 129, 0.45)",
      icon: "⚡",
    };
  }

  if (normalized === "observer" || normalized === "monitor") {
    return {
      label: rawRole.trim().toLowerCase() === "observer" ? "Observer" : rawRole.trim(),
      color: "#cbd5e1",
      bg: "rgba(148, 163, 184, 0.15)",
      border: "rgba(148, 163, 184, 0.4)",
      icon: "◉",
    };
  }

  // Custom role
  return {
    label: rawRole.trim(),
    color: "#bae6fd",
    bg: "rgba(56, 189, 248, 0.15)",
    border: "rgba(56, 189, 248, 0.4)",
    icon: "◈",
  };
}

export function TeamRoleBadge({
  role,
  size = "small",
  style,
  className,
  title,
  onClear,
}: TeamRoleBadgeProps) {
  if (!role || !role.trim()) return null;

  const visual = getRoleVisual(role);
  const isSmall = size === "small";

  return (
    <span
      className={className}
      title={title ?? `Role: ${visual.label}`}
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: "0.25rem",
        padding: isSmall ? "0.1rem 0.4rem" : "0.2rem 0.55rem",
        fontSize: isSmall ? "0.7rem" : "0.78rem",
        fontWeight: 600,
        lineHeight: 1.2,
        borderRadius: "9999px",
        color: visual.color,
        background: visual.bg,
        border: `1px solid ${visual.border}`,
        whiteSpace: "nowrap",
        flexShrink: 0,
        verticalAlign: "middle",
        userSelect: "none",
        ...style,
      }}
    >
      <span style={{ fontSize: isSmall ? "0.65rem" : "0.75rem", opacity: 0.9 }}>
        {visual.icon}
      </span>
      <span>{visual.label}</span>
      {onClear && (
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onClear();
          }}
          title={`Clear ${visual.label} role`}
          aria-label={`Clear ${visual.label} role`}
          style={{
            background: "none",
            border: "none",
            color: visual.color,
            opacity: 0.7,
            cursor: "pointer",
            padding: 0,
            marginLeft: "0.15rem",
            fontSize: "0.75rem",
            lineHeight: 1,
            display: "inline-flex",
            alignItems: "center",
          }}
        >
          ×
        </button>
      )}
    </span>
  );
}

export default TeamRoleBadge;
