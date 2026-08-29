"""Coding-Assistants Krita bridge plugin.

Opens a localhost line-JSON TCP server that `crates/mcp-krita`
(`coding-assistants-mcp-krita`) connects to. One request per line:

    {"op": "<tool>", "args": {...}}\n  ->  {"ok": true, "result": ...}\n
                                     or  {"ok": false, "error": "..."}\n

Krita's scripting API is main-thread only, so the socket thread hands each
job to a QTimer pump on the GUI thread and blocks on the reply.

Install: copy this folder to your `pykrita` resources dir as
`coding_assistants_bridge/` alongside a `coding_assistants_bridge.desktop`,
then enable it in Settings > Configure Krita > Python Plugin Manager.
See plugins/krita/README.md.
"""

from __future__ import annotations

import io
import json
import queue
import socket
import threading
import traceback
from contextlib import redirect_stdout

try:
    from krita import Extension, InfoObject, Krita  # type: ignore
    from PyQt5.QtCore import QTimer  # type: ignore

    _IN_KRITA = True
except ImportError:  # allow py_compile / ruff outside Krita
    Extension = object  # type: ignore
    InfoObject = None  # type: ignore
    Krita = None  # type: ignore
    QTimer = None  # type: ignore
    _IN_KRITA = False

DEFAULT_PORT = 9766
_HOST = "127.0.0.1"
PLUGIN_ID = "coding_assistants_bridge"


# ----------------------------------------------------------------- ops
# Each runs on the GUI thread, takes the request `args`, returns a
# JSON-serialisable value or raises.


def _active_doc():
    doc = Krita.instance().activeDocument()
    if doc is None:
        raise RuntimeError("no active document")
    return doc


def _op_get_document_summary(_args):
    doc = _active_doc()
    return {
        "name": doc.name(),
        "width": doc.width(),
        "height": doc.height(),
        "color_model": doc.colorModel(),
        "color_depth": doc.colorDepth(),
        "resolution": doc.resolution(),
        "layer_count": len(doc.topLevelNodes()),
    }


def _node_info(node):
    return {
        "name": node.name(),
        "type": node.type(),
        "visible": node.visible(),
        "opacity": round(node.opacity() / 255 * 100, 1),
    }


def _op_list_layers(_args):
    return [_node_info(n) for n in _active_doc().topLevelNodes()]


def _op_create_document(args):
    width = int(args["width"])
    height = int(args["height"])
    name = args.get("name") or "Untitled"
    model = args.get("color_model", "RGBA")
    resolution = float(args.get("resolution", 300.0))
    app = Krita.instance()
    doc = app.createDocument(width, height, name, model, "U8", "", resolution)
    app.activeWindow().addView(doc)
    doc.refreshProjection()
    return {"name": doc.name(), "width": width, "height": height}


def _op_create_paint_layer(args):
    doc = _active_doc()
    name = args.get("name") or "Paint Layer"
    node = doc.createNode(name, "paintlayer")
    root = doc.rootNode()
    root.addChildNode(node, doc.activeNode())
    doc.refreshProjection()
    return {"name": node.name()}


def _find_node(doc, name):
    node = doc.nodeByName(name)
    if node is None:
        raise RuntimeError(f"layer {name!r} not found")
    return node


def _op_set_layer_visible(args):
    doc = _active_doc()
    node = _find_node(doc, args["name"])
    node.setVisible(bool(args["visible"]))
    doc.refreshProjection()
    return {"name": node.name(), "visible": node.visible()}


def _op_set_layer_opacity(args):
    doc = _active_doc()
    node = _find_node(doc, args["name"])
    pct = max(0.0, min(100.0, float(args["opacity"])))
    node.setOpacity(round(pct / 100 * 255))
    doc.refreshProjection()
    return {"name": node.name(), "opacity": pct}


def _op_export_document(args):
    doc = _active_doc()
    path = args["path"]
    ok = doc.exportImage(path, InfoObject())
    if not ok:
        raise RuntimeError(f"Krita refused to export to {path!r} (unsupported format?)")
    return {"exported": path}


def _op_run_python(args):
    out = io.StringIO()
    ns: dict = {"Krita": Krita}
    with redirect_stdout(out):
        exec(compile(args["code"], "<coding-assistants>", "exec"), ns)  # noqa: S102
    payload = {"stdout": out.getvalue()}
    if "result" in ns:
        payload["result"] = repr(ns["result"])
    return payload


_OPS = {
    "get_document_summary": _op_get_document_summary,
    "list_layers": _op_list_layers,
    "create_document": _op_create_document,
    "create_paint_layer": _op_create_paint_layer,
    "set_layer_visible": _op_set_layer_visible,
    "set_layer_opacity": _op_set_layer_opacity,
    "export_document": _op_export_document,
    "run_python": _op_run_python,
}

_jobs: "queue.Queue[tuple]" = queue.Queue()


def _pump():
    try:
        op, args, reply = _jobs.get_nowait()
    except queue.Empty:
        return
    try:
        handler = _OPS.get(op)
        if handler is None:
            reply.put({"ok": False, "error": f"unknown op {op!r}"})
        else:
            reply.put({"ok": True, "result": handler(args)})
    except Exception as exc:  # noqa: BLE001 — report every failure to the client
        reply.put(
            {
                "ok": False,
                "error": f"{type(exc).__name__}: {exc}",
                "traceback": traceback.format_exc(),
            }
        )


class _BridgeServer(threading.Thread):
    def __init__(self, port: int):
        super().__init__(daemon=True)
        self.port = port
        self._stop = threading.Event()

    def run(self):
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        sock.bind((_HOST, self.port))
        sock.settimeout(0.5)
        sock.listen(4)
        print(f"[coding-assistants] Krita bridge listening on {_HOST}:{self.port}")
        while not self._stop.is_set():
            try:
                conn, _ = sock.accept()
            except socket.timeout:
                continue
            except OSError:
                break
            threading.Thread(target=self._serve, args=(conn,), daemon=True).start()
        sock.close()

    def _serve(self, conn: socket.socket):
        with conn:
            conn.settimeout(60)
            buf = b""
            try:
                while b"\n" not in buf:
                    chunk = conn.recv(4096)
                    if not chunk:
                        return
                    buf += chunk
            except socket.timeout:
                return
            reply = self._handle(buf.partition(b"\n")[0])
            conn.sendall((json.dumps(reply) + "\n").encode("utf-8"))

    def _handle(self, line: bytes) -> dict:
        try:
            msg = json.loads(line.decode("utf-8"))
        except (ValueError, UnicodeDecodeError) as exc:
            return {"ok": False, "error": f"invalid request JSON: {exc}"}
        reply: "queue.Queue[dict]" = queue.Queue()
        _jobs.put((msg.get("op"), msg.get("args") or {}, reply))
        try:
            return reply.get(timeout=120)
        except queue.Empty:
            return {"ok": False, "error": "op timed out on Krita's main thread"}

    def stop(self):
        self._stop.set()


class CodingAssistantsBridge(Extension):
    def __init__(self, parent):
        super().__init__(parent)
        self._server: "_BridgeServer | None" = None
        self._timer: "QTimer | None" = None

    def setup(self):
        port = DEFAULT_PORT
        self._server = _BridgeServer(port)
        self._server.start()
        self._timer = QTimer()
        self._timer.setInterval(50)
        self._timer.timeout.connect(_pump)
        self._timer.start()

    def createActions(self, window):  # noqa: N802 (Krita API name)
        pass


if _IN_KRITA:
    Krita.instance().addExtension(CodingAssistantsBridge(Krita.instance()))
