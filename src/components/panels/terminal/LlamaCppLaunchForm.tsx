import type { FormEvent } from "react";

export interface LlamaCppLaunchFormProps {
  modelPath: string;
  port: string;
  busy: boolean;
  onModelPath: (value: string) => void;
  onPort: (value: string) => void;
  onSubmit: () => void;
  onCancel: () => void;
}

const inputStyle = {
  padding: "0.3rem 0.5rem",
  fontSize: "0.8rem",
  borderRadius: "4px",
  border: "1px solid rgba(148, 163, 184, 0.3)",
  background: "rgba(0, 0, 0, 0.3)",
  color: "#f8fafc",
  fontFamily: "var(--font-mono)",
} as const;

export default function LlamaCppLaunchForm({
  modelPath,
  port,
  busy,
  onModelPath,
  onPort,
  onSubmit,
  onCancel,
}: LlamaCppLaunchFormProps) {
  const submit = (event: FormEvent) => {
    event.preventDefault();
    onSubmit();
  };

  return (
    <form
      aria-label="llama.cpp server"
      onSubmit={submit}
      style={{
        display: "flex",
        alignItems: "center",
        flexWrap: "wrap",
        gap: "0.5rem",
        padding: "0.45rem 0.85rem",
        borderRadius: "10px",
        background: "rgba(0, 0, 0, 0.3)",
        border: "1px solid var(--border-color)",
      }}
    >
      <label style={{ display: "flex", alignItems: "center", gap: "0.35rem", fontSize: "0.75rem", color: "var(--text-muted)", fontWeight: 600 }}>
        Model file
        <input
          aria-label="llama.cpp model file"
          value={modelPath}
          onChange={(event) => onModelPath(event.target.value)}
          placeholder="Absolute path to the model file"
          spellCheck={false}
          autoComplete="off"
          disabled={busy}
          style={{ ...inputStyle, minWidth: "16rem", flex: 1 }}
        />
      </label>
      <label style={{ display: "flex", alignItems: "center", gap: "0.35rem", fontSize: "0.75rem", color: "var(--text-muted)", fontWeight: 600 }}>
        Port
        <input
          aria-label="llama.cpp port"
          type="number"
          min={1}
          max={65535}
          value={port}
          onChange={(event) => onPort(event.target.value)}
          disabled={busy}
          style={{ ...inputStyle, width: "6rem" }}
        />
      </label>
      <button
        type="submit"
        className="btn-primary noc-focusable"
        style={{ marginTop: 0, padding: "0.3rem 0.65rem", fontSize: "0.8rem" }}
        disabled={busy}
      >
        Start llama-server
      </button>
      <button
        type="button"
        className="btn-secondary noc-focusable"
        style={{ marginTop: 0, padding: "0.3rem 0.65rem", fontSize: "0.8rem" }}
        disabled={busy}
        onClick={onCancel}
      >
        Cancel
      </button>
    </form>
  );
}
