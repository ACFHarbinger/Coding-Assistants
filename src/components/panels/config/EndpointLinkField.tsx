import { useEffect, useState } from "react";
import type { ModelConfig } from "./types";
import { invoke } from "../../../lib/tauri";

interface EndpointLink {
  endpoint: string;
  state: "connected" | "disconnected";
  owns_process: boolean;
}

interface EndpointHealth {
  kind: "reachable" | "authentication_required" | "unexpected" | "unreachable" | "not_probed";
  status: number | null;
  message: string;
}

export function EndpointLinkField({
  index,
  config,
  onConfigChange,
}: {
  index: number;
  config: ModelConfig;
  onConfigChange: (index: number, field: keyof ModelConfig, value: string) => void;
}) {
  const [error, setError] = useState("");
  const [pending, setPending] = useState(false);
  const [checking, setChecking] = useState(false);
  const [health, setHealth] = useState<EndpointHealth | null>(null);
  const [draft, setDraft] = useState(config.endpoint || "");
  useEffect(() => {
    setDraft(config.endpoint || "");
  }, [config.endpoint]);
  const trimmed = draft.trim();
  const connected = Boolean(trimmed) && config.endpoint_state !== "disconnected";
  const busy = pending || checking;

  const writeDraft = (value: string) => {
    if (busy) return;
    setDraft(value);
    onConfigChange(index, "endpoint", value);
    onConfigChange(index, "endpoint_state", "disconnected");
    setHealth(null);
    setError("");
  };

  const applyLink = (link: EndpointLink) => {
    if (link.owns_process) {
      setError("This endpoint is an external service. Connect does not take ownership of a process.");
      return;
    }
    onConfigChange(index, "endpoint", link.endpoint);
    onConfigChange(index, "endpoint_state", link.state);
  };

  const connect = async () => {
    setPending(true);
    setHealth(null);
    setError("");
    try {
      const link = await invoke<EndpointLink>("endpoint_link_connect", { endpoint: trimmed });
      applyLink(link);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Could not connect to that endpoint.");
    } finally {
      setPending(false);
    }
  };

  const disconnect = async () => {
    setPending(true);
    setHealth(null);
    setError("");
    try {
      const link = await invoke<EndpointLink>("endpoint_link_disconnect", { endpoint: draft });
      applyLink(link);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Could not disconnect that endpoint.");
    } finally {
      setPending(false);
    }
  };

  const check = async () => {
    setChecking(true);
    setError("");
    try {
      const result = await invoke<EndpointHealth>("endpoint_link_health", {
        endpoint: trimmed,
        endpointState: "connected",
      });
      setHealth(result);
    } catch (cause) {
      setHealth(null);
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setChecking(false);
    }
  };

  const help = !trimmed
    ? "Leave blank to let Coding Assistants start the provider."
    : !connected
      ? "Disconnected. The address is kept, and this role will not start a process for it."
      : health
        ? health.message
        : "Connected. Chat completions use this address. Reachability is not checked.";

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem" }}>
      <label className="label">
        Existing process endpoint <span style={{ color: "var(--text-muted)", fontWeight: 400 }}>(optional)</span>
      </label>
      <input
        disabled={busy}
        value={draft}
        onChange={(event) => writeDraft(event.target.value)}
        placeholder="http://127.0.0.1:1234/v1"
        aria-label="Existing process endpoint"
        style={{ padding: "0.5rem", borderRadius: "6px", background: "rgba(0,0,0,0.3)", color: "white", border: "1px solid var(--border-color)", outline: "none" }}
      />
      <div style={{ display: "flex", gap: "0.5rem", alignItems: "center" }}>
        <button type="button" className="btn-primary" onClick={connect} disabled={busy || !trimmed}>
          Connect
        </button>
        <button type="button" className="btn-secondary" onClick={disconnect} disabled={busy || !trimmed || !connected}>
          Disconnect
        </button>
        <button type="button" className="btn-secondary" onClick={check} disabled={busy || !connected}>
          {checking ? "Checking…" : "Check"}
        </button>
        <span style={{ color: "var(--text-muted)", fontSize: "0.75rem" }}>{help}</span>
      </div>
      {error && <span style={{ color: "#f59e0b", fontSize: "0.75rem" }}>{error}</span>}
    </div>
  );
}
