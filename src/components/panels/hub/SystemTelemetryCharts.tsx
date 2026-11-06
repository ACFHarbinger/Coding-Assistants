import {
  type TelemetrySeries,
  TelemetryTimeSeriesCard,
} from "../../common/telemetry/charts";
import { NOC_COLORS, NOC_SURFACES } from "../../common/telemetry/tokens";
import {
  type DiskInfo,
  formatBytes,
  formatRate,
  type GpuStatus,
  Meter,
  nocPanelStyle,
  type WatchedProcess,
} from "./SystemGauges";

export { type TelemetrySeries, TelemetryTimeSeriesCard };

/**
 * Bottom NOC detail panels for Disks, GPU, Managed Harness Processes, and Top CPU Processes.
 */
export function SystemDetailPanels({
  disks,
  ioRates,
  gpu,
  gpuFallbackDetail,
  harnessProcesses,
  topProcesses,
}: {
  disks: DiskInfo[];
  ioRates: Record<string, { read: number; write: number }>;
  gpu: GpuStatus | undefined;
  gpuFallbackDetail: string;
  harnessProcesses: WatchedProcess[];
  topProcesses: WatchedProcess[];
}) {
  return (
    <>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(300px, 1fr))", gap: "1rem" }}>
        <div style={nocPanelStyle}>
          <h3 style={{ margin: "0 0 0.85rem", fontSize: "0.95rem", color: "var(--text-main)" }}>Disks</h3>
          <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
            {disks.map((disk) => {
              const used = disk.total_bytes - disk.available_bytes;
              const percent = disk.total_bytes > 0 ? (used / disk.total_bytes) * 100 : 0;
              const rate = ioRates[disk.mount_point];
              return (
                <div key={disk.mount_point}>
                  <div style={{ display: "flex", justifyContent: "space-between", fontSize: "0.8rem", color: "var(--text-muted)", flexWrap: "wrap", gap: "0.5rem" }}>
                    <span>
                      <strong style={{ color: "var(--text-main)" }}>{disk.mount_point}</strong> · {disk.file_system}
                      {disk.name ? ` · ${disk.name}` : ""}
                    </span>
                    <span className="tabular-nums">
                      {formatBytes(used)} / {formatBytes(disk.total_bytes)} · ↓ {formatRate(rate?.read ?? NaN)} ↑ {formatRate(rate?.write ?? NaN)}
                    </span>
                  </div>
                  <div style={{ height: 7, background: NOC_SURFACES.gridline, borderRadius: 4, marginTop: "0.3rem" }}>
                    <div
                      style={{
                        width: `${Math.min(100, percent)}%`,
                        height: "100%",
                        borderRadius: 4,
                        background: percent > 90 ? NOC_COLORS.crit : NOC_COLORS.ok,
                      }}
                    />
                  </div>
                </div>
              );
            })}
            {disks.length === 0 && (
              <span style={{ color: "var(--text-muted)", fontSize: "0.85rem" }}>No volumes reported yet.</span>
            )}
          </div>
        </div>

        <div style={nocPanelStyle}>
          <h3 style={{ margin: "0 0 0.85rem", fontSize: "0.95rem", color: "var(--text-main)" }}>GPU</h3>
          {gpu?.available ? (
            <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
              {gpu.gpus.map((g) => (
                <Meter
                  key={g.index}
                  label={`${g.name} · VRAM`}
                  color={NOC_COLORS.purple}
                  percent={g.mem_total_bytes > 0 ? (g.mem_used_bytes / g.mem_total_bytes) * 100 : 0}
                  detail={`core ${g.usage_percent.toFixed(0)}% · ${formatBytes(g.mem_used_bytes)} / ${formatBytes(g.mem_total_bytes)}`}
                />
              ))}
            </div>
          ) : (
            <span style={{ color: "var(--text-muted)", fontSize: "0.85rem" }}>{gpuFallbackDetail}</span>
          )}
        </div>
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(300px, 1fr))", gap: "1rem" }}>
        <div style={nocPanelStyle}>
          <h3 style={{ margin: "0 0 0.85rem", fontSize: "0.95rem", color: "var(--text-main)" }}>Managed harness processes</h3>
          {harnessProcesses.length === 0 && (
            <span style={{ color: "var(--text-muted)", fontSize: "0.85rem" }}>No managed harness processes running.</span>
          )}
          {harnessProcesses.map((proc) => (
            <div
              key={proc.pid}
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                gap: "1rem",
                borderTop: "1px solid var(--border-color)",
                padding: "0.55rem 0",
                fontSize: "0.84rem",
                flexWrap: "wrap",
              }}
            >
              <span style={{ color: "var(--text-main)", display: "inline-flex", alignItems: "center", gap: "0.45rem" }}>
                <i style={{ width: 7, height: 7, borderRadius: "50%", background: NOC_COLORS.emerald, display: "inline-block" }} />
                <strong>{proc.harness}</strong> · {proc.name}{" "}
                <span style={{ color: "var(--text-muted)" }}>pid {proc.pid}</span>
              </span>
              <span className="tabular-nums" style={{ color: "var(--text-muted)" }}>
                CPU {proc.cpu_percent.toFixed(1)}% · {formatBytes(proc.mem_bytes)}
              </span>
            </div>
          ))}
        </div>

        <div style={nocPanelStyle}>
          <h3 style={{ margin: "0 0 0.85rem", fontSize: "0.95rem", color: "var(--text-main)" }}>Top processes by CPU</h3>
          {topProcesses.map((proc) => (
            <div
              key={proc.pid}
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                gap: "1rem",
                borderTop: "1px solid var(--border-color)",
                padding: "0.55rem 0",
                fontSize: "0.84rem",
                flexWrap: "wrap",
              }}
            >
              <span style={{ color: "var(--text-main)" }}>
                {proc.name} <span style={{ color: "var(--text-muted)" }}>pid {proc.pid}</span>
              </span>
              <span className="tabular-nums" style={{ color: "var(--text-muted)" }}>
                CPU {proc.cpu_percent.toFixed(1)}% · {formatBytes(proc.mem_bytes)}
              </span>
            </div>
          ))}
        </div>
      </div>
    </>
  );
}
