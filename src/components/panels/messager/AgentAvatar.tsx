import { useEffect, useState, type CSSProperties } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "../../../lib/tauri";
import { AvatarCropModal } from "./AvatarCropModal";
import type { AttachmentPayload, HubAgent } from "./types";

const IMAGE_FILTERS = [{ name: "Images", extensions: ["png", "jpg", "jpeg", "gif", "webp"] }];

const avatarUrlCache = new Map<string, string>();

export type AvatarPreview = {
  dataBase64: string;
  mime: string;
};

// The native file-picker dialog returns an absolute filesystem path.
// Bytes for the crop canvas come from `hub_read_avatar_preview`, a
// single-purpose read that matches `hub_set_agent_avatar`'s own
// empty-base64 path fallback — not the webview asset protocol, which
// would need a standing `$HOME/**` scope just to preview a file the
// OS dialog already granted access to.
export async function pickAvatarImagePath(): Promise<string | null> {
  const selected = await open({
    multiple: false,
    filters: IMAGE_FILTERS,
  });
  if (!selected || Array.isArray(selected)) return null;
  return selected;
}

export async function readAvatarPreview(path: string): Promise<AvatarPreview> {
  return invoke<AvatarPreview>("hub_read_avatar_preview", { path });
}

export async function setAgentAvatarFromPng(agentId: string, dataBase64: string): Promise<HubAgent> {
  return invoke<HubAgent>("hub_set_agent_avatar", {
    args: {
      agentId,
      filename: "avatar.png",
      mime: "image/png",
      dataBase64,
    },
  });
}

export async function clearAgentAvatar(agentId: string): Promise<HubAgent> {
  return invoke<HubAgent>("hub_clear_agent_avatar", { agentId });
}

async function loadAvatarUrl(attachmentId: string): Promise<string | null> {
  const cached = avatarUrlCache.get(attachmentId);
  if (cached) return cached;
  const payload = await invoke<AttachmentPayload | null>("hub_get_attachment", { id: attachmentId });
  if (!payload) return null;
  const url = `data:${payload.record.mime};base64,${payload.data_base64}`;
  avatarUrlCache.set(attachmentId, url);
  return url;
}

export function AgentAvatar(props: {
  agentId: string;
  displayName: string;
  avatarAttachmentId?: string | null;
  size?: number;
  background?: string;
  editable?: boolean;
  onChanged?: () => void;
}) {
  const {
    agentId,
    displayName,
    avatarAttachmentId,
    size = 28,
    background,
    editable = false,
    onChanged,
  } = props;
  const [url, setUrl] = useState<string | null>(
    () => (avatarAttachmentId ? avatarUrlCache.get(avatarAttachmentId) ?? null : null),
  );
  const [busy, setBusy] = useState(false);
  const [cropSrc, setCropSrc] = useState<string | null>(null);

  useEffect(() => {
    if (!avatarAttachmentId) {
      setUrl(null);
      return;
    }
    const cached = avatarUrlCache.get(avatarAttachmentId);
    if (cached) {
      setUrl(cached);
      return;
    }
    let cancelled = false;
    loadAvatarUrl(avatarAttachmentId)
      .then((next) => {
        if (!cancelled) setUrl(next);
      })
      .catch(() => {
        if (!cancelled) setUrl(null);
      });
    return () => {
      cancelled = true;
    };
  }, [avatarAttachmentId]);

  const initials = (displayName || agentId).slice(0, 2).toUpperCase();
  const box: CSSProperties = {
    width: size,
    height: size,
    borderRadius: "50%",
    background: url ? "transparent" : (background || "linear-gradient(135deg, #64748b, #334155)"),
    display: "inline-flex",
    alignItems: "center",
    justifyContent: "center",
    overflow: "hidden",
    flexShrink: 0,
    fontWeight: 700,
    color: "#fff",
    fontSize: Math.max(10, Math.round(size * 0.38)),
    boxShadow: "0 2px 6px rgba(0,0,0,0.2)",
    position: "relative",
  };

  const run = async (action: () => Promise<unknown>) => {
    if (busy) return;
    setBusy(true);
    try {
      const result = await action();
      if (result !== null) onChanged?.();
    } catch (error) {
      alert(`Avatar update failed: ${error}`);
    } finally {
      setBusy(false);
    }
  };

  const picture = url ? (
    <img src={url} alt="" style={{ width: "100%", height: "100%", objectFit: "cover" }} />
  ) : (
    initials
  );

  if (!editable) {
    return <span title={displayName} style={box}>{picture}</span>;
  }

  return (
    <span style={{ display: "inline-flex", alignItems: "center", gap: "0.2rem" }}>
      <button
        type="button"
        title={`Set avatar for ${displayName}`}
        disabled={busy}
        onClick={(event) => {
          event.stopPropagation();
          void run(async () => {
            const path = await pickAvatarImagePath();
            if (!path) return null;
            const preview = await readAvatarPreview(path);
            setCropSrc(`data:${preview.mime};base64,${preview.dataBase64}`);
            return null;
          });
        }}
        style={{ ...box, border: "none", padding: 0, cursor: busy ? "wait" : "pointer" }}
      >
        {picture}
      </button>
      {avatarAttachmentId && (
        <button
          type="button"
          title={`Clear avatar for ${displayName}`}
          disabled={busy}
          onClick={(event) => {
            event.stopPropagation();
            void run(() => clearAgentAvatar(agentId));
          }}
          style={{
            background: "transparent",
            border: "none",
            color: "var(--text-muted)",
            cursor: busy ? "wait" : "pointer",
            fontSize: "0.7rem",
            padding: 0,
            lineHeight: 1,
          }}
        >
          ×
        </button>
      )}
      {cropSrc && (
        <AvatarCropModal
          imageSrc={cropSrc}
          onCancel={() => setCropSrc(null)}
          onSave={async (pngBase64) => {
            await setAgentAvatarFromPng(agentId, pngBase64);
            setCropSrc(null);
            onChanged?.();
          }}
        />
      )}
    </span>
  );
}
