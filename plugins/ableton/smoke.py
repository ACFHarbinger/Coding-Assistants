"""Offline checks for the Ableton Remote Script.

Ableton Live is not on CI. This proves the script is valid Python 3
outside Live (`Live` / `_Framework` import fails and the dummy
ControlSurface is used) and that each op runs against a stand-in LOM.

The stand-ins expose **only** attributes the published Live Object Model
documents (`LOM_*_ATTRS` below). `get_song_summary` once read a
`Song.name` that does not exist in Live and the old fake happily
provided it — the allowlist is what keeps that class of bug from passing
offline again.
"""

from __future__ import annotations

import importlib.util
import py_compile
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent

# Live Object Model attributes used by the bridge (Live 11/12 docs).
LOM_SONG_ATTRS = {
    "file_path",
    "tempo",
    "signature_numerator",
    "signature_denominator",
    "is_playing",
    "tracks",
    "scenes",
    "create_midi_track",
}
LOM_TRACK_ATTRS = {
    "name",
    "has_midi_input",
    "has_audio_input",
    "is_foldable",
    "mute",
    "solo",
    "can_be_armed",
    "arm",
    "clip_slots",
}
LOM_SCENE_ATTRS = {"name", "clip_slots", "fire"}
LOM_CLIP_SLOT_ATTRS = {"has_clip", "fire"}


def load_bridge():
    spec = importlib.util.spec_from_file_location(
        "coding_assistants_ableton", HERE / "__init__.py"
    )
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class _Lom(object):
    """Base for stand-ins: any attribute outside the allowlist is a bug in
    the bridge (or in this smoke), exactly as it would be inside Live."""

    _ALLOWED: set = set()

    def __setattr__(self, key, value):
        if key not in self._ALLOWED and not key.startswith("_"):
            raise AttributeError("%s has no LOM attribute %r" % (type(self).__name__, key))
        object.__setattr__(self, key, value)


class FakeClipSlot(_Lom):
    _ALLOWED = LOM_CLIP_SLOT_ATTRS

    def __init__(self, has_clip=False):
        self.has_clip = has_clip
        self._fired = False

    def fire(self):
        self._fired = True


class FakeTrack(_Lom):
    _ALLOWED = LOM_TRACK_ATTRS

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


class FakeScene(_Lom):
    _ALLOWED = LOM_SCENE_ATTRS

    def __init__(self, name, clip_slots):
        self.name = name
        self.clip_slots = clip_slots
        self._fired = False

    def fire(self):
        self._fired = True


class FakeSong(_Lom):
    _ALLOWED = LOM_SONG_ATTRS

    def __init__(self, returns_new_track=True):
        self.file_path = "/tmp/Smoke Set.als"
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
        self._returns_new_track = returns_new_track

    def create_midi_track(self, index):
        track = FakeTrack("MIDI", "midi", slots=len(self.scenes))
        self.tracks.insert(index, track)
        # Live 11+ returns the Track; Live 10 returned None.
        return track if self._returns_new_track else None


def fail(label, detail):
    print("SMOKE FAILED at %s: %s" % (label, detail))
    sys.exit(1)


def expect(cond, label, detail=""):
    if not cond:
        fail(label, detail)


def check_ops(bridge):
    song = FakeSong()

    summary = bridge._op_get_song_summary(song, {})
    expect(summary["name"] == "Smoke Set", "get_song_summary name from file_path", summary)
    expect(summary["tempo"] == 120.0, "get_song_summary", summary)
    expect(summary["track_count"] == 2, "get_song_summary tracks", summary)
    expect(summary["scene_count"] == 2, "get_song_summary scenes", summary)
    expect(bridge._set_name("") == "Untitled", "unsaved set name")
    expect(bridge._set_name("C:\\Sets\\Jam.als") == "Jam", "windows set name")

    tracks = bridge._op_list_tracks(song, {})
    expect(tracks[0]["kind"] == "midi" and tracks[1]["kind"] == "audio", "list_tracks", tracks)

    scenes = bridge._op_list_scenes(song, {})
    expect(scenes[0]["clip_count"] == 1, "list_scenes", scenes)

    fired = bridge._op_fire_clip(song, {"track_index": 0, "scene_index": 0})
    expect(fired["fired"] and song.tracks[0].clip_slots[0]._fired, "fire_clip", fired)

    scene = bridge._op_fire_scene(song, {"scene_index": 1})
    expect(scene["fired"] and song.scenes[1]._fired, "fire_scene", scene)

    tempo = bridge._op_set_tempo(song, {"bpm": 96})
    expect(tempo["tempo"] == 96.0, "set_tempo", tempo)
    try:
        bridge._op_set_tempo(song, {"bpm": 5})
        fail("set_tempo range", "accepted 5 BPM")
    except RuntimeError:
        pass

    created = bridge._op_create_midi_track(song, {"name": "Bass"})
    expect(created["name"] == "Bass" and created["index"] == 2, "create_midi_track", created)

    legacy = FakeSong(returns_new_track=False)
    created = bridge._op_create_midi_track(legacy, {"name": "Keys"})
    expect(
        created["index"] == 2 and legacy.tracks[2].name == "Keys",
        "create_midi_track on a Live that returns None",
        created,
    )

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


def check_control_surface(bridge):
    """The pump must find the set whether the base class exposes `song` as
    a method (`_Framework`) or a property (`ableton.v2`)."""
    song = FakeSong()

    class MethodBase(bridge.ControlSurface):
        def song(self):
            return song

    class PropertyBase(bridge.ControlSurface):
        @property
        def song(self):
            return song

    for base in (MethodBase, PropertyBase):
        surface = type("Bridge", (bridge.CodingAssistantsBridge, base), {})
        instance = surface.__new__(surface)
        # Do not run __init__ (it binds the TCP port); only the accessor.
        expect(
            bridge.CodingAssistantsBridge._current_song(instance) is song,
            "song accessor via %s" % base.__name__,
        )


def main() -> None:
    py_compile.compile(str(HERE / "__init__.py"), doraise=True)
    print("plugins/ableton/__init__.py compiles")

    bridge = load_bridge()
    expect(not bridge._IN_LIVE, "dummy ControlSurface", "Live was importable")
    check_ops(bridge)
    check_control_surface(bridge)
    print("SMOKE OK")


if __name__ == "__main__":
    main()
