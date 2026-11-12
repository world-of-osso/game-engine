extends SceneTree

## World-object census capture for one map pose. Unlike `water_coast_compare`, this does not
## need a liquid surface or a terrain height at the point: it waits for the preview's object
## queue to drain and reports how many objects spawned and how many failed, which is what the
## Forever receipt census has to move.
##
## WATER_CENSUS_MAP      map directory (required)
## WATER_CENSUS_POINT    looked-at point as "x,y,z" (required)
## WATER_CENSUS_OUTPUT   screenshot path (required)
## WATER_CENSUS_TIMEOUT  wait budget in milliseconds (default 180000)

const STABLE_FRAMES := 30

var client: Node3D

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	var map := OS.get_environment("WATER_CENSUS_MAP")
	var point_text := OS.get_environment("WATER_CENSUS_POINT")
	var output := OS.get_environment("WATER_CENSUS_OUTPUT")
	if map == "" or point_text == "" or output == "":
		fail("WATER_CENSUS_MAP, WATER_CENSUS_POINT and WATER_CENSUS_OUTPUT are required")
		return
	var timeout := 180000
	if OS.get_environment("WATER_CENSUS_TIMEOUT") != "":
		timeout = int(OS.get_environment("WATER_CENSUS_TIMEOUT"))
	root.size = Vector2i(1280, 720)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	for frame in 120:
		await process_frame
	var point := parse_vector(point_text)
	var error: String = client.preview_world_map(map, point)
	if error != "":
		fail(error)
		return
	await report_wdt_order(map, point)
	# A pose can land in a WDT MAIN slot that has no ADT (open sea, a void edge). The live
	# streamer correctly shows nothing there, so the census picks its own pose: the centre of
	# the authored tiles' bounding box, snapped to an authored tile.
	if OS.get_environment("WATER_CENSUS_SNAP") != "0":
		point = await snapped_pose(map, point)
	client.set_world_minutes(1440.0)
	var camera := Camera3D.new()
	camera.far = 4000.0
	root.add_child(camera)
	camera.current = true
	var eye := Vector3(90.0, 140.0, 90.0)
	if OS.get_environment("WATER_CENSUS_EYE") != "":
		eye = parse_vector(OS.get_environment("WATER_CENSUS_EYE"))
	camera.position = point + eye
	camera.look_at(point)
	var deadline := Time.get_ticks_msec() + timeout
	var stable := 0
	var state: Dictionary = {}
	var drained := false
	var lit := false
	while Time.get_ticks_msec() < deadline:
		await process_frame
		camera.current = true
		state = client.account_state()
		# The preview adopts the map's light only when this pass runs, and the call fails until
		# the WDT is loaded. Without it `scene_light` stays unset, so the terrain and the M2
		# shaders read an ambient of zero and the frame is a black silhouette.
		if not lit:
			var lighting_error: String = client.preview_world_lighting(point, camera.position)
			if lighting_error == "":
				lit = true
			continue
		var objects: Dictionary = state.world_objects
		if int(objects.pending) == 0 and int(objects.spawned) > 0:
			stable += 1
			if stable >= STABLE_FRAMES:
				drained = true
				break
		else:
			stable = 0
	# Every streamed tile must have loaded its own ADT: the file stem's `_{col}_{row}` has to
	# equal the slot the streamer placed it in. A swapped lookup shows up here as a mismatch
	# even when the object counts look healthy.
	var ok := 0
	var bad: Array[String] = []
	for tile in state.terrain.get("parsed_tiles", []):
		var path := String(tile.get("root_path", ""))
		var stem := path.get_file().get_basename()
		# Named stems are `map_{row}_{col}`, the order the streamer carries.
		var suffix := "_%d_%d" % [int(tile.get("tile_y", -1)), int(tile.get("tile_x", -1))]
		if stem.ends_with(suffix) or not stem.contains("_"):
			ok += 1
		else:
			bad.append("%s in slot %s" % [stem, suffix])
	print("WORLD_CENSUS_PLACEMENT ok=%d bad=%d %s" % [ok, bad.size(), bad])
	# The root viewport keeps the frame it was last handed: reading it without hiding the UI
	# returns the interface frame, which is identical for every pose.
	hide_ui(client)
	for frame in 30:
		camera.current = true
		await process_frame
		await RenderingServer.frame_post_draw
	# A pose whose tile has no ADT streams the nearest authored tile instead, so aim at what
	# actually loaded: otherwise the capture is empty sky from far outside the terrain.
	var tiles: Array = state.terrain.parsed_tiles
	if tiles.size() > 0:
		var tile: Dictionary = tiles[0]
		var row := float(tile.tile_y)
		var col := float(tile.tile_x)
		var centre := Vector3((32.0 - col - 0.5) * 533.3333, camera.position.y, (row - 31.5) * 533.3333)
		if centre.distance_to(point) > 533.0:
			camera.position = centre + Vector3(90.0, 140.0, 90.0)
			camera.look_at(centre)
			print("WORLD_CENSUS_RETARGET pose_tile_empty=%s camera_tile=%s" % [point, centre])
			for frame in 20:
				camera.current = true
				await process_frame
				await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	image.save_png(output)
	var objects: Dictionary = state.world_objects
	print("WORLD_CENSUS_PROOF map=%s point=%s drained=%s spawned=%d failures=%d pending=%d tiles=%d terrain_failures=%d screenshot=%s" % [
		map, point, drained, int(objects.spawned), int(objects.failures), int(objects.pending),
		state.terrain.parsed_tiles.size(), state.terrain.failures.size(), output])
	if not drained:
		print("WORLD_CENSUS_WARNING object queue did not drain within %d ms" % timeout)
	if state.terrain.parsed_tiles.size() == 0:
		print("WORLD_CENSUS_TERRAIN attached=%s terrain=%s" % [state.world_attached, state.terrain])
	quit(0)

## MAIN's authored tiles in the order the file stores them, `(index % 64, index / 64)`.
func main_tiles(bytes: PackedByteArray) -> Array[Vector2i]:
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

## Reports which tuple order the map's MAIN set agrees with, for the tile the streamer derives
## from `point`. `bevy_to_tile_coords` returns `(row, col)`; MAIN stores `(col, row)`.
func report_wdt_order(map: String, point: Vector3) -> void:
	var state: Dictionary = client.account_state()
	var deadline := Time.get_ticks_msec() + 30000
	while String(state.terrain.wdt_path) == "" and Time.get_ticks_msec() < deadline:
		await process_frame
		state = client.account_state()
	var path := String(state.terrain.wdt_path)
	if path == "":
		print("WORLD_CENSUS_WDT map=%s wdt=<missing>" % map)
		return
	var file := FileAccess.open(path, FileAccess.READ)
	if file == null:
		print("WORLD_CENSUS_WDT map=%s wdt=<unreadable %s>" % [map, path])
		return
	var active := main_tiles(file.get_buffer(file.get_length()))
	var row := int(floor((32.0 * 533.3333 + point.z) / 533.3333))
	var col := int(floor((32.0 * 533.3333 - point.x) / 533.3333))
	var rc := Vector2i(row, col)
	var cr := Vector2i(col, row)
	print("WORLD_CENSUS_WDT map=%s main=%d pose=%s in_rc=%s in_cr=%s rotation=%s first=%s" % [
		map, active.size(), rc, active.has(rc), active.has(cr),
		int(state.terrain.get("wdt_flags", -1)), active.slice(0, 4)])

## Reads the map's WDT and returns the centre of the authored tiles' bounding box, snapped to an
## authored tile. Falls back to `point` when the WDT is unreadable or has no authored tile.
func snapped_pose(map: String, point: Vector3) -> Vector3:
	# The WDT is read on the terrain worker, so its path is empty until the map request lands.
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
	var bytes := file.get_buffer(file.get_length())
	var active := main_tiles(bytes)
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
	# Invert `bevy_to_tile_coords`: row = floor((32 + bz) / 533.33), col = floor((32 - bx) / 533.33),
	# so the pose that lands on tile `(row, col)` is the tile centre. MAIN stores
	# `(index % 64, index / 64)`, which is that same `(row, col)` — verified against the ADT's own
	# first KNCM position, `azeroth_32_48` sitting at x=-8533.33, z=233.
	var world := Vector3(
		(31.5 - float(best.y)) * 533.3333, point.y, (float(best.x) - 31.5) * 533.3333)
	print("WORLD_CENSUS_SNAP map=%s tiles=%d centre=%s pose=%s" % [map, active.size(), best, world])
	var snap_error: String = client.preview_world_map(map, world)
	if snap_error != "":
		push_error(snap_error)
		return point
	for frame in 90:
		await process_frame
	return world

func hide_ui(node: Node) -> void:
	for child in node.get_children():
		if child is CanvasLayer:
			child.visible = false
		elif child is Control:
			child.hide()
		else:
			hide_ui(child)

func parse_vector(text: String) -> Vector3:
	var parts := text.split(",")
	return Vector3(float(parts[0]), float(parts[1]), float(parts[2]))

func fail(message: String) -> void:
	push_error(message)
	quit(1)
