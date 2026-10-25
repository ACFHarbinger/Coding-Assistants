import { useCallback, useEffect, useState } from "react";
import { FieldRow } from "./shared";
import SyncConflicts from "./SyncConflicts";
import SyncOps from "./SyncOps";
import {
  hubSyncCancel,
  hubSyncPreview,
  hubSyncRetry,
  hubSyncStart,
  hubSyncStatus,
  type SyncPlan,
  type SyncSession,
} from "./syncApi";
import { NOC_COLORS, NOC_SURFACES } from "../../common/telemetry/tokens";
import { nocCardDenseStyle } from "../../common/telemetry/panel";

export default function SyncTab() {
  const [plan, setPlan] = useState<SyncPlan | null>(null);
  const [session, setSession] = useState<SyncSession | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    const status = await hubSyncStatus();
    setPlan(status.plan);
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

  const counts = plan
    ? Object.entries(plan.category_counts)
        .map(([name, count]) => `${name}: ${count}`)
        .join(" · ")
    : "—";

  return (
    <div style={{ display: "grid", gap: "1.25rem" }}>
      <p style={{ color: NOC_SURFACES.textMuted, fontSize: "0.85rem", lineHeight: 1.5, margin: 0 }}>
        Owner-started only. Preview never transfers. A run holds the Hub lock
        and encrypts/restores a snapshot; live hub.db is never replaced.
        Connect Drive or optional Firebase/Supabase/OneDrive/Dropbox credentials under Credentials.
        Tokens are never shown. Local rehearsal uses CA_SYNC_FAKE_ROOT or
        hub/sync/remote.
      </p>

      <div style={{ ...nocCardDenseStyle, padding: "1.25rem" }}>
        <FieldRow label="Account" hint="Presence only. The refresh token is never shown.">
          <span style={{ color: plan?.account_connected ? NOC_COLORS.ok : NOC_SURFACES.textMuted, fontWeight: 600 }}>
            {plan?.account_connected ? `connected (${plan.provider})` : "not connected"}
          </span>
        </FieldRow>
        <FieldRow label="Schema">
          <span style={{ color: NOC_SURFACES.textPrimary }}>
            local {plan?.local_schema ?? "—"}
            {plan?.replica_schema ? ` · replica ${plan.replica_schema}` : ""}
          </span>
        </FieldRow>
        {plan?.schema_warning && (
          <p style={{ color: NOC_COLORS.warn, fontSize: "0.85rem", margin: "-0.5rem 0 1rem" }}>{plan.schema_warning}</p>
        )}
        <FieldRow label="Last verified base">
          <code style={{ fontSize: "0.78rem", color: NOC_SURFACES.textSecondary }}>{plan?.last_verified_base ?? "none"}</code>
        </FieldRow>
        <FieldRow label="Policy counts" hint="Category totals only; secret names are omitted.">
          <span style={{ color: NOC_SURFACES.textPrimary }}>{counts}</span>
        </FieldRow>
        <FieldRow label="Lock">
          <span style={{ color: plan?.lock_held ? NOC_COLORS.warn : NOC_COLORS.ok, fontWeight: 600 }}>
            {plan?.lock_held ? "held" : "free"}
          </span>
        </FieldRow>

        <div style={{ display: "flex", flexWrap: "wrap", gap: "0.5rem", marginTop: "1rem" }}>
          <button
            type="button"
            className="btn-secondary noc-focusable"
            style={{ marginTop: 0 }}
            disabled={busy}
            onClick={() =>
              void run(async () => {
                setPlan(await hubSyncPreview());
                setSession(null);
              })
            }
          >
            Preview
          </button>
          {(["up", "down", "sync"] as const).map((action) => (
            <button
              key={action}
              type="button"
              className="btn-primary noc-focusable"
              style={{ marginTop: 0 }}
              disabled={busy}
              onClick={() =>
                void run(async () => {
                  const next = await hubSyncStart(action);
                  setSession(next);
                  setPlan(next.plan);
                })
              }
            >
              {action}
            </button>
          ))}
          <button
            type="button"
            className="btn-secondary noc-focusable"
            style={{ marginTop: 0 }}
            disabled={busy}
            onClick={() =>
              void run(async () => {
                await hubSyncCancel();
                setSession(null);
                await refresh();
              })
            }
          >
            Cancel
          </button>
        </div>
      </div>

      {error && (
        <p role="alert" style={{ color: NOC_COLORS.crit, fontSize: "0.85rem", margin: 0 }}>
          {error}
        </p>
      )}
      {session?.result.warnings.map((warning) => (
        <p key={warning} style={{ color: NOC_COLORS.warn, fontSize: "0.85rem", margin: 0 }}>
          {warning}
        </p>
      ))}
      <SyncOps
        disabled={busy}
        onSession={async () => {
          const next = await hubSyncRetry();
          setSession(next);
          setPlan(next.plan);
        }}
      />
      <SyncConflicts disabled={busy} />
    </div>
  );
}
