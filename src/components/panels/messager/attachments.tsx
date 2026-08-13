// @ts-nocheck
import { useEffect, useState } from "react";
import { invoke } from "../../../lib/tauri";

/** Self-contained inline token embedded in a message body — no schema
 * change to messages, mirrors the existing `[Memory #<id>]` convention. */
const ATTACHMENT_TOKEN_RE = /\[attachment:([0-9a-f-]{36}):([^\]]*)\]/g;

export function attachmentToken(id: string, filename: string): string {
  return `[attachment:${id}:${encodeURIComponent(filename)}]`;
}

export function extractAttachmentTokens(body: string): { id: string; filename: string }[] {
  const found: { id: string; filename: string }[] = [];
  for (const match of body.matchAll(ATTACHMENT_TOKEN_RE)) {
    found.push({ id: match[1], filename: decodeURIComponent(match[2] || "") });
  }
  return found;
}

export function fileToBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onerror = () => reject(reader.error ?? new Error("failed to read file"));
    reader.onload = () => {
      const result = String(reader.result || "");
      const comma = result.indexOf(",");
      resolve(comma >= 0 ? result.slice(comma + 1) : result);
    };
    reader.readAsDataURL(file);
  });
}

/** All harnesses are dispatched as a plain CLI subprocess prompt, not a
 * multimodal API call, so an attachment is handed over the same way any
 * coding-agent CLI expects to see a file: as an absolute path in the
 * prompt text for the agent's own file-read/vision tool to open. Runs
 * only on the copy of the body sent to `hub_inject_harness` — the stored
 * Hub message keeps the `[attachment:...]` token for inline UI rendering. */
export function resolveDispatchBody(
  body: string,
  attachments: { id: string; absolutePath: string; filename: string }[],
): string {
  if (attachments.length === 0) return body;
  const byId = new Map(attachments.map(a => [a.id, a]));
  return body.replace(ATTACHMENT_TOKEN_RE, (full, id, encodedName) => {
    const known = byId.get(id);
    if (!known) return full;
    return `${decodeURIComponent(encodedName)} (attached file: ${known.absolutePath})`;
  });
}

export async function uploadAttachment(file: File) {
  const dataBase64 = await fileToBase64(file);
  return invoke("hub_save_attachment", {
    args: {
      filename: file.name || "attachment",
      mime: file.type || "application/octet-stream",
      dataBase64,
    },
  });
}

/** Splits a message body into plain-text runs and `<AttachmentInline>`
 * nodes wherever an attachment token appears, for direct use as React
 * children in place of a raw `{msg.body}` text node. */
export function renderMessageBody(body: string): React.ReactNode[] {
  const nodes: React.ReactNode[] = [];
  let lastIndex = 0;
  let matchIndex = 0;
  for (const match of body.matchAll(ATTACHMENT_TOKEN_RE)) {
    const start = match.index ?? 0;
    if (start > lastIndex) nodes.push(body.slice(lastIndex, start));
    const id = match[1];
    const filename = decodeURIComponent(match[2] || "");
    nodes.push(<AttachmentInline key={`att-${id}-${matchIndex++}`} id={id} filename={filename} />);
    lastIndex = start + match[0].length;
  }
  if (lastIndex < body.length) nodes.push(body.slice(lastIndex));
  return nodes.length > 0 ? nodes : [body];
}

const attachmentCache = new Map<string, any>();

export function AttachmentInline({ id, filename }: { id: string; filename: string }) {
  const [payload, setPayload] = useState(() => attachmentCache.get(id) ?? null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    if (payload || failed) return;
    let cancelled = false;
    invoke("hub_get_attachment", { id })
      .then((result: any) => {
        if (cancelled) return;
        if (!result) {
          setFailed(true);
          return;
        }
        attachmentCache.set(id, result);
        setPayload(result);
      })
      .catch(() => {
        if (!cancelled) setFailed(true);
      });
    return () => {
      cancelled = true;
    };
  }, [id, payload, failed]);

  const chipStyle: React.CSSProperties = {
    display: "inline-flex",
    alignItems: "center",
    gap: "0.4rem",
    padding: "0.35rem 0.65rem",
    borderRadius: "8px",
    background: "rgba(255,255,255,0.06)",
    border: "1px solid var(--border-color)",
    color: "var(--text-main)",
    fontSize: "0.8rem",
    textDecoration: "none",
    verticalAlign: "middle",
    margin: "0.15rem 0",
  };

  if (failed) {
    return <span style={{ ...chipStyle, color: "#f87171" }}>⚠ Attachment unavailable: {filename}</span>;
  }
  if (!payload) {
    return <span style={chipStyle}>📎 Loading {filename}…</span>;
  }

  const { record, data_base64 } = payload;
  const dataUrl = `data:${record.mime};base64,${data_base64}`;
  const sizeKb = (record.byte_size / 1024).toFixed(1);

  if (record.mime.startsWith("image/")) {
    return (
      <a href={dataUrl} download={record.filename} style={{ display: "block", margin: "0.35rem 0" }}>
        <img
          src={dataUrl}
          alt={record.filename}
          style={{
            maxWidth: "320px",
            maxHeight: "260px",
            borderRadius: "10px",
            border: "1px solid var(--border-color)",
            display: "block",
          }}
        />
      </a>
    );
  }

  return (
    <a href={dataUrl} download={record.filename} style={chipStyle}>
      📎 {record.filename} <span style={{ color: "var(--text-muted)" }}>({sizeKb} KB)</span>
    </a>
  );
}
