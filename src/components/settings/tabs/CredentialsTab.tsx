/**
 * Credentials tab — #284.
 *
 * Security invariants (enforced here, not just documented):
 *  - The credential VALUE never crosses IPC; only SecretStatus (presence,
 *    source, last-updated) is read back.
 *  - Inputs are type="password", never displayed as plain text.
 *  - Draft values are kept in component state and never logged or emitted.
 *  - After a successful set, the draft is immediately cleared; the component
 *    never holds the value longer than the user takes to submit.
 *  - "Never render/return a stored secret" — this component has no read path
 *    for a credential value.
 */

import { useCallback, useEffect, useState } from "react";
import type { FieldSpec, OwnerKind, SecretSource, SecretStatus } from "../types";
import { clearCredential, getCredentialStatus, listCredentialFields, setCredential } from "../api";
import ConnectedAccountsSection from "./ConnectedAccountsSection";

// ─── helpers ──────────────────────────────────────────────────────────────────

const OWNER_LABELS: Record<OwnerKind, string> = {
  provider: "Providers",
  harness: "Harnesses",
  tool: "Tools",
  mcp: "MCP Servers",
};

const OWNER_ORDER: OwnerKind[] = ["provider", "harness", "tool", "mcp"];

function sourceLabel(source: SecretSource): string {
  switch (source) {
    case "keychain": return "Keychain";
    case "file":     return "File";
    case "env_var":  return "Env Var";
    default:         return "Not Set";
  }
}

function sourceBadgeStyle(source: SecretSource, isSet: boolean): React.CSSProperties {
  if (!isSet) {
    return {
      background: "rgba(255,255,255,0.06)",
      border: "1px solid var(--border-color)",
      color: "var(--text-muted)",
    };
  }
  if (source === "env_var") {
    return {
      background: "rgba(245,158,11,0.12)",
      border: "1px solid rgba(245,158,11,0.35)",
      color: "#fde68a",
    };
  }
  return {
    background: "rgba(16,185,129,0.12)",
    border: "1px solid rgba(16,185,129,0.35)",
    color: "#6ee7b7",
  };
}

function formatUpdatedAt(ts: number | null): string {
  if (!ts) return "";
  return `Last saved ${new Date(ts * 1000).toLocaleDateString()}`;
}

// ─── types ────────────────────────────────────────────────────────────────────

interface RowState {
  status: SecretStatus | null;
  draft: string;       // write-only; cleared after successful set
  saving: boolean;
  error: string | null;
}

// ─── sub-components ───────────────────────────────────────────────────────────

interface CredentialRowProps {
  spec: FieldSpec;
  rowState: RowState;
  onDraftChange: (value: string) => void;
  onSet: () => void;
  onClear: () => void;
}

function CredentialRow({ spec, rowState, onDraftChange, onSet, onClear }: CredentialRowProps) {
  const { status, draft, saving, error } = rowState;
  const isSet = status?.isSet ?? false;

  return (
    <div
      style={{
        padding: "0.85rem 1rem",
        borderRadius: "10px",
        border: `1px solid ${isSet ? "rgba(99,102,241,0.35)" : "var(--border-color)"}`,
        background: isSet ? "rgba(99,102,241,0.05)" : "rgba(0,0,0,0.18)",
        display: "grid",
        gap: "0.5rem",
      }}
    >
      {/* Header row */}
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: "0.5rem" }}>
        <div style={{ display: "flex", alignItems: "center", gap: "0.55rem", flexWrap: "wrap" }}>
          <strong style={{ fontSize: "0.9rem", color: "var(--text-main)" }}>
            {spec.displayName}
          </strong>
          {/* Source badge */}
          {status && (
            <span
              style={{
                fontSize: "0.7rem",
                fontWeight: 600,
                padding: "0.1rem 0.45rem",
                borderRadius: "999px",
                ...sourceBadgeStyle(status.source, isSet),
              }}
              title={isSet ? `Stored in: ${sourceLabel(status.source)}` : "No credential stored"}
            >
              {isSet ? sourceLabel(status.source) : "Not Set"}
            </span>
          )}
          {spec.docsUrl && (
            <a
              href={spec.docsUrl}
              target="_blank"
              rel="noreferrer"
              style={{ fontSize: "0.72rem", color: "var(--primary-color, #60a5fa)", textDecoration: "none" }}
            >
              Docs ↗
            </a>
          )}
        </div>
        {status?.updatedAt && (
          <span style={{ fontSize: "0.72rem", color: "var(--text-muted)" }}>
            {formatUpdatedAt(status.updatedAt)}
          </span>
        )}
      </div>

      {/* Notes */}
      {spec.notes && (
        <div style={{ fontSize: "0.78rem", color: "var(--text-muted)" }}>{spec.notes}</div>
      )}

      {/* Env-var hint */}
      {spec.envVar && (
        <div style={{ fontSize: "0.75rem", color: "rgba(255,255,255,0.35)" }}>
          Env fallback: <code>{spec.envVar}</code>
          {status?.source === "env_var" && isSet
            ? " — active (no vault entry; set above to store in keychain)"
            : ""}
        </div>
      )}

      {/* Error */}
      {error && (
        <div
          style={{
            padding: "0.4rem 0.7rem",
            borderRadius: "6px",
            background: "rgba(239,68,68,0.12)",
            border: "1px solid rgba(248,113,113,0.4)",
            color: "#fca5a5",
            fontSize: "0.78rem",
          }}
        >
          {error}
        </div>
      )}

      {/* Write-only input row — only for secret fields */}
      {spec.secret && (
        <div style={{ display: "flex", gap: "0.5rem", alignItems: "center", flexWrap: "wrap" }}>
          <input
            type="password"
            autoComplete="off"
            placeholder={isSet ? "Replace stored credential…" : "Paste credential…"}
            value={draft}
            onChange={(e) => onDraftChange(e.target.value)}
            disabled={saving}
            style={{
              flex: 1,
              minWidth: 0,
              padding: "0.35rem 0.6rem",
              fontSize: "0.82rem",
              borderRadius: "6px",
              border: "1px solid var(--border-color)",
              background: "rgba(0,0,0,0.25)",
              color: "var(--text-main)",
            }}
          />
          <button
            type="button"
            className="btn-primary"
            disabled={saving || draft.trim().length === 0}
            onClick={onSet}
            style={{ padding: "0.35rem 0.75rem", fontSize: "0.78rem", marginTop: 0 }}
          >
            {saving ? "Saving…" : isSet ? "Update" : "Save"}
          </button>
          {isSet && status?.source !== "env_var" && (
            <button
              type="button"
              className="btn-secondary"
              disabled={saving}
              onClick={onClear}
              style={{ padding: "0.35rem 0.75rem", fontSize: "0.78rem", marginTop: 0 }}
            >
              Clear
            </button>
          )}
        </div>
      )}
    </div>
  );
}

// ─── main component ───────────────────────────────────────────────────────────

export default function CredentialsTab() {
  const [fields, setFields] = useState<FieldSpec[]>([]);
  const [rowStates, setRowStates] = useState<Record<string, RowState>>({});
  const [loadError, setLoadError] = useState<string | null>(null);

  const initRow = (): RowState => ({
    status: null,
    draft: "",
    saving: false,
    error: null,
  });

  const patchRow = useCallback((id: string, patch: Partial<RowState>) => {
    setRowStates((prev) => ({ ...prev, [id]: { ...prev[id], ...patch } }));
  }, []);

  const loadAll = useCallback(async () => {
    setLoadError(null);
    try {
      const catalog = await listCredentialFields();
      setFields(catalog);
      const initial: Record<string, RowState> = {};
      for (const spec of catalog) {
        initial[spec.id] = initRow();
      }
      setRowStates(initial);
      // Fetch statuses in parallel; errors per-field are non-fatal
      await Promise.all(
        catalog
          .filter((spec) => spec.secret)
          .map(async (spec) => {
            try {
              const status = await getCredentialStatus(spec.id);
              setRowStates((prev) => ({
                ...prev,
                [spec.id]: { ...prev[spec.id], status },
              }));
            } catch {
              // Non-fatal: leave status as null; row renders without a badge
            }
          }),
      );
    } catch (err) {
      setLoadError(String(err));
    }
  }, []);

  useEffect(() => {
    void loadAll();
  }, [loadAll]);

  const handleSet = async (spec: FieldSpec) => {
    const draft = rowStates[spec.id]?.draft ?? "";
    if (!draft.trim()) return;
    patchRow(spec.id, { saving: true, error: null });
    try {
      const status = await setCredential(spec.id, draft);
      // Clear the draft immediately — never keep the value in state
      patchRow(spec.id, { status, draft: "", saving: false });
    } catch (err) {
      patchRow(spec.id, { saving: false, error: String(err) });
    }
  };

  const handleClear = async (spec: FieldSpec) => {
    patchRow(spec.id, { saving: true, error: null });
    try {
      const status = await clearCredential(spec.id);
      patchRow(spec.id, { status, saving: false });
    } catch (err) {
      patchRow(spec.id, { saving: false, error: String(err) });
    }
  };

  // Group by ownerKind in canonical order
  const groups = OWNER_ORDER.map((kind) => ({
    kind,
    label: OWNER_LABELS[kind],
    specs: fields.filter((f) => f.ownerKind === kind),
  })).filter((g) => g.specs.length > 0);

  return (
    <div style={{ display: "grid", gap: "1.5rem" }}>
      <div>
        <p style={{ margin: 0, color: "var(--text-muted)", fontSize: "0.82rem", lineHeight: 1.55 }}>
          Credentials are stored in the system keychain and are never written to disk as plain
          text, logged, or returned over IPC. Values entered here replace or supplement
          environment variables — the resolver prefers the vault entry when both are present.
        </p>
      </div>

      {loadError && (
        <div
          style={{
            padding: "0.6rem 0.85rem",
            borderRadius: "8px",
            background: "rgba(239,68,68,0.12)",
            border: "1px solid rgba(248,113,113,0.45)",
            color: "#fca5a5",
            fontSize: "0.82rem",
          }}
        >
          {loadError}
        </div>
      )}

      {/* External account connections panel (#284 / #286) */}
      <ConnectedAccountsSection />

      {groups.map(({ kind, label, specs }) => (
        <section key={kind}>
          <h3
            style={{
              margin: "0 0 0.65rem",
              fontSize: "0.85rem",
              fontWeight: 700,
              textTransform: "uppercase",
              letterSpacing: "0.06em",
              color: "var(--text-muted)",
            }}
          >
            {label}
          </h3>
          <div style={{ display: "grid", gap: "0.65rem" }}>
            {specs.map((spec) => (
              <CredentialRow
                key={spec.id}
                spec={spec}
                rowState={rowStates[spec.id] ?? initRow()}
                onDraftChange={(value) => patchRow(spec.id, { draft: value })}
                onSet={() => void handleSet(spec)}
                onClear={() => void handleClear(spec)}
              />
            ))}
          </div>
        </section>
      ))}
    </div>
  );
}
