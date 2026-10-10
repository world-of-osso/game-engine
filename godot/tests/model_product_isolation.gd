extends SceneTree

# Native offline proof through WorldModels, not direct unqualified M2 loading.
# GAME_ENGINE_ASSET_MODE=extracted-only; MODEL_ISOLATION_SHOTS=absolute PNG directory.
const DISPLAYS := {139403: [1100087, "wow_classic_beta"], 139409: [1100258, "wow_classic_beta"], 21774: [126278, "wow"]}
var loader: WowAssetLoader
var stage: Node3D
var camera: Camera3D
var shots := ""
var failures: Array[String] = []

func _initialize() -> void:
	call_deferred("run")

func fail(message: String) -> void:
	failures.append(message)
	push_error("MODEL_ISOLATION " + message)

func run() -> void:
	await run_displays(DISPLAYS.keys(), true)

func run_displays(displays: Array, include_player := false) -> void:
	shots = OS.get_environment("MODEL_ISOLATION_SHOTS")
	if DisplayServer.get_name() == "headless" or not shots.is_absolute_path():
		fail("Require native renderer and absolute PNG output directory")
		quit(1)
		return
	root.size = Vector2i(1280, 720)
	loader = WowAssetLoader.new()
	if loader.asset_runtime_status().mode != "ExtractedOnly":
		fail("Require extracted-only runtime")
		quit(1)
		return
	stage = Node3D.new()
	root.add_child(stage)
	var environment := WorldEnvironment.new()
	environment.environment = Environment.new()
	environment.environment.background_mode = Environment.BG_COLOR
	environment.environment.background_color = Color(0.055, 0.065, 0.075)
	environment.environment.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	environment.environment.ambient_light_color = Color.WHITE
	environment.environment.ambient_light_energy = 0.65
	stage.add_child(environment)
	var light := DirectionalLight3D.new()
	light.rotation_degrees = Vector3(-35, -30, 0)
	light.light_energy = 1.2
	stage.add_child(light)
	camera = Camera3D.new()
	camera.fov = 32.0
	camera.near = 0.05
	stage.add_child(camera)
	camera.make_current()
	var index = JSON.parse_string(FileAccess.get_file_as_string("res://../data/cache/model-asset-index.json"))
	if not index is Dictionary or DirAccess.make_dir_recursive_absolute(shots) != OK:
		fail("Cannot read asset receipts or create capture directory")
		quit(1)
		return
	var passed := 0
	for display in displays:
		if await capture_display(display, index):
			passed += 1
	if include_player and await capture_equipped_player(index):
		passed += 1
	var total := displays.size() + (1 if include_player else 0)
	var status := loader.asset_runtime_status()
	if status.forbidden_casc_accesses != 0:
		fail("CASC tripwire count " + str(status.forbidden_casc_accesses))
	print("MODEL_ISOLATION SUMMARY passed=", passed, " total=", total, " failures=", failures, " mode=", status.mode, " tripwire=", status.forbidden_casc_accesses)
	if failures.is_empty():
		print("MODEL_ISOLATION PASS displays=", displays, " mode=", status.mode, " tripwire=", status.forbidden_casc_accesses)
	stage.free()
	quit(0 if failures.is_empty() else 1)

func capture_display(display: int, index: Dictionary) -> bool:
	var result := loader.load_creature_display(display)
	if result.has("error"):
		fail("Display %d: %s" % [display, result.error])
		return false
	var visual: Node3D = result.node
	stage.add_child(visual)
	var model := visual.get_node("NpcModel") as Node3D
	var source := str(model.get_meta("m2_source_path")).simplify_path()
	var expected: Dictionary = {}
	for receipt in index.assets:
		if receipt.product == DISPLAYS[display][1] and receipt.fdid == DISPLAYS[display][0] and receipt.kind == "m2":
			expected = receipt
	if expected.is_empty() or not source.ends_with("/" + expected.path) or FileAccess.get_sha256(source) != expected.sha256:
		fail("Display %d: model source/hash differs from actual-build receipt: %s" % [display, source])
		visual.free()
		return false
	var meshes := 0
	for child in model.find_children("*", "MeshInstance3D", true, false):
		var mesh := child as MeshInstance3D
		if mesh.is_visible_in_tree():
			meshes += 1
	if meshes == 0:
		fail("Display %d has no visible meshes" % display)
		visual.free()
		return false
	var bounds: AABB = model.get_meta("m2_bounds")
	var focus := visual.global_transform * bounds.get_center()
	var radius := maxf(bounds.size.length() * visual.scale.x * 0.5, 0.5)
	camera.position = focus + Vector3(0, radius * 0.1, radius * 4.0)
	camera.look_at(focus)
	for frame in range(45):
		await process_frame
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var path := shots.path_join("display-%d.png" % display)
	if image.save_png(path) != OK:
		fail("Cannot save " + path)
		visual.free()
		return false
	print("MODEL_ISOLATION DISPLAY id=", display, " model=", DISPLAYS[display][0], " product=", expected.product, " actual_build=", expected.build, " source=", source, " meshes=", meshes)
	visual.free()
	return true

func capture_equipped_player(index: Dictionary) -> bool:
	var items := [{"slot": "MainHand", "item_id": 25, "inventory_type": 21}]
	var result := loader.load_player(1, 0, 1, items)
	if result.has("error"):
		fail("Player HumanMale item 25: " + str(result.error))
		return false
	var model: Node3D = result.node
	stage.add_child(model)
	var placement := loader.place_player_weapons(model, items, 1)
	var item := model.find_child("EquipmentMainHand", true, false) as Node3D
	if placement != "" or item == null or not item.is_visible_in_tree():
		fail("Player equipped item missing or placement failed: " + placement)
		model.free()
		return false
	for visual in [model, item]:
		var source := str(visual.get_meta("m2_source_path", "")).simplify_path()
		var authenticated := false
		for receipt in index.assets:
			if receipt.product == "wow" and receipt.kind == "m2" and source.ends_with("/" + receipt.path):
				authenticated = FileAccess.get_sha256(source) == receipt.sha256
		if not authenticated:
			fail("Player/item source differs from authenticated Retail receipt: " + source)
			model.free()
			return false
	var bounds: AABB = model.get_meta("m2_bounds")
	var center := model.global_transform * bounds.get_center()
	var height := maxf(bounds.size.y, maxf(bounds.size.x, bounds.size.z))
	camera.look_at_from_position(center + Vector3(height * 1.5, height * 0.15, height * 0.35), center)
	for frame in range(45):
		await process_frame
	await RenderingServer.frame_post_draw
	var shown := root.get_texture().get_image()
	item.visible = false
	await RenderingServer.frame_post_draw
	await RenderingServer.frame_post_draw
	var hidden := root.get_texture().get_image()
	item.visible = true
	var pixels = load("res://tests/world_player_equipment_pixels.gd").new()
	var changed: int = pixels.count_changed_pixels(shown, hidden)
	if changed < 5 or shown.save_png(shots.path_join("player-equipped-item-25.png")) != OK:
		fail("Equipped item has no captured pixels: " + str(changed))
		model.free()
		return false
	print("MODEL_ISOLATION PLAYER race=1 sex=0 item=25 item_pixels=", changed, " source=", model.get_meta("m2_source_path"), " item_source=", item.get_meta("m2_source_path"))
	model.free()
	return true
