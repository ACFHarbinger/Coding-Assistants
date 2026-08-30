"""Coding-Assistants Unreal Engine bridge.

Unreal auto-runs `init_unreal.py` from any `Content/Python/` folder at
editor startup when the *Python Editor Script Plugin* is enabled. This one
opens a localhost line-JSON TCP server that `crates/mcp-unreal`
(`coding-assistants-mcp-unreal`) connects to:

    {"op": "<tool>", "args": {...}}\n  ->  {"ok": true, "result": ...}\n
                                     or  {"ok": false, "error": "..."}\n

`unreal.*` is game-thread only, so the socket thread queues each job and a
slate post-tick callback runs it on the game thread; the socket thread
blocks on the reply.

Install: copy this file to `<YourProject>/Content/Python/init_unreal.py`
(create the folders if needed), enable *Edit > Plugins > Python Editor
Script Plugin*, restart the editor. See plugins/unreal/README.md.
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
    import unreal

    _IN_UNREAL = True
except ImportError:  # allow py_compile / ruff outside the editor
    unreal = None
    _IN_UNREAL = False

PORT = 9768
_HOST = "127.0.0.1"

_jobs: "queue.Queue[tuple]" = queue.Queue()
_tick_handle = None
_server_thread: "threading.Thread | None" = None


# ----------------------------------------------------------------- ops


def _actor_subsystem():
    return unreal.get_editor_subsystem(unreal.EditorActorSubsystem)


def _level_subsystem():
    return unreal.get_editor_subsystem(unreal.LevelEditorSubsystem)


def _vec(triple):
    return unreal.Vector(float(triple[0]), float(triple[1]), float(triple[2]))


def _resolve_class(class_path: str):
    if "/" not in class_path and hasattr(unreal, class_path):
        return getattr(unreal, class_path)
    loaded = unreal.EditorAssetLibrary.load_blueprint_class(class_path)
    if loaded is None:
        raise RuntimeError(f"could not resolve class {class_path!r}")
    return loaded


def _find_actor(label: str):
    for actor in _actor_subsystem().get_all_level_actors():
        if actor.get_actor_label() == label:
            return actor
    raise RuntimeError(f"no actor with label {label!r}")


def _op_get_editor_summary(_args):
    return {
        "project_dir": unreal.SystemLibrary.get_project_directory(),
        "engine_version": unreal.SystemLibrary.get_engine_version(),
        "current_level": str(_level_subsystem().get_current_level().get_path_name()),
        "actor_count": len(_actor_subsystem().get_all_level_actors()),
    }


def _op_list_actors(args):
    limit = int(args.get("limit", 200))
    out = []
    for actor in _actor_subsystem().get_all_level_actors()[:limit]:
        loc = actor.get_actor_location()
        out.append(
            {
                "label": actor.get_actor_label(),
                "class": actor.get_class().get_name(),
                "location": [loc.x, loc.y, loc.z],
            }
        )
    return out


def _op_spawn_actor(args):
    cls = _resolve_class(args["class_path"])
    loc = _vec(args.get("location") or (0, 0, 0))
    actor = _actor_subsystem().spawn_actor_from_class(cls, loc, unreal.Rotator(0, 0, 0))
    if actor is None:
        raise RuntimeError("spawn_actor_from_class returned None")
    if args.get("label"):
        actor.set_actor_label(args["label"])
    return {"label": actor.get_actor_label()}


def _op_destroy_actor(args):
    actor = _find_actor(args["label"])
    _actor_subsystem().destroy_actor(actor)
    return {"destroyed": args["label"]}


def _op_set_actor_transform(args):
    actor = _find_actor(args["label"])
    if "location" in args:
        actor.set_actor_location(_vec(args["location"]), False, False)
    if "rotation" in args:
        r = args["rotation"]
        actor.set_actor_rotation(unreal.Rotator(float(r[0]), float(r[1]), float(r[2])), False)
    if "scale" in args:
        actor.set_actor_scale3d(_vec(args["scale"]))
    loc = actor.get_actor_location()
    return {"label": args["label"], "location": [loc.x, loc.y, loc.z]}


def _op_list_assets(args):
    path = args.get("path", "/Game")
    recursive = bool(args.get("recursive", False))
    return list(unreal.EditorAssetLibrary.list_assets(path, recursive, False))


def _op_save_level(_args):
    _level_subsystem().save_current_level()
    return {"saved": str(_level_subsystem().get_current_level().get_path_name())}


def _op_run_python(args):
    out = io.StringIO()
    ns: dict = {"unreal": unreal}
    with redirect_stdout(out):
        exec(compile(args["code"], "<coding-assistants>", "exec"), ns)  # noqa: S102
    payload = {"stdout": out.getvalue()}
    if "result" in ns:
        payload["result"] = repr(ns["result"])
    return payload


_OPS = {
    "get_editor_summary": _op_get_editor_summary,
    "list_actors": _op_list_actors,
    "spawn_actor": _op_spawn_actor,
    "destroy_actor": _op_destroy_actor,
    "set_actor_transform": _op_set_actor_transform,
    "list_assets": _op_list_assets,
    "save_level": _op_save_level,
    "run_python": _op_run_python,
}


# ----------------------------------------------------- game-thread pump


def _pump(_delta_seconds: float = 0.0):
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
    except Exception as exc:  # noqa: BLE001 — surface every failure to the client
        reply.put(
            {
                "ok": False,
                "error": f"{type(exc).__name__}: {exc}",
                "traceback": traceback.format_exc(),
            }
        )


# ------------------------------------------------------------------ server


class _BridgeServer(threading.Thread):
    def __init__(self, port: int):
        super().__init__(daemon=True)
        self.port = port

    def run(self):
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        sock.bind((_HOST, self.port))
        sock.listen(4)
        unreal.log(f"[coding-assistants] Unreal bridge listening on {_HOST}:{self.port}")
        while True:
            try:
                conn, _ = sock.accept()
            except OSError:
                break
            threading.Thread(target=self._serve, args=(conn,), daemon=True).start()

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
            return {"ok": False, "error": "op timed out on the game thread"}


def _start():
    global _tick_handle, _server_thread
    if _server_thread is not None:
        return
    _server_thread = _BridgeServer(PORT)
    _server_thread.start()
    _tick_handle = unreal.register_slate_post_tick_callback(_pump)


if _IN_UNREAL:
    _start()
