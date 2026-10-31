extends RefCounted

## Record the actual in-tree visibility and shadow mode of every Stormwind doodad
## and its mesh descendants. The comparison runs against two real client builds.
static func write(client: Node, path: String) -> int:
	var rows: Array[String] = []
	var stack: Array[Node] = [client]
	while not stack.is_empty():
		var node: Node = stack.pop_back()
		stack.append_array(node.get_children())
		if not node is Node3D:
			continue
		var name := str(node.name)
		if not (name.begins_with("Doodad") or name.begins_with("WmoDoodad")):
			continue
		var meshes: Array[String] = []
		var descendants: Array[Node] = node.get_children()
		while not descendants.is_empty():
			var child: Node = descendants.pop_back()
			descendants.append_array(child.get_children())
			if child is MeshInstance3D:
				var visible: bool = child.is_visible_in_tree()
				meshes.append(JSON.stringify({"path": str(node.get_path_to(child)),
					"visible": visible, "shadow": int(child.cast_shadow) if visible else null}))
		meshes.sort()
		rows.append(JSON.stringify({"path": str(node.get_path()),
			"visible": node.is_visible_in_tree(), "meshes": meshes}))
	rows.sort()
	var file := FileAccess.open(path, FileAccess.WRITE)
	if file == null:
		push_error("Cannot write scenery receipt: " + path)
		return -1
	for row in rows:
		file.store_line(row)
	file.flush()
	return rows.size()
