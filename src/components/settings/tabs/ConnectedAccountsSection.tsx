/**
 * Connected Accounts Section — #284 / #286 (H7).
 *
 * Surfaces account-connection status for external providers (ChatGPT, Claude,
 * Google, etc.) backed by #286 single-user storage under the provisional "local" key.
 *
 * Security invariants:
 *  - Tokens never cross IPC or land in the UI.
 *  - Only metadata (provider, external label, connection kind, timestamp) is read.
 *  - Connecting records the user's vendor-login association; authentication
 *    remains with that vendor's native CLI or device flow.
 */

import { useCallback, useEffect, useState } from "react";
import type { LinkedAccountStatus } from "../types";
import { linkAccountCli, listLinkedAccounts, unlinkAccount } from "../api";
import { NOC_RADIUS, NOC_SURFACES } from "../../common/telemetry/tokens";
import { nocCardDenseStyle } from "../../common/telemetry/panel";

interface ProviderMeta {
  displayName: string;
  hint: string;
  defaultLabelPlaceholder: string;
}

const PROVIDER_METAS: Record<string, ProviderMeta> = {
  openai: {
    displayName: "ChatGPT / OpenAI",
    hint: "Connect your OpenAI / ChatGPT subscription or API account.",
    defaultLabelPlaceholder: "e.g. user@openai.com",
  },
  anthropic: {
    displayName: "Claude (Anthropic)",
    hint: "Uses vendor CLI session (claude auth login) or recorded token.",
    defaultLabelPlaceholder: "e.g. user@anthropic.com",
  },
  google: {
    displayName: "Google / Gemini",
    hint: "Uses vendor CLI session (agy / gemini auth) or recorded token.",
    defaultLabelPlaceholder: "e.g. user@gmail.com",
  },
  deepseek: {
    displayName: "DeepSeek",
    hint: "Direct account or API session link.",
    defaultLabelPlaceholder: "e.g. user@deepseek.com",
  },
};

export default function ConnectedAccountsSection() {
  const [accounts, setAccounts] = useState<LinkedAccountStatus[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [actionBusy, setActionBusy] = useState<string | null>(null);
  const [labels, setLabels] = useState<Record<string, string>>({});
  const [showConnectInput, setShowConnectInput] = useState<Record<string, boolean>>({});

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const list = await listLinkedAccounts();
      setAccounts(list);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const handleConnect = async (provider: string) => {
    setActionBusy(provider);
    setError(null);
    try {
      const label = labels[provider]?.trim() || null;
      await linkAccountCli(provider, label);
      setShowConnectInput((prev) => ({ ...prev, [provider]: false }));
      setLabels((prev) => ({ ...prev, [provider]: "" }));
      await refresh();
    } catch (err) {
      setError(String(err));
    } finally {
      setActionBusy(null);
    }
  };

  const handleDisconnect = async (provider: string) => {
    setActionBusy(provider);
    setError(null);
    try {
      await unlinkAccount(provider);
      await refresh();
    } catch (err) {
      setError(String(err));
    } finally {
      setActionBusy(null);
    }
  };

  return (
    <section style={{ display: "grid", gap: "0.85rem" }}>
      <div style={{ marginBottom: "0.85rem" }}>
        <h3
          style={{
            margin: "0 0 0.35rem",
            fontSize: "0.82rem",
            fontWeight: 700,
            textTransform: "uppercase",
            letterSpacing: "0.06em",
            color: NOC_SURFACES.textMuted,
          }}
        >
          Connected Accounts (#286)
        </h3>
        <p style={{ margin: 0, color: NOC_SURFACES.textMuted, fontSize: "0.82rem", lineHeight: 1.55 }}>
          Link your external vendor accounts (ChatGPT, Claude, Google, etc.). Connecting records
          your vendor CLI or OAuth session under your single-user profile (provisional &ldquo;local&rdquo; key).
          This records the account association only. Authentication remains with the vendor's
          native CLI or device flow; tokens are never displayed here.
        </p>
      </div>

      {error && (
        <div
          style={{
            padding: "0.5rem 0.8rem",
            borderRadius: NOC_RADIUS.s,
            background: "rgba(239, 68, 68, 0.12)",
            border: "1px solid rgba(248, 113, 113, 0.45)",
            color: "#fca5a5",
            fontSize: "0.82rem",
          }}
        >
          {error}
        </div>
      )}

      <div style={{ display: "grid", gap: "0.65rem" }}>
        {accounts.map((account) => {
          const meta = PROVIDER_METAS[account.provider] ?? {
            displayName: account.provider,
            hint: "External account connection.",
            defaultLabelPlaceholder: "Account label",
          };
          const isBusy = actionBusy === account.provider;
          const isConnecting = showConnectInput[account.provider] ?? false;

          return (
            <div
              key={account.provider}
              style={{
                ...nocCardDenseStyle,
                border: `1px solid ${account.isLinked ? "rgba(16, 185, 129, 0.4)" : NOC_SURFACES.border}`,
                background: account.isLinked ? "rgba(16, 185, 129, 0.08)" : NOC_SURFACES.bgDeep,
                display: "grid",
                gap: "0.5rem",
              }}
            >
              <div
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                  flexWrap: "wrap",
                  gap: "0.5rem",
                }}
              >
                <div style={{ display: "flex", alignItems: "center", gap: "0.6rem", flexWrap: "wrap" }}>
                  <strong style={{ fontSize: "0.92rem", color: NOC_SURFACES.textPrimary }}>
                    {meta.displayName}
                  </strong>
                  <span
                    style={{
                      fontSize: "0.7rem",
                      fontWeight: 600,
                      padding: "0.1rem 0.45rem",
                      borderRadius: "999px",
                      background: account.isLinked ? "rgba(16, 185, 129, 0.12)" : "rgba(15, 23, 42, 0.6)",
                      border: `1px solid ${account.isLinked ? "rgba(16, 185, 129, 0.35)" : NOC_SURFACES.border}`,
                      color: account.isLinked ? "#6ee7b7" : NOC_SURFACES.textMuted,
                    }}
                  >
                    {account.isLinked ? "Connected" : "Not Connected"}
                  </span>
                  {account.isLinked && (
                    <span
                      style={{
                        fontSize: "0.7rem",
                        padding: "0.1rem 0.45rem",
                        borderRadius: "999px",
                        background: "rgba(99, 102, 241, 0.12)",
                        border: "1px solid rgba(99, 102, 241, 0.35)",
                        color: "#a5b4fc",
                      }}
                    >
                      {account.source === "vendor_cli" ? "Vendor CLI" : "OAuth Vault"}
                    </span>
                  )}
                </div>

                <div>
                  {account.isLinked ? (
                    <button
                      type="button"
                      className="btn-secondary noc-focusable"
                      disabled={isBusy || loading}
                      onClick={() => void handleDisconnect(account.provider)}
                      style={{
                        padding: "0.3rem 0.7rem",
                        fontSize: "0.78rem",
                        marginTop: 0,
                        color: "#fca5a5",
                        borderColor: "rgba(239, 68, 68, 0.35)",
                      }}
                    >
                      {isBusy ? "Disconnecting…" : "Disconnect"}
                    </button>
                  ) : isConnecting ? (
                    <div style={{ display: "flex", gap: "0.4rem", alignItems: "center" }}>
                      <input
                        type="text"
                        placeholder={meta.defaultLabelPlaceholder}
                        value={labels[account.provider] ?? ""}
                        onChange={(e) =>
                          setLabels((prev) => ({ ...prev, [account.provider]: e.target.value }))
                        }
                        disabled={isBusy}
                        className="noc-focusable"
                        style={{
                          padding: "0.3rem 0.55rem",
                          fontSize: "0.8rem",
                          borderRadius: NOC_RADIUS.s,
                          border: `1px solid ${NOC_SURFACES.border}`,
                          background: "rgba(15, 23, 42, 0.6)",
                          color: NOC_SURFACES.textPrimary,
                          width: "180px",
                        }}
                      />
                      <button
                        type="button"
                        className="btn-primary noc-focusable"
                        disabled={isBusy}
                        onClick={() => void handleConnect(account.provider)}
                        style={{ padding: "0.3rem 0.65rem", fontSize: "0.78rem", marginTop: 0 }}
                      >
                        {isBusy ? "Saving…" : "Save Link"}
                      </button>
                      <button
                        type="button"
                        className="btn-secondary noc-focusable"
                        disabled={isBusy}
                        onClick={() =>
                          setShowConnectInput((prev) => ({ ...prev, [account.provider]: false }))
                        }
                        style={{ padding: "0.3rem 0.55rem", fontSize: "0.78rem", marginTop: 0 }}
                      >
                        Cancel
                      </button>
                    </div>
                  ) : (
                    <button
                      type="button"
                      className="btn-primary noc-focusable"
                      disabled={isBusy || loading}
                      onClick={() =>
                        setShowConnectInput((prev) => ({ ...prev, [account.provider]: true }))
                      }
                      style={{ padding: "0.3rem 0.75rem", fontSize: "0.78rem", marginTop: 0 }}
                    >
                      Connect
                    </button>
                  )}
                </div>
              </div>

              <div style={{ fontSize: "0.78rem", color: NOC_SURFACES.textMuted }}>
                {account.isLinked ? (
                  <span>
                    Account: <strong style={{ color: NOC_SURFACES.textPrimary }}>{account.externalLabel ?? "Active session"}</strong>
                    {account.linkedAt && (
                      <span className="tabular-nums" style={{ marginLeft: "0.75rem", color: NOC_SURFACES.textMuted }}>
                        Linked {new Date(account.linkedAt * 1000).toLocaleDateString()}
                      </span>
                    )}
                  </span>
                ) : (
                  meta.hint
                )}
              </div>
            </div>
          );
        })}
      </div>
    </section>
  );
}
