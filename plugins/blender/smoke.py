"""Local smoke test for the Blender bridge add-on.

    blender --background --python plugins/blender/smoke.py

Registers the add-on, then drives a handful of ops through a real localhost
socket (the same path `crates/mcp-blender` uses) and checks the replies.
Exits non-zero on failure. Not run in CI — no Blender on the runners.
"""

import json
import socket
import sys
import time

sys.path.insert(0, __file__.rsplit("/", 1)[0])
import coding_assistants_bridge as bridge  # noqa: E402

PORT = 9765


def call(op, **args):
    with socket.create_connection(("127.0.0.1", PORT), timeout=10) as s:
        s.sendall((json.dumps({"op": op, "args": args}) + "\n").encode())
        buf = b""
        while b"\n" not in buf:
            buf += s.recv(4096)
    return json.loads(buf.partition(b"\n")[0])


def expect_ok(reply, label):
    if not reply.get("ok"):
        print(f"SMOKE FAILED at {label}: {reply}")
        sys.exit(1)
    return reply["result"]


def main():
    bridge.register()
    time.sleep(0.5)  # let the listener bind

    summary = expect_ok(call("get_scene_summary"), "get_scene_summary")
    print("scene:", summary["scene"], "objects:", summary["object_count"])

    made = expect_ok(
        call("create_primitive", kind="cube", location=[1, 2, 3], name="SmokeCube"),
        "create_primitive",
    )
    assert made["name"] == "SmokeCube", made

    names = [o["name"] for o in expect_ok(call("list_objects"), "list_objects")]
    assert "SmokeCube" in names, names

    expect_ok(call("delete_object", name="SmokeCube"), "delete_object")
    names = [o["name"] for o in expect_ok(call("list_objects"), "list_objects")]
    assert "SmokeCube" not in names, names

    bad = call("delete_object", name="DoesNotExist")
    assert not bad["ok"] and "not found" in bad["error"], bad

    print("SMOKE OK")
    bridge.unregister()


if __name__ == "__main__":
    main()
