@tool
extends EditorPlugin

## Coding-Assistants Godot bridge.
##
## Opens a localhost line-JSON TCP server that `crates/mcp-godot`
## (`coding-assistants-mcp-godot`) connects to. One request per line:
##
##   {"op": "<tool>", "args": {...}}\n  ->  {"ok": true, "result": ...}\n
##                                    or  {"ok": false, "error": "..."}\n
##
## Everything runs on the editor's main thread, polled from `_process`, so
## no thread marshalling is needed (unlike the Blender/Krita plugins).
##
## Install: copy this folder to `res://addons/coding_assistants_bridge/`
## in your project, then Project > Project Settings > Plugins > enable it.

const PORT := 9767
const MAX_DEPTH_DEFAULT := 6

var _server: TCPServer
var _clients: Array = []  # [{ peer: StreamPeerTCP, buf: PackedByteArray }]


func _enter_tree() -> void:
	_server = TCPServer.new()
	var err := _server.listen(PORT, "127.0.0.1")
	if err != OK:
		push_error("[coding-assistants] Godot bridge could not listen on 127.0.0.1:%d (err %d)" % [PORT, err])
		return
	set_process(true)
	print("[coding-assistants] Godot bridge listening on 127.0.0.1:%d" % PORT)


func _exit_tree() -> void:
	set_process(false)
	for c in _clients:
		c.peer.disconnect_from_host()
	_clients.clear()
	if _server:
		_server.stop()
		_server = null


func _process(_delta: float) -> void:
	if _server == null:
		return
	while _server.is_connection_available():
		var peer := _server.take_connection()
		peer.set_no_delay(true)
		_clients.append({"peer": peer, "buf": PackedByteArray()})

	var still_open: Array = []
	for c in _clients:
		var peer: StreamPeerTCP = c.peer
		peer.poll()
		if peer.get_status() != StreamPeerTCP.STATUS_CONNECTED:
			continue
		var avail := peer.get_available_bytes()
		if avail > 0:
			var chunk := peer.get_data(avail)
			if chunk[0] == OK:
				c.buf.append_array(chunk[1])
		var nl := c.buf.find(10)  # '\n'
		if nl == -1:
			still_open.append(c)
			continue
		var line := c.buf.slice(0, nl).get_string_from_utf8()
		var reply := _handle_line(line)
		var out := (JSON.stringify(reply) + "\n").to_utf8_buffer()
		peer.put_data(out)
		peer.disconnect_from_host()
	_clients = still_open


func _handle_line(line: String) -> Dictionary:
	var parsed = JSON.parse_string(line)
	if typeof(parsed) != TYPE_DICTIONARY:
		return {"ok": false, "error": "invalid request JSON"}
	var op: String = parsed.get("op", "")
	var args: Dictionary = parsed.get("args", {})
	var result
	match op:
		"get_scene_summary": result = _op_get_scene_summary(args)
		"list_nodes": result = _op_list_nodes(args)
		"add_node": result = _op_add_node(args)
		"delete_node": result = _op_delete_node(args)
		"set_node_property": result = _op_set_node_property(args)
		"save_scene": result = _op_save_scene(args)
		"open_scene": result = _op_open_scene(args)
		"list_project_scenes": result = _op_list_project_scenes(args)
		"run_gdscript": result = _op_run_gdscript(args)
		_: return {"ok": false, "error": "unknown op %s" % op}
	if result is Dictionary and result.has("__error"):
		return {"ok": false, "error": result["__error"]}
	return {"ok": true, "result": result}


# --------------------------------------------------------------- ops


func _edited_root() -> Node:
	return get_editor_interface().get_edited_scene_root()


func _err(msg: String) -> Dictionary:
	return {"__error": msg}


func _op_get_scene_summary(_args: Dictionary):
	var root := _edited_root()
	if root == null:
		return _err("no scene is being edited")
	return {
		"path": root.scene_file_path,
		"root_name": root.name,
		"root_type": root.get_class(),
		"node_count": _count_nodes(root),
	}


func _count_nodes(node: Node) -> int:
	var n := 1
	for child in node.get_children():
		n += _count_nodes(child)
	return n


func _op_list_nodes(args: Dictionary):
	var root := _edited_root()
	if root == null:
		return _err("no scene is being edited")
	var max_depth: int = int(args.get("max_depth", MAX_DEPTH_DEFAULT))
	var out: Array = []
	_collect_nodes(root, root, 0, max_depth, out)
	return out


func _collect_nodes(node: Node, root: Node, depth: int, max_depth: int, out: Array) -> void:
	var rel := "" if node == root else String(root.get_path_to(node))
	out.append({"name": node.name, "type": node.get_class(), "path": rel})
	if depth >= max_depth:
		return
	for child in node.get_children():
		_collect_nodes(child, root, depth + 1, max_depth, out)


func _resolve(root: Node, path: String) -> Node:
	if path == "" or path == ".":
		return root
	return root.get_node_or_null(NodePath(path))


func _op_add_node(args: Dictionary):
	var root := _edited_root()
	if root == null:
		return _err("no scene is being edited")
	var cls: String = args.get("class_name", "")
	if not ClassDB.can_instantiate(cls):
		return _err("cannot instantiate class %s" % cls)
	var parent := _resolve(root, args.get("parent", ""))
	if parent == null:
		return _err("parent node %s not found" % args.get("parent", ""))
	var node = ClassDB.instantiate(cls)
	if not (node is Node):
		return _err("%s is not a Node" % cls)
	if args.has("name"):
		node.name = String(args["name"])
	parent.add_child(node)
	node.owner = root
	return {"path": String(root.get_path_to(node))}


func _op_delete_node(args: Dictionary):
	var root := _edited_root()
	if root == null:
		return _err("no scene is being edited")
	var node := _resolve(root, args.get("path", ""))
	if node == null or node == root:
		return _err("node %s not found (or is the scene root)" % args.get("path", ""))
	node.get_parent().remove_child(node)
	node.queue_free()
	return {"deleted": args["path"]}


func _op_set_node_property(args: Dictionary):
	var root := _edited_root()
	if root == null:
		return _err("no scene is being edited")
	var node := _resolve(root, args.get("path", ""))
	if node == null:
		return _err("node %s not found" % args.get("path", ""))
	var prop: String = args.get("property", "")
	var value = _coerce(node.get(prop), args.get("value"))
	node.set(prop, value)
	return {"path": args["path"], "property": prop, "value": str(node.get(prop))}


func _coerce(current, value):
	# Map a JSON array onto the vector/color type the property already holds.
	if value is Array:
		match typeof(current):
			TYPE_VECTOR2: return Vector2(value[0], value[1])
			TYPE_VECTOR3: return Vector3(value[0], value[1], value[2])
			TYPE_COLOR:
				if value.size() >= 4:
					return Color(value[0], value[1], value[2], value[3])
				return Color(value[0], value[1], value[2])
	return value


func _op_save_scene(_args: Dictionary):
	var root := _edited_root()
	if root == null:
		return _err("no scene is being edited")
	get_editor_interface().save_scene()
	return {"saved": root.scene_file_path}


func _op_open_scene(args: Dictionary):
	var path: String = args.get("path", "")
	if not FileAccess.file_exists(path):
		return _err("scene file %s does not exist" % path)
	get_editor_interface().open_scene_from_path(path)
	return {"opened": path}


func _op_list_project_scenes(_args: Dictionary):
	var found: Array = []
	_scan_dir("res://", found)
	found.sort()
	return found


func _scan_dir(path: String, found: Array) -> void:
	var dir := DirAccess.open(path)
	if dir == null:
		return
	dir.list_dir_begin()
	var name := dir.get_next()
	while name != "":
		if name.begins_with("."):
			name = dir.get_next()
			continue
		var full := path.path_join(name)
		if dir.current_is_dir():
			_scan_dir(full, found)
		elif name.ends_with(".tscn") or name.ends_with(".scn"):
			found.append(full)
		name = dir.get_next()
	dir.list_dir_end()


func _op_run_gdscript(args: Dictionary):
	var code: String = args.get("code", "")
	var src := "@tool\nextends RefCounted\nfunc _run():\n\tvar result = null\n"
	for l in code.split("\n"):
		src += "\t" + l + "\n"
	src += "\treturn result\n"
	var script := GDScript.new()
	script.source_code = src
	var err := script.reload()
	if err != OK:
		return _err("GDScript failed to compile (err %d)" % err)
	var inst = script.new()
	var value = inst.call("_run")
	return {"result": str(value)}
