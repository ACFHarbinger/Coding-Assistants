import type { LayoutNode, LeafNode } from "./layoutTree";

/** DeepSeek's web app. The terminal-grid browser pane opens here. */
export const DEEPSEEK_GUI_HOME = "https://chat.deepseek.com/";

export const BROWSER_PANE_URL_ERROR = "browser pane only opens public https pages";

export function isBrowserLeaf(node: LeafNode): boolean {
  return node.kind === "browser" || node.harness === "browser";
}

export function stampLeaf(
  root: LayoutNode,
  leafId: string,
  patch: Partial<Pick<LeafNode, "kind" | "url" | "harness" | "cwd">>,
): LayoutNode {
  if (root.type === "leaf") {
    return root.id === leafId ? { ...root, ...patch } : root;
  }
  return {
    ...root,
    first: stampLeaf(root.first, leafId, patch),
    second: stampLeaf(root.second, leafId, patch),
  };
}

function ipv4Blocked(host: string): boolean | null {
  const parts = host.split(".");
  if (parts.length !== 4 || parts.some((part) => !/^[0-9]{1,3}$/.test(part))) return null;
  const nums = parts.map((part) => Number(part));
  if (nums.some((n) => n > 255)) return null;
  const [a, b] = nums;
  if (a === 0 || a === 10 || a === 127 || a >= 224) return true;
  if (a === 169 && b === 254) return true;
  if (a === 172 && b >= 16 && b <= 31) return true;
  if (a === 192 && b === 168) return true;
  if (a === 192 && b === 0 && nums[2] === 2) return true;
  if (a === 198 && b === 51 && nums[2] === 100) return true;
  if (a === 203 && b === 0 && nums[2] === 113) return true;
  return false;
}

function ipv6Blocked(host: string): boolean {
  const bare = host.replace(/^\[|\]$/g, "").toLowerCase();
  if (!bare.includes(":")) return false;
  if (bare.startsWith("::ffff:")) return true;
  if (bare === "::" || bare === "::1") return true;
  if (/^fe[89ab]/.test(bare)) return true;
  return bare.startsWith("fc") || bare.startsWith("fd");
}

function hostBlocked(hostname: string): boolean {
  const host = hostname.replace(/\.$/, "").toLowerCase();
  if (!host || host === "localhost" || host.endsWith(".localhost") || host === "tauri.localhost" || host.endsWith(".tauri.localhost")) {
    return true;
  }
  const v4 = ipv4Blocked(host);
  if (v4 !== null) return v4;
  return ipv6Blocked(host);
}

/** `null` when the address may be opened. The desktop command applies the same rule. */
export function browserPaneUrlError(raw: string): string | null {
  const text = raw.trim();
  if (!text || text.length > 2048) return BROWSER_PANE_URL_ERROR;
  let url: URL;
  try {
    url = new URL(text);
  } catch {
    return BROWSER_PANE_URL_ERROR;
  }
  if (url.protocol !== "https:" || url.username || url.password || hostBlocked(url.hostname)) {
    return BROWSER_PANE_URL_ERROR;
  }
  return null;
}
