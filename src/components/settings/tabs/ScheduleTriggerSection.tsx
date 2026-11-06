import { useCallback, useEffect, useState } from "react";
import {
  getScheduleTriggerStatus,
  installScheduleTrigger,
  removeScheduleTrigger,
  type ScheduleTriggerStatus,
} from "./scheduleTriggerApi";
import { inputStyle, shortenPath } from "./shared";

const DEFAULT_INTERVAL_MINUTES = 5;

function Chip({ label, tone }: { label: string; tone: "ok" | "off" | "warn" }) {
  const color =
    tone === "ok" ? "#86efac" : tone === "warn" ? "#fca5a5" : "var(--text-muted)";
  const border =
    tone === "ok"
      ? "rgba(134, 239, 172, 0.45)"
      : tone === "warn"
        ? "rgba(252, 165, 165, 0.45)"
        : "var(--border-color)";
  return (
    <span
      style={{
        fontSize: "0.7rem",
        fontWeight: 600,
        padding: "0.15rem 0.55rem",
        borderRadius: "999px",
        border: `1px solid ${border}`,
        color,
        background: "rgba(255,255,255,0.03)",
        whiteSpace: "nowrap",
      }}
    >
      {label}
    </span>
  );
}

export interface ScheduleTriggerSectionProps {
  workspaceRoot: string | null;
  busy: boolean;
}

export default function ScheduleTriggerSection({
  workspaceRoot,
  busy,
}: ScheduleTriggerSectionProps) {
  const [status, setStatus] = useState<ScheduleTriggerStatus | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [intervalDraft, setIntervalDraft] = useState(String(DEFAULT_INTERVAL_MINUTES));
  const [caBinDraft, setCaBinDraft] = useState("");

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setStatus(await getScheduleTriggerStatus());
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const handleInstall = async () => {
    if (!workspaceRoot) return;
    const interval = Number(intervalDraft);
    if (!Number.isInteger(interval) || interval < 1) {
      setError("Interval must be a whole number of minutes, at least 1.");
      return;
    }
    setLoading(true);
    setError(null);
    setNotice(null);
    try {
      const next = await installScheduleTrigger({
        ca_bin: caBinDraft.trim() === "" ? null : caBinDraft.trim(),
        interval_minutes: interval,
        workspace: workspaceRoot,
      });
      setStatus(next);
      setNotice(next.enabled ? "Timer installed and enabled." : "Units written; timer not enabled.");
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleRemove = async () => {
    setLoading(true);
    setError(null);
    setNotice(null);
    try {
      const next = await removeScheduleTrigger();
      setStatus(next);
      setNotice("Timer disabled and unit files removed.");
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const disabled = busy || loading;
  const installed = status?.installed ?? false;
  const available = status?.available ?? false;

  return (
    <div
      style={{
        marginTop: "1.5rem",
        paddingTop: "1.1rem",
        borderTop: "1px solid var(--border-color)",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: "0.6rem", flexWrap: "wrap" }}>
        <label style={{ fontWeight: 600, fontSize: "0.9rem" }}>
          Headless trigger (Linux systemd user timer)
        </label>
        {status && (
          <>
            <Chip label={available ? "systemd available" : "systemd unavailable"} tone={available ? "ok" : "warn"} />
            <Chip label={installed ? "installed" : "not installed"} tone={installed ? "ok" : "off"} />
            {installed && <Chip label={status.enabled ? "enabled" : "disabled"} tone={status.enabled ? "ok" : "off"} />}
            {installed && <Chip label={status.active ? "active" : "inactive"} tone={status.active ? "ok" : "off"} />}
          </>
        )}
        <button
          type="button"
          className="btn-secondary"
          style={{ marginTop: 0, padding: "0.25rem 0.6rem", fontSize: "0.72rem" }}
          disabled={disabled}
          onClick={() => void refresh()}
        >
          Refresh
        </button>
      </div>
      <p style={{ margin: "0.45rem 0 0.7rem", color: "var(--text-muted)", fontSize: "0.8rem", lineHeight: 1.5 }}>
        Installs user-owned <code>ca-schedule.service</code> + <code>ca-schedule.timer</code> under
        <code> ~/.config/systemd/user</code>. The service runs only{" "}
        <code>ca schedule tick</code> with explicit argv — no privileged unit and no arbitrary action
        scheduler. Headless ticks enforce the in-hub credit allowance only; live provider-quota
        pauses still require the app to be open.
      </p>

      {status?.detail && (
        <p style={{ margin: "0 0 0.7rem", color: "#fca5a5", fontSize: "0.78rem", lineHeight: 1.5 }}>
          {status.detail}
        </p>
      )}

      {installed && status && (
        <div style={{ fontSize: "0.75rem", color: "var(--text-muted)", marginBottom: "0.7rem" }}>
          <div>Timer: {shortenPath(status.timer_path, 60)}</div>
          <div>Service: {shortenPath(status.service_path, 60)}</div>
          {status.exec_start && (
            <div>
              Runs: <code>{shortenPath(status.exec_start, 70)}</code>
            </div>
          )}
        </div>
      )}

      {!workspaceRoot && (
        <p style={{ margin: "0 0 0.7rem", color: "#fca5a5", fontSize: "0.8rem" }}>
          Select an active workspace in Orchestrate before installing the trigger.
        </p>
      )}

      <div style={{ display: "flex", gap: "0.5rem", flexWrap: "wrap", alignItems: "center" }}>
        <label style={{ fontSize: "0.78rem", color: "var(--text-muted)" }}>
          Every
          <input
            type="number"
            min={1}
            step={1}
            style={{ ...inputStyle, flex: "0 0 90px", minWidth: "70px", marginLeft: "0.4rem" }}
            value={intervalDraft}
            onChange={(event) => setIntervalDraft(event.target.value)}
            disabled={disabled}
          />
          min
        </label>
        <input
          style={{ ...inputStyle, flex: "1 1 240px" }}
          placeholder="ca binary (blank = auto-detect on PATH)"
          value={caBinDraft}
          onChange={(event) => setCaBinDraft(event.target.value)}
          disabled={disabled}
        />
        <button
          type="button"
          className="btn-primary"
          style={{ marginTop: 0 }}
          disabled={disabled || !workspaceRoot || !available}
          onClick={() => void handleInstall()}
        >
          {installed ? "Update trigger" : "Install trigger"}
        </button>
        {installed && (
          <button
            type="button"
            className="btn-secondary"
            style={{ marginTop: 0 }}
            disabled={disabled}
            onClick={() => void handleRemove()}
          >
            Remove trigger
          </button>
        )}
      </div>

      {notice && <p style={{ margin: "0.6rem 0 0", color: "#86efac", fontSize: "0.78rem" }}>{notice}</p>}
      {error && <p style={{ margin: "0.6rem 0 0", color: "#fca5a5", fontSize: "0.78rem" }}>{error}</p>}
    </div>
  );
}
