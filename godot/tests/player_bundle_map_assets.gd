extends SceneTree

const CAPTURE_SIZE := Vector2i(1920, 1080)
const TILE_WAIT_MS := 30000
const MAP_CASES := [
	{"panel": "DazarAlorWorldMap", "tile_prefix": "WorldMapTile", "capture": "dazar-alor.png"},
	{"panel": "HousingFinderMap", "tile_prefix": "HouseFinderMapTile", "capture": "founders-point.png"},
]

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = CAPTURE_SIZE
	root.content_scale_size = Vector2i.ZERO
	var output := OS.get_environment("PLAYER_BUNDLE_MAP_CAPTURES")
	if output.is_empty():
		fail("PLAYER_BUNDLE_MAP_CAPTURES is required")
		return
	if DirAccess.make_dir_recursive_absolute(output) != OK:
		fail("Cannot create capture directory: " + output)
		return
	for map_case in MAP_CASES:
		if not await capture_map(map_case, output):
			return
	print("PASS: Forever player-bundle map tiles loaded for Dazar'alor and Founder's Point")
	quit(0)

func capture_map(map_case: Dictionary, output: String) -> bool:
	var owner := ClassDB.instantiate("RegistryUi") as Node
	root.add_child(owner)
	var error: String = owner.call("show_template_blend_preview", true, map_case.panel)
	if not error.is_empty():
		owner.free()
		return fail_map(map_case.panel, error)
	var result := await wait_for_map_tiles(owner, map_case.tile_prefix)
	if not result.error.is_empty():
		owner.free()
		return fail_map(map_case.panel, result.error)
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var path := output.path_join(map_case.capture)
	if image == null or image.get_size() != CAPTURE_SIZE or image.save_png(path) != OK:
		owner.free()
		return fail_map(map_case.panel, "Capture failed or had the wrong dimensions")
	print("PLAYER_BUNDLE_MAP ", JSON.stringify({
		"skin": "Forever",
		"panel": map_case.panel,
		"rendered_tiles": result.tile_count,
		"missing_tiles": 0,
		"capture": path,
	}))
	owner.free()
	await process_frame
	return true

func wait_for_map_tiles(owner: Node, prefix: String) -> Dictionary:
	var deadline := Time.get_ticks_msec() + TILE_WAIT_MS
	var tiles: Array[Node] = []
	var missing: Array[String] = []
	while Time.get_ticks_msec() < deadline:
		tiles = owner.find_children(prefix + "*", "", true, false)
		missing = missing_map_tiles(tiles)
		if not tiles.is_empty() and missing.is_empty():
			return {"tile_count": tiles.size(), "error": ""}
		await process_frame
	return {
		"tile_count": tiles.size(),
		"error": "Tile load timed out: expected visible textures for " + str(missing),
	}

func missing_map_tiles(tiles: Array[Node]) -> Array[String]:
	var missing: Array[String] = []
	for tile in tiles:
		var texture := tile.get_node_or_null("Parts/Part0") as TextureRect
		if texture == null or texture.texture == null:
			missing.append(str(tile.name))
	return missing

func fail_map(panel: String, message: String) -> bool:
	return fail(panel + ": " + message)

func fail(message: String) -> bool:
	push_error(message)
	quit(1)
	return false
