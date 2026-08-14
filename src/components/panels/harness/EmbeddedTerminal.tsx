import { useEffect, useRef } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { listen } from "@tauri-apps/api/event";
import "@xterm/xterm/css/xterm.css";
import { invoke } from "../../../lib/tauri";

/**
 * Renders one in-app PTY session (see `src-tauri/src/pty.rs`) via xterm.js.
 * `sessionId` must already be spawned (`pty_spawn` / `hub_relaunch_harness_embedded`)
 * before this mounts — this component only attaches to its output/exit
 * events and forwards keystrokes/resizes back, it never spawns anything
 * itself.
 */
export default function EmbeddedTerminal({
  sessionId,
  onExit,
}: {
  sessionId: string;
  onExit?: (detail: string) => void;
}) {
  const containerRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const term = new Terminal({
      convertEol: true,
      cursorBlink: true,
      fontSize: 13,
      fontFamily: "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
      theme: { background: "#0b0f14" },
    });
    const fit = new FitAddon();
    term.loadAddon(fit);
    if (containerRef.current) {
      term.open(containerRef.current);
      fit.fit();
    }

    const writeSub = term.onData((data) => {
      void invoke("pty_write", { sessionId, data }).catch(() => {});
    });

    const resizeSync = () => {
      fit.fit();
      void invoke("pty_resize", { sessionId, rows: term.rows, cols: term.cols }).catch(() => {});
    };
    const resizeObserver = new ResizeObserver(resizeSync);
    if (containerRef.current) resizeObserver.observe(containerRef.current);
    resizeSync();

    let disposed = false;
    const unlistenPromises = [
      listen<string>(`pty-output:${sessionId}`, (event) => {
        if (disposed) return;
        const bytes = atob(event.payload);
        const buf = new Uint8Array(bytes.length);
        for (let i = 0; i < bytes.length; i += 1) buf[i] = bytes.charCodeAt(i);
        term.write(buf);
      }),
      listen<string>(`pty-exit:${sessionId}`, (event) => {
        if (disposed) return;
        term.write(`\r\n\x1b[90m[${event.payload}]\x1b[0m\r\n`);
        onExit?.(event.payload);
      }),
    ];

    return () => {
      disposed = true;
      writeSub.dispose();
      resizeObserver.disconnect();
      for (const promise of unlistenPromises) {
        void promise.then((unlisten) => unlisten());
      }
      term.dispose();
    };
    // sessionId identifies the pty this instance is attached to; a change
    // means a different session, so a fresh terminal/subscriptions is correct.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sessionId]);

  return (
    <div
      ref={containerRef}
      style={{
        width: "100%",
        height: "100%",
        minHeight: "320px",
        borderRadius: "8px",
        overflow: "hidden",
        background: "#0b0f14",
        padding: "0.4rem",
      }}
    />
  );
}
