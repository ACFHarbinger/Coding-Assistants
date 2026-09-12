"""Offline checks for the Ableton Remote Script.

Ableton Live is not on CI. This proves the script is valid Python 3
outside Live (`Live` / `_Framework` import fails and the dummy
ControlSurface is used) and that each op runs against a stand-in LOM.
"""

from __future__ import annotations

import importlib.util
import py_compile
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent


def load_bridge():
    spec = importlib.util.spec_from_file_location(
        "coding_assistants_ableton", HERE / "__init__.py"
    )
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class FakeClipSlot:
    def __init__(self, has_clip=False):
        self.has_clip = has_clip
        self.fired = False

    def fire(self):
        self.fired = True


class FakeTrack:
    def __init__(self, name, kind="midi", slots=0, clip_on=()):
        self.name = name
        self.has_midi_input = kind == "midi"
        self.has_audio_input = kind == "audio"
        self.is_foldable = kind == "group"
        self.mute = False
        self.solo = False
        self.can_be_armed = kind in ("midi", "audio")
        self.arm = False
        self.clip_slots = [FakeClipSlot(has_clip=(i in clip_on)) for i in range(slots)]


class FakeScene:
    def __init__(self, name, clip_slots):
        self.name = name
        self.clip_slots = clip_slots
        self.fired = False

    def fire(self):
        self.fired = True


class FakeSong:
    def __init__(self):
        self.name = "Smoke Set"
        self.file_path = "/tmp/smoke.als"
        self.tempo = 120.0
        self.signature_numerator = 4
        self.signature_denominator = 4
        self.is_playing = False
        midi = FakeTrack("Drums", "midi", slots=2, clip_on=(0,))
        audio = FakeTrack("Vocals", "audio", slots=2)
        self.tracks = [midi, audio]
        self.scenes = [
            FakeScene("Intro", [midi.clip_slots[0], audio.clip_slots[0]]),
            FakeScene("Drop", [midi.clip_slots[1], audio.clip_slots[1]]),
        ]

    def create_midi_track(self, index):
        track = FakeTrack("MIDI", "midi", slots=len(self.scenes))
        self.tracks.insert(index, track)
        return track


def fail(label, detail):
    print("SMOKE FAILED at %s: %s" % (label, detail))
    sys.exit(1)


def expect(cond, label, detail=""):
    if not cond:
        fail(label, detail)


def main() -> None:
    py_compile.compile(str(HERE / "__init__.py"), doraise=True)
    print("plugins/ableton/__init__.py compiles")

    bridge = load_bridge()
    expect(not bridge._IN_LIVE, "dummy ControlSurface", "Live was importable")
    song = FakeSong()

    summary = bridge._op_get_song_summary(song, {})
    expect(summary["tempo"] == 120.0, "get_song_summary", summary)
    expect(summary["track_count"] == 2, "get_song_summary tracks", summary)
    expect(summary["scene_count"] == 2, "get_song_summary scenes", summary)

    tracks = bridge._op_list_tracks(song, {})
    expect(tracks[0]["kind"] == "midi" and tracks[1]["kind"] == "audio", "list_tracks", tracks)

    scenes = bridge._op_list_scenes(song, {})
    expect(scenes[0]["clip_count"] == 1, "list_scenes", scenes)

    fired = bridge._op_fire_clip(song, {"track_index": 0, "scene_index": 0})
    expect(fired["fired"] and song.tracks[0].clip_slots[0].fired, "fire_clip", fired)

    scene = bridge._op_fire_scene(song, {"scene_index": 1})
    expect(scene["fired"] and song.scenes[1].fired, "fire_scene", scene)

    tempo = bridge._op_set_tempo(song, {"bpm": 96})
    expect(tempo["tempo"] == 96.0, "set_tempo", tempo)
    try:
        bridge._op_set_tempo(song, {"bpm": 5})
        fail("set_tempo range", "accepted 5 BPM")
    except RuntimeError:
        pass

    created = bridge._op_create_midi_track(song, {"name": "Bass"})
    expect(created["name"] == "Bass" and created["index"] == 2, "create_midi_track", created)

    renamed = bridge._op_set_track_name(song, {"track_index": 2, "name": "Sub"})
    expect(renamed["name"] == "Sub", "set_track_name", renamed)
    try:
        bridge._op_set_track_name(song, {"track_index": 99, "name": "x"})
        fail("set_track_name range", "accepted 99")
    except RuntimeError:
        pass

    lom = bridge._op_run_lom(song, {"code": "print('hi')\nresult = song.tempo"})
    expect("hi" in lom["stdout"] and "96" in lom["result"], "run_lom", lom)

    expect("run_lom" in bridge._OPS, "ops table")
    print("SMOKE OK")


if __name__ == "__main__":
    main()
