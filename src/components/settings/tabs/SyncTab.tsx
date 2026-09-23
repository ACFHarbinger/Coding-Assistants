import { useCallback, useEffect, useState } from "react";
import { FieldRow } from "./shared";
import SyncConflicts from "./SyncConflicts";
import {
  hubSyncCancel,
  hubSyncPreview,
  hubSyncStart,
  hubSyncStatus,
  type SyncPlan,
  type SyncSession,
} from "./syncApi";

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
    <div>
      <p style={{ color: "var(--text-muted)", fontSize: "0.85rem", lineHeight: 1.5 }}>
        Owner-started only. Preview never transfers. A run holds the Hub lock
        and encrypts/restores a snapshot; live hub.db is never replaced.
        Connect the Drive refresh token under Credentials. Local rehearsal
        uses CA_SYNC_FAKE_ROOT or hub/sync/remote.
      </p>
      <FieldRow label="Account" hint="Presence only. The refresh token is never shown.">
        <span>
          {plan?.account_connected ? `connected (${plan.provider})` : "not connected"}
        </span>
      </FieldRow>
      <FieldRow label="Schema">
        <span>
          local {plan?.local_schema ?? "—"}
          {plan?.replica_schema ? ` · replica ${plan.replica_schema}` : ""}
        </span>
      </FieldRow>
      {plan?.schema_warning && (
        <p style={{ color: "#fbbf24" }}>{plan.schema_warning}</p>
      )}
      <FieldRow label="Last verified base">
        <code>{plan?.last_verified_base ?? "none"}</code>
      </FieldRow>
      <FieldRow label="Policy counts" hint="Category totals only; secret names are omitted.">
        <span>{counts}</span>
      </FieldRow>
      <FieldRow label="Lock">
        <span>{plan?.lock_held ? "held" : "free"}</span>
      </FieldRow>
      <div style={{ display: "flex", flexWrap: "wrap", gap: "0.5rem", marginBottom: "1rem" }}>
        <button
          type="button"
          className="btn-secondary"
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
            className="btn-primary"
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
          className="btn-secondary"
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
      {error && (
        <p role="alert" style={{ color: "#fca5a5" }}>
          {error}
        </p>
      )}
      {session?.result.warnings.map((warning) => (
        <p key={warning} style={{ color: "#fbbf24", fontSize: "0.85rem" }}>
          {warning}
        </p>
      ))}
      <SyncConflicts disabled={busy} />
    </div>
  );
}
