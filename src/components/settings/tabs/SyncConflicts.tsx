import { useCallback, useEffect, useState } from "react";
import {
  hubSyncConflicts,
  hubSyncExpired,
  hubSyncPurgeExpired,
  hubSyncResolve,
  hubSyncTombstones,
  type ConflictChoice,
  type ConflictItem,
  type Tombstone,
} from "./syncApi";

const SECRET = /cloud-sync\.key|\btoken\b|refresh/i;

const CHOICES: { choice: ConflictChoice; label: string }[] = [
  { choice: "local", label: "Local" },
  { choice: "remote", label: "Remote" },
  { choice: "keep-both", label: "Keep both" },
  { choice: "manual", label: "Manual" },
];

function isSafe(item: ConflictItem): boolean {
  return !SECRET.test(`${item.slug} ${item.path} ${item.reason}`);
}

export default function SyncConflicts({ disabled }: { disabled: boolean }) {
  const [items, setItems] = useState<ConflictItem[]>([]);
  const [tombstones, setTombstones] = useState<Tombstone[]>([]);
  const [expiredCount, setExpiredCount] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    const [nextItems, nextTombstones, expired] = await Promise.all([
      hubSyncConflicts(),
      hubSyncTombstones(),
      hubSyncExpired(),
    ]);
    setItems(nextItems);
    setTombstones(nextTombstones.filter((item) => !SECRET.test(`${item.slug} ${item.path}`)));
    setExpiredCount(expired.filter((item) => !SECRET.test(item.slug)).length);
  }, []);

  useEffect(() => {
    void refresh().catch((err) => setError(String(err)));
  }, [refresh]);

  async function resolve(slug: string, choice: ConflictChoice) {
    setBusy(true);
    setError(null);
    try {
      await hubSyncResolve(slug, choice);
      await refresh();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function purgeExpired() {
    setBusy(true);
    setError(null);
    try {
      await hubSyncPurgeExpired(true);
      await refresh();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  const shown = items.filter(isSafe);

  return (
    <div style={{ marginTop: "1.25rem" }}>
      <p style={{ fontWeight: 600, fontSize: "0.9rem", marginBottom: "0.4rem" }}>
        Queued conflicts
      </p>
      <p style={{ color: "var(--text-muted)", fontSize: "0.8rem", marginBottom: "0.75rem" }}>
        Both versions stay under sync/conflicts/. Live hub.db is never replaced.
      </p>
      {shown.length === 0 && (
        <p style={{ color: "var(--text-muted)", fontSize: "0.85rem" }}>No queued conflicts.</p>
      )}
      {shown.map((item) => (
        <div
          key={item.slug}
          style={{
            marginBottom: "0.85rem",
            padding: "0.65rem 0.75rem",
            border: "1px solid var(--border-color)",
            borderRadius: "8px",
          }}
        >
          <div style={{ fontSize: "0.85rem", marginBottom: "0.4rem" }}>
            <code>{item.slug}</code>
            <span style={{ color: "var(--text-muted)", marginLeft: "0.6rem" }}>{item.reason}</span>
            <span style={{ marginLeft: "0.6rem" }}>{item.decision?.choice ?? "pending"}</span>
          </div>
          <div style={{ display: "flex", flexWrap: "wrap", gap: "0.4rem" }}>
            {CHOICES.map(({ choice, label }) => (
              <button
                key={choice}
                type="button"
                className="btn-secondary"
                disabled={disabled || busy}
                onClick={() => void resolve(item.slug, choice)}
              >
                {label}
              </button>
            ))}
          </div>
        </div>
      ))}
      <p style={{ fontWeight: 600, fontSize: "0.9rem", margin: "1.25rem 0 0.4rem" }}>
        Tombstones
      </p>
      <p style={{ color: "var(--text-muted)", fontSize: "0.8rem", marginBottom: "0.75rem" }}>
        Confirm-only deletes. Expired copies ({expiredCount}) stay until you purge.
        Default retention is 30 days. Live hub.db is never deleted.
      </p>
      {tombstones.length === 0 && (
        <p style={{ color: "var(--text-muted)", fontSize: "0.85rem" }}>No tombstones.</p>
      )}
      {tombstones.map((item) => (
        <div key={item.slug} style={{ fontSize: "0.85rem", marginBottom: "0.35rem" }}>
          <code>{item.slug}</code>
          <span style={{ color: "var(--text-muted)", marginLeft: "0.6rem" }}>
            expires {item.expires_at}
          </span>
        </div>
      ))}
      <button
        type="button"
        className="btn-secondary"
        disabled={disabled || busy || expiredCount === 0}
        onClick={() => void purgeExpired()}
        style={{ marginTop: "0.75rem" }}
      >
        Purge expired (30d)
      </button>
      {error && (
        <p role="alert" style={{ color: "#fca5a5" }}>
          {error}
        </p>
      )}
    </div>
  );
}
