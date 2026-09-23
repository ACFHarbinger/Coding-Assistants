import { useCallback, useEffect, useState } from "react";
import {
  hubSyncConflicts,
  hubSyncResolve,
  type ConflictChoice,
  type ConflictItem,
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
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    setItems(await hubSyncConflicts());
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
      {error && (
        <p role="alert" style={{ color: "#fca5a5" }}>
          {error}
        </p>
      )}
    </div>
  );
}
