import type { DetectedProcess } from "./types";
import { processTargetId } from "./types";

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
            display: "flex",
            flexDirection: "column",
            gap: "0.65rem",
            padding: "1rem",
            marginBottom: "1.5rem",
            border: "1px solid var(--border-color)",
            borderRadius: "10px",
            background: "rgba(168, 85, 247, 0.06)",
          }}
        >
          <div style={{ color: "var(--text-muted)", fontSize: "0.8rem" }}>
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
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  gap: "1rem",
                  flexWrap: "wrap",
                  padding: "0.65rem 0.75rem",
                  borderRadius: "8px",
                  background: "rgba(0,0,0,0.25)",
                }}
              >
                <div style={{ minWidth: 0 }}>
                  <strong style={{ color: "var(--primary)" }}>{process.agent}</strong>
                  <span style={{ color: "var(--text-muted)", marginLeft: "0.6rem", fontSize: "0.8rem" }}>
                    PID {process.pid}
                  </span>
                  <div
                    style={{
                      color: "var(--text-muted)",
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
                  className={isAdded ? "btn-secondary" : "btn-primary"}
                  onClick={() => (isAdded ? onRemoveProcess(process) : onAddProcess(process))}
                >
                  {isAdded ? "Remove from team" : "Add to team"}
                </button>
              </div>
            );
          })}
          {detectedProcesses.length === 0 && (
            <span style={{ color: "var(--text-muted)", fontSize: "0.85rem" }}>
              No supported agent processes found.
            </span>
          )}
        </div>
      )}
    </>
  );
}
