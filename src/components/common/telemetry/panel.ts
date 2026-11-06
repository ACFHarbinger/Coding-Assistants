import type React from "react";
import { NOC_ELEVATION, NOC_RADIUS, NOC_SPACING, NOC_SURFACES } from "./tokens";

export const nocPanelStyle: React.CSSProperties = {
  background: "linear-gradient(180deg, rgba(15, 23, 42, 0.92) 0%, rgba(9, 14, 26, 0.96) 100%)",
  border: `1px solid ${NOC_SURFACES.border}`,
  borderRadius: "10px",
  padding: "1rem 1.15rem",
  boxShadow: NOC_ELEVATION.elev2,
};

export const nocCardDenseStyle: React.CSSProperties = {
  background: "rgba(15, 23, 42, 0.72)",
  border: `1px solid ${NOC_SURFACES.border}`,
  borderRadius: NOC_RADIUS.s,
  padding: `${NOC_SPACING.space2} ${NOC_SPACING.space3}`,
};

export const nocSubsurfaceStyle: React.CSSProperties = {
  background: "rgba(9, 14, 26, 0.65)",
  border: `1px solid ${NOC_SURFACES.border}`,
  borderRadius: NOC_RADIUS.s,
  padding: `${NOC_SPACING.space3} ${NOC_SPACING.space4}`,
};
