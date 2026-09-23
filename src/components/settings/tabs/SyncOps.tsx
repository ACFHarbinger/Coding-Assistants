import { useCallback, useEffect, useState } from "react";
import {
  hubSyncDevices,
  hubSyncDiagnostics,
  hubSyncLimits,
  hubSyncRetry,
  type Diagnostics,
  type SyncLimits,
  type TrustList,
} from "./syncApi";

const SECRET = /cloud-sync\.key|\btoken\b|refresh|ya29|Bearer/i;

function safeText(value: string): boolean {
  return !SECRET.test(value);
}

export default function SyncOps({
  disabled,
  onSession,
}: {
  disabled: boolean;
  onSession: (action: "retry") => Promise<void>;
}) {
  const [trust, setTrust] = useState<TrustList | null>(null);
  const [limits, setLimits] = useState<SyncLimits | null>(null);
  const [diagnostics, setDiagnostics] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    const [nextTrust, nextLimits] = await Promise.all([hubSyncDevices(), hubSyncLimits()]);
    setTrust(nextTrust);
    setLimits(nextLimits);
  }, []);

  useEffect(() => {
    void refresh().catch((err) => setError(String(err)));
  }, [refresh]);

  async function run(work: () => Promise<void>) {
    setBusy(true);
    setError(null);
    try {
      await work();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  const devices = (trust?.devices ?? []).filter(
    (row) => safeText(row.folder) && safeText(row.status),
  );
  const trusted = devices.filter((row) => row.status === "trusted").length;
  const revoked = devices.filter((row) => row.status === "revoked").length;

  return (
    <div style={{ marginTop: "1.25rem" }}>
      <p style={{ fontWeight: 600, fontSize: "0.9rem", marginBottom: "0.4rem" }}>
        Device trust
      </p>
      <p style={{ color: "var(--text-muted)", fontSize: "0.8rem", marginBottom: "0.75rem" }}>
        Trust list only — revoke does not rotate the replica key. Hashed folders
        shown. Owner retry is explicit.
      </p>
      <p style={{ fontSize: "0.85rem", marginBottom: "0.5rem" }}>
        Trusted {trusted} · Revoked {revoked}
        {limits
          ? ` · limits ${limits.max_objects} objects / ${limits.max_bytes} bytes / ${limits.max_concurrent} concurrent`
          : ""}
      </p>
      {devices.map((row) => (
        <div key={row.folder} style={{ fontSize: "0.85rem", marginBottom: "0.3rem" }}>
          <code>{row.folder}</code>
          <span style={{ color: "var(--text-muted)", marginLeft: "0.6rem" }}>{row.status}</span>
        </div>
      ))}
      <div style={{ display: "flex", flexWrap: "wrap", gap: "0.5rem", marginTop: "0.75rem" }}>
        <button
          type="button"
          className="btn-primary"
          disabled={disabled || busy}
          onClick={() => void run(() => onSession("retry"))}
        >
          Retry
        </button>
        <button
          type="button"
          className="btn-secondary"
          disabled={disabled || busy}
          onClick={() =>
            void run(async () => {
              const next = await hubSyncDiagnostics();
              const json = JSON.stringify(next, null, 2);
              setDiagnostics(safeDiagnostics(next) && safeText(json) ? json : "{}");
            })
          }
        >
          Diagnostics
        </button>
      </div>
      {diagnostics && (
        <pre
          style={{
            marginTop: "0.75rem",
            fontSize: "0.75rem",
            maxHeight: "12rem",
            overflow: "auto",
            whiteSpace: "pre-wrap",
          }}
        >
          {diagnostics}
        </pre>
      )}
      {error && (
        <p role="alert" style={{ color: "#fca5a5" }}>
          {error}
        </p>
      )}
    </div>
  );
}

function safeDiagnostics(diag: Diagnostics): boolean {
  const blob = [
    diag.last_verified_base ?? "",
    diag.resume_action ?? "",
    ...diag.trust.flatMap((row) => [row.folder, row.status]),
    ...diag.history.flatMap((row) => [row.action, ...row.warnings]),
  ].join(" ");
  return safeText(blob);
}
