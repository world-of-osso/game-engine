extends SceneTree

## One extracted-only certification pose (docs/specs/shipped-assets.md
## "P4 no-install certification run"). Writes facts as JSON; the caller joins
## them with emitted asset errors from this process's log.
##
## CERT_MAP       map directory (required)
## CERT_POINT     requested position "x,y,z" (required)
## CERT_MODE      "terrain" (default) or "global_wmo"
## CERT_SNAP      1 to move to the authored tile centre (Forever maps; default 0)
## CERT_OUTPUT    absolute JSON path (required)
## CERT_SHOT      absolute PNG capture path (required)
## CERT_TIMEOUT   queue budget in milliseconds (default 180000)

const STABLE_FRAMES := 30

var client: Node3D

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	var map := OS.get_environment("CERT_MAP")
	var point_text := OS.get_environment("CERT_POINT")
	var output := OS.get_environment("CERT_OUTPUT")
	var shot := OS.get_environment("CERT_SHOT")
	if map == "" or point_text == "" or output == "" or shot == "":
		_fail("CERT_MAP, CERT_POINT, CERT_OUTPUT and CERT_SHOT are required")
		return
	if not output.is_absolute_path() or not shot.is_absolute_path():
		_fail("CERT_OUTPUT and CERT_SHOT must be absolute")
		return
	var mode := OS.get_environment("CERT_MODE")
	if mode == "":
		mode = "terrain"
	var timeout := 180000
	if OS.get_environment("CERT_TIMEOUT") != "":
		timeout = int(OS.get_environment("CERT_TIMEOUT"))
	var loader := WowAssetLoader.new()
	if loader.asset_runtime_status().mode != "ExtractedOnly":
		_fail("certification requires extracted-only")
		return
	root.size = Vector2i(1280, 720)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	for frame in 120:
		await process_frame
	var point := _parse_vector(point_text)
	var error: String = client.preview_world_map(map, point)
	if error != "":
		_fail(error)
		return
	if OS.get_environment("CERT_SNAP") == "1" and mode == "terrain":
		point = await _snapped_pose(map, point)
	var skin_error: String = client.preview_world_skin(true)
	if skin_error != "":
		print("NOINSTALL_CERT_SKIN_ERROR ", skin_error)
	client.set_world_minutes(1440.0)
	var camera := Camera3D.new()
	camera.far = 4000.0
	root.add_child(camera)
	camera.current = true
	camera.position = point + Vector3(90.0, 140.0, 90.0)
	camera.look_at(point)
	var deadline := Time.get_ticks_msec() + timeout
	var stable := 0
	var drained := false
	var lit := false
	var frame_count := 0
	var trace := OS.get_environment("CERT_TRACE") == "1"
	var last_lighting_error := ""
	while Time.get_ticks_msec() < deadline:
		await process_frame
		frame_count += 1
		camera.current = true
		var state: Dictionary = client.account_state()
		if trace and frame_count % 60 == 0:
			print("CERT_TRACE frame=%d objects=%s terrain_pending=%d" % [frame_count, state.world_objects, int(state.terrain.pending_count)])
		if not lit:
			var lighting_error: String = client.preview_world_lighting(point, camera.position)
			if lighting_error == "":
				lit = true
			else:
				if lighting_error != "Preview lighting requires a loaded WDT" and lighting_error != last_lighting_error:
					last_lighting_error = lighting_error
					print("NOINSTALL_CERT_LIGHTING ", lighting_error)
				elif trace and frame_count % 60 == 0:
					print("CERT_TRACE lighting_error=", lighting_error)
			continue
		if mode == "global_wmo":
			var objects: Dictionary = state.world_objects
			if int(state.terrain.pending_count) == 0 and _int_value(state.terrain.get("global_wmo_fdid", 0)) > 0 and int(objects.pending) == 0:
				stable += 1
				if stable >= STABLE_FRAMES:
					drained = true
					break
			else:
				stable = 0
			continue
		var objects: Dictionary = state.world_objects
		if int(objects.pending) == 0 and int(objects.spawned) > 0:
			stable += 1
			if stable >= STABLE_FRAMES:
				drained = true
				break
		else:
			stable = 0
	var state: Dictionary = client.account_state()
	if mode == "terrain" and int(state.terrain.parsed_tiles.size()) > 0 and OS.get_environment("CERT_SNAP") != "0":
		var tiles: Array = state.terrain.parsed_tiles
		var tile: Dictionary = tiles[0]
		var centre := Vector3(
			(32.0 - float(tile.tile_x) - 0.5) * 533.3333,
			camera.position.y,
			(float(tile.tile_y) - 31.5) * 533.3333)
		if centre.distance_to(point) > 533.0:
			camera.position = centre + Vector3(90.0, 140.0, 90.0)
			camera.look_at(centre)
	_hide_ui(client)
	for frame in 30:
		camera.current = true
		await process_frame
		await RenderingServer.frame_post_draw
	root.get_texture().get_image().save_png(shot)
	var objects: Dictionary = state.world_objects
	var terrain: Dictionary = state.terrain
	var result := {
		"map": map,
		"mode": mode,
		"requested_point": [_parse_vector(point_text).x, _parse_vector(point_text).y, _parse_vector(point_text).z],
		"resolved_point": [point.x, point.y, point.z],
		"drained": drained,
		"spawned": int(objects.get("spawned", 0)),
		"object_failures": int(objects.get("failures", 0)),
		"pending": int(objects.get("pending", 0)),
		"tiles": terrain.parsed_tiles.size(),
		"terrain_failures": terrain.failures.size(),
		"wdt_path": String(terrain.get("wdt_path", "")),
		"global_wmo_fdid": _int_value(terrain.get("global_wmo_fdid", 0)),
		"lighting_error": last_lighting_error,
		"ui_skin": String(client.account_state().get("ui_skin", "")),
		"skin_error": skin_error,
		"visible_meshes": _visible_meshes(client),
		"runtime": loader.asset_runtime_status(),
		"screenshot": shot,
	}
	var file := FileAccess.open(output, FileAccess.WRITE)
	file.store_string(JSON.stringify(result, "\t"))
	file.close()
	print("NOINSTALL_CERT ", JSON.stringify(result))
	client.queue_free()
	loader = null
	for frame in 8:
		await process_frame
	quit(0)

func _int_value(value: Variant) -> int:
	match typeof(value):
		TYPE_INT:
			return value
		TYPE_FLOAT:
			return int(value)
		TYPE_STRING, TYPE_STRING_NAME:
			var text := str(value)
			return int(text) if text.is_valid_int() else 0
	return 0

func _visible_meshes(node: Node) -> int:
	var count := 0
	for mesh in node.find_children("*", "MeshInstance3D", true, false):
		if mesh.is_visible_in_tree():
			count += 1
	return count

func _hide_ui(node: Node) -> void:
	for child in node.get_children():
		if child is CanvasLayer:
			child.visible = false
		elif child is Control:
			child.hide()
		else:
			_hide_ui(child)

func _parse_vector(text: String) -> Vector3:
	var parts := text.split(",")
	return Vector3(float(parts[0]), float(parts[1]), float(parts[2]))

## MAIN's authored tiles in file order, `(index % 64, index / 64)`.
func _main_tiles(bytes: PackedByteArray) -> Array[Vector2i]:
	var active: Array[Vector2i] = []
	var offset := 0
	while offset + 8 <= bytes.size():
		var tag := PackedByteArray([bytes[offset + 3], bytes[offset + 2], bytes[offset + 1], bytes[offset]])
		var size := bytes.decode_u32(offset + 4)
		if tag.get_string_from_ascii() == "MAIN" and size == 4096 * 8:
			for index in 4096:
				if bytes.decode_u32(offset + 8 + index * 8) & 1:
					active.append(Vector2i(index % 64, index / 64))
			break
		offset += 8 + size
	return active

func _snapped_pose(map: String, point: Vector3) -> Vector3:
	var state: Dictionary = client.account_state()
	var deadline := Time.get_ticks_msec() + 30000
	while String(state.terrain.wdt_path) == "" and Time.get_ticks_msec() < deadline:
		await process_frame
		state = client.account_state()
	var path := String(state.terrain.wdt_path)
	if path == "":
		return point
	var file := FileAccess.open(path, FileAccess.READ)
	if file == null:
		return point
	var active := _main_tiles(file.get_buffer(file.get_length()))
	if active.is_empty():
		return point
	var sum := Vector2i.ZERO
	for tile in active:
		sum += tile
	var centre := Vector2i(sum.x / active.size(), sum.y / active.size())
	var best := active[0]
	for tile in active:
		if (tile - centre).length_squared() < (best - centre).length_squared():
			best = tile
	var world := Vector3(
		(31.5 - float(best.y)) * 533.3333, point.y, (float(best.x) - 31.5) * 533.3333)
	var snap_error: String = client.preview_world_map(map, world)
	if snap_error != "":
		_fail(snap_error)
		return point
	for frame in 90:
		await process_frame
	return world

func _fail(message: String) -> void:
	push_error(message)
	quit(1)
