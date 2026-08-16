import { useEffect, useRef, useState } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { listen } from "@tauri-apps/api/event";
import "@xterm/xterm/css/xterm.css";
import { invoke } from "../../../lib/tauri";
import type { PtySessionStatus } from "./types";

type TerminalState = "pending" | "running" | "exited" | "missing";

function decodeBase64(raw: string): Uint8Array {
  const bytes = atob(raw);
  const buf = new Uint8Array(bytes.length);
  for (let i = 0; i < bytes.length; i += 1) buf[i] = bytes.charCodeAt(i);
  return buf;
}

/** CSI ? 1000/1002/1003/1006 h = mouse tracking; 1049/1047/47 = alt screen. */
function applyDecPrivateModes(chunk: string, modes: { mouse: boolean; alt: boolean }): void {
  const re = /\x1b\[\?([\d;]+)([hl])/g;
  let match: RegExpExecArray | null;
  while ((match = re.exec(chunk)) !== null) {
    const on = match[2] === "h";
    for (const mode of match[1].split(";")) {
      if (mode === "1000" || mode === "1002" || mode === "1003" || mode === "1006" || mode === "1015" || mode === "1016") {
        modes.mouse = on;
      }
      if (mode === "1049" || mode === "1047" || mode === "47") {
        modes.alt = on;
      }
    }
  }
}

function wheelToPtyData(deltaY: number, mouseTracking: boolean): string {
  const steps = Math.max(1, Math.min(8, Math.round(Math.abs(deltaY) / 40) || 1));
  const up = deltaY < 0;
  if (mouseTracking) {
    const button = up ? 64 : 65;
    return Array.from({ length: steps }, () => `\x1b[<${button};1;1M`).join("");
  }
  return (up ? "\x1b[5~" : "\x1b[6~").repeat(Math.min(steps, 3));
}

/**
 * Renders one in-app PTY session (see `src-tauri/src/pty.rs`) via xterm.js.
 * `sessionId` must already be spawned (`pty_spawn` / `hub_relaunch_harness_embedded`)
 * before this mounts — this component only attaches to its output/exit
 * events and forwards keystrokes/resizes back, it never spawns anything
 * itself.
 *
 * #161 truthfulness: on mount it queries `pty_session_status` so a session
 * that already exited (fast-failing harness CLI, killed by a later replace)
 * still shows its retained output and exit reason instead of a silently
 * blank terminal; a session that no longer exists renders an explicit
 * error state instead of nothing. IPC failures on write/resize are surfaced
 * through `onError` instead of being swallowed.
 */
export default function EmbeddedTerminal({
  sessionId,
  onExit,
  onError,
}: {
  sessionId: string;
  onExit?: (detail: string) => void;
  onError?: (detail: string) => void;
}) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const wrapperRef = useRef<HTMLDivElement | null>(null);
  const [state, setState] = useState<TerminalState>("pending");
  const [failureDetail, setFailureDetail] = useState("");
  const isFocusedRef = useRef(false);

  useEffect(() => {
    let disposed = false;
    let term: Terminal | null = null;
    let fit: FitAddon | null = null;
    let writeSub: { dispose(): void } | null = null;
    let resizeRafId: number | null = null;
    const unlistenPromises: Promise<() => void>[] = [];
    const reportedError = { current: false };
    const decModes = { mouse: false, alt: false };
    const latin1 = new TextDecoder("latin-1");

    const noteOutput = (bytes: Uint8Array) => {
      applyDecPrivateModes(latin1.decode(bytes), decModes);
    };

    const handleWindowClick = (e: MouseEvent) => {
      if (wrapperRef.current && wrapperRef.current.contains(e.target as Node)) {
        isFocusedRef.current = true;
      } else {
        isFocusedRef.current = false;
      }
    };
    window.addEventListener("mousedown", handleWindowClick, true);

    const reportErrorOnce = (detail: string) => {
      if (reportedError.current) return;
      reportedError.current = true;
      onError?.(detail);
    };

    const resizeSync = () => {
      if (disposed || !containerRef.current || !term || !fit) return;
      if (containerRef.current.clientWidth <= 0 || containerRef.current.clientHeight <= 0) return;

      if (resizeRafId !== null) {
        cancelAnimationFrame(resizeRafId);
      }

      resizeRafId = requestAnimationFrame(() => {
        if (disposed || !containerRef.current || !term || !fit) return;
        try {
          fit.fit();
          if (term.rows > 0 && term.cols > 0) {
            void invoke("pty_resize", { sessionId, rows: term.rows, cols: term.cols }).catch(
              (cause) => reportErrorOnce(String(cause).replace(/^Error:\s*/, "")),
            );
          }
        } catch {
          // Ignore transient resize errors while elements are hidden or resizing
        }
      });
    };

    const writeTail = (b64: string) => {
      if (!b64 || !term) return;
      try {
        const bytes = decodeBase64(b64);
        noteOutput(bytes);
        term.write(bytes);
      } catch {
        // Ignore corrupted payload or closed terminal writes
      }
    };

    const writeExitLine = (detail: string) => {
      if (!term) return;
      try {
        term.write(`\r\n\x1b[90m[${detail}]\x1b[0m\r\n`);
      } catch {
        // Terminal might already be disposed
      }
    };

    const resizeObserver = new ResizeObserver(() => {
      resizeSync();
    });

    const attach = async () => {
      try {
        const firstStatus = await invoke<PtySessionStatus>("pty_session_status", { sessionId });
        if (disposed) return;

        if (!firstStatus.found) {
          const detail =
            "Terminal session not found — it may have failed to start or been replaced. Close and try again.";
          setFailureDetail(detail);
          setState("missing");
          reportErrorOnce(detail);
          return;
        }

        if (containerRef.current) {
          term = new Terminal({
            convertEol: true,
            cursorBlink: true,
            fontSize: 13,
            fontFamily: "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
            theme: { background: "#0b0f14" },
            scrollback: 5000,
            scrollSensitivity: 1,
            fastScrollSensitivity: 5,
          });

          // Focused wheel: Claude (primary buffer, no mouse mode) keeps
          // native .xterm-viewport overflow scroll. Grok's fullscreen TUI
          // enables mouse tracking / alt-screen — returning true here used
          // to *eat* the event so xterm never sent SGR wheel CSI, and there
          // is no local scrollback. Forward wheel to the PTY instead.
          const opened = term;
          opened.attachCustomWheelEventHandler((e: WheelEvent) => {
            if (!isFocusedRef.current) {
              return false;
            }
            const grokSession = sessionId.startsWith("harness-terminal:grok:");
            const tuiOwnsWheel =
              grokSession
              || decModes.mouse
              || decModes.alt
              || opened.buffer.active.type === "alternate";
            if (tuiOwnsWheel) {
              e.preventDefault();
              e.stopPropagation();
              const data = wheelToPtyData(e.deltaY, grokSession || decModes.mouse || decModes.alt);
              void invoke("pty_write", { sessionId, data }).catch((cause) =>
                reportErrorOnce(String(cause).replace(/^Error:\s*/, "")),
              );
              return true;
            }
            e.stopPropagation();
            return true;
          });

          fit = new FitAddon();
          term.loadAddon(fit);
          term.open(containerRef.current);
          // Only fit if the container has non-zero dimensions
          if (containerRef.current.clientWidth > 0 && containerRef.current.clientHeight > 0) {
            try {
              fit.fit();
            } catch {
              // ignore fit error on unrendered elements
            }
          }
        }

        if (firstStatus.exited) {
          // Fast exit happened before we attached: show the retained output
          // and the real exit reason instead of a silently blank terminal.
          writeTail(firstStatus.output_tail_b64);
          const detail = firstStatus.exit_detail ?? "exited";
          writeExitLine(detail);
          setState("exited");
          onExit?.(detail);
          return;
        }

        setState("running");
        writeTail(firstStatus.output_tail_b64);

        if (term) {
          writeSub = term.onData((data) => {
            void invoke("pty_write", { sessionId, data }).catch((cause) =>
              reportErrorOnce(String(cause).replace(/^Error:\s*/, "")),
            );
          });
        }

        if (containerRef.current) {
          resizeObserver.observe(containerRef.current);
        }
        resizeSync();

        unlistenPromises.push(
          listen<string>(`pty-output:${sessionId}`, (event) => {
            if (disposed) return;
            try {
              const bytes = decodeBase64(event.payload);
              noteOutput(bytes);
              term?.write(bytes);
            } catch {
              // Ignore corrupted payload or closed terminal writes
            }
          }),
          listen<string>(`pty-exit:${sessionId}`, (event) => {
            if (disposed) return;
            writeExitLine(event.payload);
            setState("exited");
            onExit?.(event.payload);
          }),
        );

        // Close the attach race: an exit that lands between the first status
        // query and listener registration would otherwise be lost. The
        // second tail is a strict extension of the first, so only the suffix
        // is written — never duplicating bytes already shown.
        const latest = await invoke<PtySessionStatus>("pty_session_status", { sessionId });
        if (disposed) return;
        if (latest.exited) {
          const first = decodeBase64(firstStatus.output_tail_b64);
          const second = decodeBase64(latest.output_tail_b64);
          if (second.length > first.length) {
            const prefixMatches =
              first.length === 0 ||
              second.subarray(0, first.length).every((byte, index) => byte === first[index]);
            term?.write(second.subarray(prefixMatches ? first.length : 0));
          }
          const detail = latest.exit_detail ?? "exited";
          writeExitLine(detail);
          setState("exited");
          onExit?.(detail);
        }
      } catch (cause) {
        if (disposed) return;
        const detail = String(cause).replace(/^Error:\s*/, "") || "Failed to connect to terminal session.";
        setFailureDetail(detail);
        setState("missing");
        reportErrorOnce(detail);
      }
    };

    void attach();

    return () => {
      disposed = true;
      window.removeEventListener("mousedown", handleWindowClick, true);
      if (resizeRafId !== null) {
        cancelAnimationFrame(resizeRafId);
      }
      writeSub?.dispose();
      resizeObserver.disconnect();
      for (const promise of unlistenPromises) {
        void promise.then((unlisten) => unlisten());
      }
      try {
        term?.dispose();
      } catch {
        // Ignore errors during terminal teardown
      }
    };
    // sessionId identifies the pty this instance is attached to; a change
    // means a different session, so a fresh terminal/subscriptions is correct.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sessionId]);

  return (
    <div
      ref={wrapperRef}
      style={{
        position: "relative",
        width: "100%",
        height: "100%",
        minHeight: "320px",
        borderRadius: "8px",
        overflow: "hidden",
        background: "#0b0f14",
      }}
      onClick={() => {
        isFocusedRef.current = true;
      }}
    >
      <div ref={containerRef} style={{ width: "100%", height: "100%", position: "relative" }} />
      {state === "pending" && (
        <div
          style={{
            position: "absolute",
            inset: 0,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            color: "var(--text-muted)",
            fontSize: "0.82rem",
            pointerEvents: "none",
          }}
        >
          Connecting to terminal…
        </div>
      )}
      {state === "missing" && (
        <div
          style={{
            position: "absolute",
            inset: 0,
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            gap: "0.4rem",
            padding: "1rem",
            textAlign: "center",
          }}
        >
          <div style={{ color: "#fecaca", fontWeight: 600, fontSize: "0.9rem" }}>
            Terminal session not found
          </div>
          <div style={{ color: "var(--text-muted)", fontSize: "0.78rem", maxWidth: "28rem" }}>
            {failureDetail}
          </div>
        </div>
      )}
      {state === "exited" && (
        <div
          style={{
            position: "absolute",
            top: "0.35rem",
            right: "0.5rem",
            padding: "0.15rem 0.5rem",
            borderRadius: "999px",
            fontSize: "0.68rem",
            fontWeight: 700,
            letterSpacing: "0.03em",
            textTransform: "uppercase",
            color: "#e2e8f0",
            border: "1px solid rgba(148, 163, 184, 0.5)",
            background: "rgba(30, 41, 59, 0.75)",
          }}
        >
          exited
        </div>
      )}
    </div>
  );
}
