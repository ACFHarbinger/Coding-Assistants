import { useEffect, useState } from "react";
import type { ModelConfig } from "./types";
import { invoke } from "../../../lib/tauri";

interface EndpointLink {
  endpoint: string;
  state: "connected" | "disconnected";
  owns_process: boolean;
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
  const [draft, setDraft] = useState(config.endpoint || "");
  useEffect(() => {
    setDraft(config.endpoint || "");
  }, [config.endpoint]);
  const trimmed = draft.trim();
  const connected = Boolean(trimmed) && config.endpoint_state !== "disconnected";

  const writeDraft = (value: string) => {
    if (pending) return;
    setDraft(value);
    onConfigChange(index, "endpoint", value);
    onConfigChange(index, "endpoint_state", "disconnected");
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

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem" }}>
      <label className="label">
        Existing process endpoint <span style={{ color: "var(--text-muted)", fontWeight: 400 }}>(optional)</span>
      </label>
      <input
        disabled={pending}
        value={draft}
        onChange={(event) => writeDraft(event.target.value)}
        placeholder="http://127.0.0.1:1234/v1"
        aria-label="Existing process endpoint"
        style={{ padding: "0.5rem", borderRadius: "6px", background: "rgba(0,0,0,0.3)", color: "white", border: "1px solid var(--border-color)", outline: "none" }}
      />
      <div style={{ display: "flex", gap: "0.5rem", alignItems: "center" }}>
        <button type="button" className="btn-primary" onClick={connect} disabled={pending || !trimmed}>
          Connect
        </button>
        <button type="button" className="btn-secondary" onClick={disconnect} disabled={pending || !trimmed || !connected}>
          Disconnect
        </button>
        <span style={{ color: "var(--text-muted)", fontSize: "0.75rem" }}>
          {trimmed
            ? connected
              ? "Connected. Chat completions use this address. Reachability is not checked."
              : "Disconnected. The address is kept, and this role will not start a process for it."
            : "Leave blank to let Coding Assistants start the provider."}
        </span>
      </div>
      {error && <span style={{ color: "#f59e0b", fontSize: "0.75rem" }}>{error}</span>}
    </div>
  );
}
