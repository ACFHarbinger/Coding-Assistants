"""Coding-Assistants Blender bridge addon.

Opens a localhost line-JSON TCP server that `crates/mcp-blender`
(`coding-assistants-mcp-blender`) connects to. One request per line:

    {"op": "<tool>", "args": {...}}\n  ->  {"ok": true, "result": ...}\n
                                     or  {"ok": false, "error": "..."}\n

`bpy` is only safe to touch on Blender's main thread, so the socket thread
hands each job to a `bpy.app.timers` callback and blocks on the reply.

Install: Edit > Preferences > Add-ons > Install..., pick this file, tick it.
Port defaults to 9765; change it in the add-on preferences (must match the
bridge's `--port`).
"""

from __future__ import annotations

import io
import json
import queue
import socket
import threading
import traceback
from contextlib import redirect_stdout

bl_info = {
    "name": "Coding-Assistants Bridge",
    "author": "Coding-Assistants",
    "version": (0, 1, 0),
    "blender": (3, 6, 0),
    "location": "Runs in the background once enabled",
    "description": "Line-JSON TCP bridge for the coding-assistants-mcp-blender MCP server",
    "category": "System",
}

try:
    import bpy
    import mathutils  # noqa: F401  (imported for side effects / availability check)
except ImportError:  # allows `python -m py_compile` / ruff outside Blender
    bpy = None

DEFAULT_PORT = 9765
_HOST = "127.0.0.1"

_server: "_BridgeServer | None" = None

_PRIMITIVE_OPS = {
    "cube": "mesh.primitive_cube_add",
    "uv_sphere": "mesh.primitive_uv_sphere_add",
    "ico_sphere": "mesh.primitive_ico_sphere_add",
    "cylinder": "mesh.primitive_cylinder_add",
    "cone": "mesh.primitive_cone_add",
    "plane": "mesh.primitive_plane_add",
    "torus": "mesh.primitive_torus_add",
}


# ---------------------------------------------------------------- operations
# Each takes the request `args` dict, runs on the main thread, returns a
# JSON-serialisable value or raises.


def _op_get_scene_summary(_args):
    scene = bpy.context.scene
    return {
        "scene": scene.name,
        "object_count": len(scene.objects),
        "active_object": scene.objects.active.name
        if getattr(scene.objects, "active", None)
        else None,
        "frame_start": scene.frame_start,
        "frame_end": scene.frame_end,
        "frame_current": scene.frame_current,
        "render_engine": scene.render.engine,
        "unit_system": scene.unit_settings.system,
    }


def _op_list_objects(_args):
    return [
        {"name": o.name, "type": o.type, "location": list(o.location)}
        for o in bpy.context.scene.objects
    ]


def _op_create_primitive(args):
    kind = args.get("kind")
    if kind not in _PRIMITIVE_OPS:
        raise ValueError(f"unknown primitive kind {kind!r}")
    location = tuple(args.get("location") or (0.0, 0.0, 0.0))
    size = float(args.get("size", 2.0))
    op = getattr(bpy.ops.mesh, _PRIMITIVE_OPS[kind].split(".", 1)[1])

    before = set(bpy.context.scene.objects.keys())
    # `size` vs `radius` differs by primitive; pass whichever the op accepts.
    kwargs = {"location": location}
    if "size" in op.get_rna_type().properties:
        kwargs["size"] = size
    elif "radius" in op.get_rna_type().properties:
        kwargs["radius"] = size
    op(**kwargs)
    new = set(bpy.context.scene.objects.keys()) - before
    name = next(iter(new)) if new else bpy.context.active_object.name
    if args.get("name"):
        bpy.data.objects[name].name = args["name"]
        name = args["name"]
    return {"name": name}


def _op_delete_object(args):
    name = args["name"]
    obj = bpy.data.objects.get(name)
    if obj is None:
        raise ValueError(f"object {name!r} not found")
    bpy.data.objects.remove(obj, do_unlink=True)
    return {"deleted": name}


def _op_export_scene(args):
    path = args["path"]
    selection_only = bool(args.get("selection_only", False))
    lower = path.lower()
    if lower.endswith((".glb", ".gltf")):
        bpy.ops.export_scene.gltf(filepath=path, use_selection=selection_only)
    elif lower.endswith(".obj"):
        bpy.ops.wm.obj_export(filepath=path, export_selected_objects=selection_only)
    elif lower.endswith(".fbx"):
        bpy.ops.export_scene.fbx(filepath=path, use_selection=selection_only)
    elif lower.endswith(".stl"):
        bpy.ops.wm.stl_export(filepath=path, export_selected_objects=selection_only)
    else:
        raise ValueError(f"unsupported export extension for {path!r}")
    return {"exported": path}


def _op_render_still(args):
    path = args["path"]
    bpy.context.scene.render.filepath = path
    bpy.ops.render.render(write_still=True)
    return {"rendered": path}


def _op_run_python(args):
    code = args["code"]
    out = io.StringIO()
    ns: dict = {"bpy": bpy}
    with redirect_stdout(out):
        exec(compile(code, "<coding-assistants>", "exec"), ns)  # noqa: S102
    payload = {"stdout": out.getvalue()}
    if "result" in ns:
        payload["result"] = repr(ns["result"])
    return payload


_OPS = {
    "get_scene_summary": _op_get_scene_summary,
    "list_objects": _op_list_objects,
    "create_primitive": _op_create_primitive,
    "delete_object": _op_delete_object,
    "export_scene": _op_export_scene,
    "render_still": _op_render_still,
    "run_python": _op_run_python,
}


# ------------------------------------------------------- main-thread pump
# Jobs are (op, args, reply_queue). The timer drains one per tick.

_jobs: "queue.Queue[tuple]" = queue.Queue()


def _pump():
    try:
        op, args, reply = _jobs.get_nowait()
    except queue.Empty:
        return 0.05
    try:
        handler = _OPS.get(op)
        if handler is None:
            reply.put({"ok": False, "error": f"unknown op {op!r}"})
        else:
            reply.put({"ok": True, "result": handler(args)})
    except Exception as exc:  # noqa: BLE001 — report every failure to the client
        reply.put({"ok": False, "error": f"{type(exc).__name__}: {exc}", "traceback": traceback.format_exc()})
    return 0.0


# ------------------------------------------------------------------ server


class _BridgeServer(threading.Thread):
    def __init__(self, port: int):
        super().__init__(daemon=True)
        self.port = port
        self._stop = threading.Event()
        self._sock: "socket.socket | None" = None

    def run(self):
        self._sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self._sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        self._sock.bind((_HOST, self.port))
        self._sock.settimeout(0.5)
        self._sock.listen(4)
        print(f"[coding-assistants] Blender bridge listening on {_HOST}:{self.port}")
        while not self._stop.is_set():
            try:
                conn, _ = self._sock.accept()
            except socket.timeout:
                continue
            except OSError:
                break
            threading.Thread(target=self._serve, args=(conn,), daemon=True).start()
        self._sock.close()

    def _serve(self, conn: socket.socket):
        with conn:
            buf = b""
            conn.settimeout(60)
            try:
                while b"\n" not in buf:
                    chunk = conn.recv(4096)
                    if not chunk:
                        return
                    buf += chunk
            except socket.timeout:
                return
            line, _, _ = buf.partition(b"\n")
            reply = self._handle(line)
            conn.sendall((json.dumps(reply) + "\n").encode("utf-8"))

    def _handle(self, line: bytes) -> dict:
        try:
            msg = json.loads(line.decode("utf-8"))
        except (ValueError, UnicodeDecodeError) as exc:
            return {"ok": False, "error": f"invalid request JSON: {exc}"}
        op = msg.get("op")
        args = msg.get("args") or {}
        reply: "queue.Queue[dict]" = queue.Queue()
        _jobs.put((op, args, reply))
        try:
            return reply.get(timeout=120)
        except queue.Empty:
            return {"ok": False, "error": f"op {op!r} timed out on Blender's main thread"}

    def stop(self):
        self._stop.set()


# ----------------------------------------------------------- addon glue


class CodingAssistantsBridgePrefs(bpy.types.AddonPreferences if bpy else object):
    bl_idname = __name__

    if bpy:
        port: bpy.props.IntProperty(  # type: ignore[valid-type]
            name="Port",
            default=DEFAULT_PORT,
            min=1024,
            max=65535,
            description="TCP port for the Coding-Assistants MCP bridge (must match --port)",
        )

    def draw(self, _context):
        self.layout.prop(self, "port")
        self.layout.label(text="Re-enable the add-on after changing the port.")


def _start(port: int):
    global _server
    _stop()
    _server = _BridgeServer(port)
    _server.start()
    bpy.app.timers.register(_pump, persistent=True)


def _stop():
    global _server
    if _server is not None:
        _server.stop()
        _server = None
    if bpy and bpy.app.timers.is_registered(_pump):
        bpy.app.timers.unregister(_pump)


def register():
    bpy.utils.register_class(CodingAssistantsBridgePrefs)
    prefs = bpy.context.preferences.addons[__name__].preferences
    _start(int(getattr(prefs, "port", DEFAULT_PORT)))


def unregister():
    _stop()
    bpy.utils.unregister_class(CodingAssistantsBridgePrefs)


if __name__ == "__main__" and bpy is not None:
    # `blender --python coding_assistants_bridge.py` (no install) for smoke runs.
    register()
