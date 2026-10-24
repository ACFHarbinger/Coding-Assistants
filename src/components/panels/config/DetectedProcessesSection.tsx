import type { DetectedProcess } from "./types";
import { processTargetId } from "./types";
import { nocCardDenseStyle, nocPanelStyle } from "../../common/telemetry/panel";
import { EmptyStateChip } from "../../common/telemetry/status";
import { NOC_COLORS, NOC_SURFACES } from "../../common/telemetry/tokens";

interface DetectedProcessesSectionProps {
  hasScanned: boolean;
  detectError: string;
  detectedProcesses: DetectedProcess[];
  addedPids: number[];
  teamMemberIds: string[];
  onAddProcess: (process: DetectedProcess) => void;
  onRemoveProcess: (process: DetectedProcess) => void;
}

export default function DetectedProcessesSection({
  hasScanned,
  detectError,
  detectedProcesses,
  addedPids,
  teamMemberIds,
  onAddProcess,
  onRemoveProcess,
}: DetectedProcessesSectionProps) {
  if (!hasScanned && !detectError) return null;

  return (
    <>
      {detectError && (
        <div style={{ color: "#ef4444", fontSize: "0.85rem", marginBottom: "1rem" }}>
          {detectError}
        </div>
      )}
      {hasScanned && (
        <div
          style={{
            ...nocPanelStyle,
            borderLeft: `3px solid ${NOC_COLORS.purple}`,
            display: "flex",
            flexDirection: "column",
            gap: "0.65rem",
            marginBottom: "1.25rem",
          }}
        >
          <div style={{ color: NOC_SURFACES.textMuted, fontSize: "0.8rem" }}>
            External agent binaries currently running anywhere on this machine (not workspace-scoped,
            not a live Chat &amp; Memory connection). Selecting one adds its identity to the team; it
            does not take ownership of or terminate the process. A task executes automatically only
            when that provider has a registered, supported active-session bridge.
          </div>
          {detectedProcesses.map((process) => {
            const isAdded =
              addedPids.includes(process.pid) || teamMemberIds.includes(processTargetId(process));
            return (
              <div
                key={process.pid}
                style={{
                  ...nocCardDenseStyle,
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  gap: "1rem",
                  flexWrap: "wrap",
                  padding: "0.65rem 0.85rem",
                }}
              >
                <div style={{ minWidth: 0 }}>
                  <strong style={{ color: NOC_COLORS.cyan }}>{process.agent}</strong>
                  <span className="tabular-nums" style={{ color: NOC_SURFACES.textMuted, marginLeft: "0.6rem", fontSize: "0.78rem", fontFamily: "var(--font-mono)" }}>
                    PID {process.pid}
                  </span>
                  <div
                    style={{
                      color: NOC_SURFACES.textMuted,
                      fontFamily: "var(--font-mono)",
                      fontSize: "0.75rem",
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap",
                      maxWidth: "min(65vw, 680px)",
                    }}
                  >
                    {process.command}
                  </div>
                </div>
                <button
                  className={isAdded ? "btn-secondary noc-focusable" : "btn-primary noc-focusable"}
                  style={{ marginTop: 0 }}
                  onClick={() => (isAdded ? onRemoveProcess(process) : onAddProcess(process))}
                >
                  {isAdded ? "Remove from team" : "Add to team"}
                </button>
              </div>
            );
          })}
          {detectedProcesses.length === 0 && (
            <EmptyStateChip title="No supported agent processes found" detail="Start an agent CLI or scan again after launching." tone="idle" />
          )}
        </div>
      )}
    </>
  );
}
