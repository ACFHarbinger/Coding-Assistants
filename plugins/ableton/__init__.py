"""Coding-Assistants Ableton Live MIDI Remote Script.

Opens a localhost line-JSON TCP server that `crates/mcp/ableton`
(`coding-assistants-mcp ableton`) connects to. One request per line:

    {"op": "<tool>", "args": {...}}\\n  ->  {"ok": true, "result": ...}\\n
                                     or  {"ok": false, "error": "..."}\\n

The Live Object Model is main-thread only, so the socket thread queues
each job and `update_display` (Live calls this ~10 Hz) pumps it.

Install: copy this folder to
`~/Music/Ableton/User Library/Remote Scripts/CodingAssistants/`
then pick **CodingAssistants** in Preferences → Link/MIDI → Control
Surface. See plugins/ableton/README.md.
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
    import Live  # type: ignore

    try:
        from _Framework.ControlSurface import ControlSurface  # type: ignore
    except ImportError:  # Live 12 prefers ableton.v2; _Framework still on 11
        from ableton.v2.control_surface import ControlSurface  # type: ignore

    _IN_LIVE = True
except ImportError:  # allow py_compile / ruff outside Live
    Live = None  # type: ignore
    _IN_LIVE = False

    class ControlSurface(object):  # type: ignore
        def __init__(self, c_instance=None):
            self._c_instance = c_instance

        def song(self):
            return None

        def log_message(self, message):
            print(message)

        def disconnect(self):
            pass

        def update_display(self):
            pass


DEFAULT_PORT = 9770
_HOST = "127.0.0.1"


def create_instance(c_instance):
    return CodingAssistantsBridge(c_instance)


def _track_kind(track):
    if getattr(track, "is_foldable", False):
        return "group"
    if track.has_midi_input:
        return "midi"
    if track.has_audio_input:
        return "audio"
    return "other"


def _track_info(index, track):
    info = {
        "index": index,
        "name": track.name,
        "kind": _track_kind(track),
        "mute": bool(track.mute),
        "solo": bool(track.solo),
    }
    if getattr(track, "can_be_armed", False):
        info["arm"] = bool(track.arm)
    return info


def _nth(seq, index, label):
    if index < 0 or index >= len(seq):
        raise RuntimeError("%s %s out of range" % (label, index))
    return seq[index]


def _op_get_song_summary(song, _args):
    return {
        "name": song.name,
        "file_path": song.file_path,
        "tempo": song.tempo,
        "signature_numerator": song.signature_numerator,
        "signature_denominator": song.signature_denominator,
        "is_playing": bool(song.is_playing),
        "track_count": len(song.tracks),
        "scene_count": len(song.scenes),
    }


def _op_list_tracks(song, _args):
    return [_track_info(i, t) for i, t in enumerate(song.tracks)]


def _op_list_scenes(song, _args):
    scenes = []
    for i, scene in enumerate(song.scenes):
        clip_count = sum(1 for slot in scene.clip_slots if slot.has_clip)
        scenes.append({"index": i, "name": scene.name, "clip_count": clip_count})
    return scenes


def _op_fire_clip(song, args):
    track_index = int(args["track_index"])
    scene_index = int(args["scene_index"])
    track = _nth(song.tracks, track_index, "track_index")
    slot = _nth(track.clip_slots, scene_index, "scene_index")
    slot.fire()
    return {
        "track_index": track_index,
        "scene_index": scene_index,
        "fired": True,
        "has_clip": bool(slot.has_clip),
    }


def _op_fire_scene(song, args):
    scene_index = int(args["scene_index"])
    scene = _nth(song.scenes, scene_index, "scene_index")
    scene.fire()
    return {"scene_index": scene_index, "fired": True}


def _op_set_tempo(song, args):
    bpm = float(args["bpm"])
    if bpm < 20.0 or bpm > 999.0:
        raise RuntimeError("tempo must be between 20 and 999 BPM")
    song.tempo = bpm
    return {"tempo": song.tempo}


def _op_create_midi_track(song, args):
    # LOM: 0 <= index <= len(tracks). len(tracks) appends. (-1 is a
    # community alias for "end" but is not in the published signature.)
    track = song.create_midi_track(len(song.tracks))
    name = args.get("name")
    if name:
        track.name = name
    return {"index": len(song.tracks) - 1, "name": track.name}


def _op_set_track_name(song, args):
    track = _nth(song.tracks, int(args["track_index"]), "track_index")
    track.name = args["name"]
    return {"index": int(args["track_index"]), "name": track.name}


def _op_run_lom(song, args):
    out = io.StringIO()
    ns = {"song": song, "Live": Live}
    with redirect_stdout(out):
        exec(compile(args["code"], "<coding-assistants>", "exec"), ns)  # noqa: S102
    payload = {"stdout": out.getvalue()}
    if "result" in ns:
        payload["result"] = repr(ns["result"])
    return payload


_OPS = {
    "get_song_summary": _op_get_song_summary,
    "list_tracks": _op_list_tracks,
    "list_scenes": _op_list_scenes,
    "fire_clip": _op_fire_clip,
    "fire_scene": _op_fire_scene,
    "set_tempo": _op_set_tempo,
    "create_midi_track": _op_create_midi_track,
    "set_track_name": _op_set_track_name,
    "run_lom": _op_run_lom,
}


class _BridgeServer(threading.Thread):
    def __init__(self, port, jobs):
        super(_BridgeServer, self).__init__(daemon=True)
        self.port = port
        self._jobs = jobs
        self._stop = threading.Event()
        self._sock = None

    def run(self):
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        try:
            sock.bind((_HOST, self.port))
        except OSError as exc:
            print(
                "[coding-assistants] Ableton bridge bind failed on %s:%s: %s"
                % (_HOST, self.port, exc)
            )
            return
        sock.settimeout(0.5)
        sock.listen(4)
        self._sock = sock
        while not self._stop.is_set():
            try:
                conn, _ = sock.accept()
            except socket.timeout:
                continue
            except OSError:
                break
            threading.Thread(target=self._serve, args=(conn,), daemon=True).start()
        sock.close()

    def _serve(self, conn):
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
            try:
                conn.sendall((json.dumps(reply) + "\n").encode("utf-8"))
            except OSError:
                return

    def _handle(self, line):
        try:
            msg = json.loads(line.decode("utf-8"))
        except (ValueError, UnicodeDecodeError) as exc:
            return {"ok": False, "error": "invalid request JSON: %s" % exc}
        reply = queue.Queue()
        self._jobs.put((msg.get("op"), msg.get("args") or {}, reply))
        try:
            return reply.get(timeout=120)
        except queue.Empty:
            return {"ok": False, "error": "op timed out on Live's main thread"}

    def stop(self):
        self._stop.set()
        if self._sock is not None:
            try:
                self._sock.close()
            except OSError:
                pass


class CodingAssistantsBridge(ControlSurface):
    def __init__(self, c_instance):
        ControlSurface.__init__(self, c_instance)
        self._jobs = queue.Queue()
        self._server = _BridgeServer(DEFAULT_PORT, self._jobs)
        self._server.start()
        self.log_message(
            "[coding-assistants] Ableton bridge listening on %s:%s"
            % (_HOST, DEFAULT_PORT)
        )

    def disconnect(self):
        if self._server is not None:
            self._server.stop()
        ControlSurface.disconnect(self)

    def update_display(self):
        ControlSurface.update_display(self)
        self._pump()

    def _pump(self):
        while True:
            try:
                op, args, reply = self._jobs.get_nowait()
            except queue.Empty:
                return
            try:
                handler = _OPS.get(op)
                if handler is None:
                    reply.put({"ok": False, "error": "unknown op %r" % (op,)})
                    continue
                song = self.song()
                if song is None:
                    raise RuntimeError("no Live set")
                reply.put({"ok": True, "result": handler(song, args)})
            except Exception as exc:  # noqa: BLE001 — report every failure to the client
                reply.put(
                    {
                        "ok": False,
                        "error": "%s: %s" % (type(exc).__name__, exc),
                        "traceback": traceback.format_exc(),
                    }
                )
