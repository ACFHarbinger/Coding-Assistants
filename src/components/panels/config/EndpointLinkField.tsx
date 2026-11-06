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

interface EndpointCredentialStatus {
  is_set: boolean;
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
  const [savingKey, setSavingKey] = useState(false);
  const [health, setHealth] = useState<EndpointHealth | null>(null);
  const [draft, setDraft] = useState(config.endpoint || "");
  const [apiKey, setApiKey] = useState("");
  const [keySet, setKeySet] = useState(false);
  useEffect(() => {
    setDraft(config.endpoint || "");
  }, [config.endpoint]);
  const trimmed = draft.trim();
  const connected = Boolean(trimmed) && config.endpoint_state !== "disconnected";
  const busy = pending || checking || savingKey;

  useEffect(() => {
    if (!trimmed) {
      setKeySet(false);
      return;
    }
    let cancelled = false;
    invoke<EndpointCredentialStatus>("endpoint_credential_status", { endpoint: trimmed })
      .then((status) => {
        if (!cancelled) setKeySet(status.is_set);
      })
      .catch(() => {
        if (!cancelled) setKeySet(false);
      });
    return () => {
      cancelled = true;
    };
  }, [trimmed]);

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

  const saveKey = async () => {
    setSavingKey(true);
    setError("");
    try {
      const status = await invoke<EndpointCredentialStatus>("endpoint_credential_set", {
        endpoint: trimmed,
        apiKey,
      });
      setKeySet(status.is_set);
      setApiKey("");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Could not save the API key.");
    } finally {
      setSavingKey(false);
    }
  };

  const clearKey = async () => {
    setSavingKey(true);
    setError("");
    try {
      const status = await invoke<EndpointCredentialStatus>("endpoint_credential_clear", {
        endpoint: trimmed,
      });
      setKeySet(status.is_set);
      setApiKey("");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Could not clear the API key.");
    } finally {
      setSavingKey(false);
    }
  };

  const help = !trimmed
    ? "Leave blank to let Coding Assistants start the provider."
    : !connected
      ? "Disconnected. The address is kept, and this role will not start a process for it."
      : health
        ? health.message
        : "Connected. Chat completions use this address. Reachability is not checked.";
  const keyHelp = !trimmed
    ? "An API key can be saved after an address is entered."
    : keySet
      ? "A key is saved for this address. Every role that uses it sends that key."
      : "No key is saved for this address. The request is sent without one.";

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
      <label className="label">
        Endpoint API key <span style={{ color: "var(--text-muted)", fontWeight: 400 }}>(optional, for this address)</span>
      </label>
      <input
        disabled={busy || !trimmed}
        type="password"
        autoComplete="off"
        value={apiKey}
        onChange={(event) => setApiKey(event.target.value)}
        placeholder="Not stored on the role"
        aria-label="Endpoint API key"
        style={{ padding: "0.5rem", borderRadius: "6px", background: "rgba(0,0,0,0.3)", color: "white", border: "1px solid var(--border-color)", outline: "none" }}
      />
      <div style={{ display: "flex", gap: "0.5rem", alignItems: "center" }}>
        <button type="button" className="btn-secondary" onClick={saveKey} disabled={busy || !trimmed || !apiKey.trim()}>
          Save key
        </button>
        <button type="button" className="btn-secondary" onClick={clearKey} disabled={busy || !trimmed || !keySet}>
          Clear key
        </button>
        <span style={{ color: "var(--text-muted)", fontSize: "0.75rem" }}>{keyHelp}</span>
      </div>
      {error && <span style={{ color: "#f59e0b", fontSize: "0.75rem" }}>{error}</span>}
    </div>
  );
}
