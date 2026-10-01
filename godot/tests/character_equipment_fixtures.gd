extends SceneTree
## Named equipped characters built by the world's player loader (WowAssetLoader.load_player),
## posed on an authored clip, captured, and checked for body geosets, item attachments and
## animation. Run with a rendered display (scripts/agent/headless-client.sh start-godot).
## CHAREQUIP_FIXTURES=Name,Name limits the run; captures go to CHAREQUIP_CAPTURE_DIR.

const DEFAULT_CAPTURE_DIR := "res://../data/diagnostics/charequip/"
const EPSILON := 0.001

# Item slots/IDs/InventoryType from Item.db2 (build 12.1.0.69933). Attachment IDs: 0 shield,
# 1 right palm, 2 left palm, 5/6 right/left shoulder, 11 helm, 26/27 main/off back sheath,
# 28 shield back sheath, 30/31 main/off large back, 32/33 main/off hip (solarityclient
# `GetSheatheLink`).
const FIXTURES := [
	{
		"name": "AldricDrawn", "race": 1, "sex": 0, "class": 1, "sheath": 1, "anim": 26, "time_ms": 300.0,
		"items": [
			{"slot": "Shirt", "item_id": 38, "inventory_type": 4},
			{"slot": "Legs", "item_id": 39, "inventory_type": 7},
			{"slot": "Feet", "item_id": 40, "inventory_type": 8},
			{"slot": "MainHand", "item_id": 25, "inventory_type": 21},
			{"slot": "OffHand", "item_id": 2362, "inventory_type": 14},
		],
		"attachments": {"EquipmentMainHand": 1, "EquipmentOffHand": 0},
	},
	{
		"name": "AldricSheathed", "race": 1, "sex": 0, "class": 1, "sheath": 0, "anim": 0, "time_ms": 0.0,
		"items": [
			{"slot": "Shirt", "item_id": 38, "inventory_type": 4},
			{"slot": "Legs", "item_id": 39, "inventory_type": 7},
			{"slot": "Feet", "item_id": 40, "inventory_type": 8},
			{"slot": "MainHand", "item_id": 25, "inventory_type": 21},
			{"slot": "OffHand", "item_id": 2362, "inventory_type": 14},
		],
		"attachments": {"EquipmentMainHand": 32, "EquipmentOffHand": 28},
	},
	{
		"name": "ElowenRobe", "race": 1, "sex": 1, "class": 8, "sheath": 0, "anim": 5, "time_ms": 250.0,
		"items": [
			{"slot": "Chest", "item_id": 56, "inventory_type": 20},
			{"slot": "Legs", "item_id": 1395, "inventory_type": 7},
			{"slot": "Shirt", "item_id": 6096, "inventory_type": 4},
			{"slot": "Feet", "item_id": 55, "inventory_type": 8},
			{"slot": "MainHand", "item_id": 35, "inventory_type": 17},
		],
		# Apprentice's Robe GeosetGroup 1/0/1: sleeves 802 and robe 1302 over the pants.
		"visible_geosets": [802, 1302],
		"hidden_geosets": [1101, 1102, 501, 502],
		"attachments": {"EquipmentMainHand": 30},
	},
	{
		"name": "VaelisHood", "race": 29, "sex": 1, "class": 5, "sheath": 0, "anim": 0, "time_ms": 0.0,
		# Cloaked Hood: model resource 17420 has no Void Elf file; ChrRaces borrows Blood Elf.
		"items": [{"slot": "Head", "item_id": 1280, "inventory_type": 1}],
		"attachments": {"EquipmentHead": 11},
	},
	{
		"name": "ZuliHood", "race": 31, "sex": 1, "class": 5, "sheath": 0, "anim": 0, "time_ms": 0.0,
		"items": [{"slot": "Head", "item_id": 1280, "inventory_type": 1}],
		"attachments": {"EquipmentHead": 11},
	},
	{
		"name": "ShaeDualSheathed", "race": 4, "sex": 0, "class": 4, "sheath": 0, "anim": 0, "time_ms": 0.0,
		# Two SheatheType 1 swords cross on the back: main hand 26, off hand 27.
		"items": [
			{"slot": "MainHand", "item_id": 7961, "inventory_type": 13},
			{"slot": "OffHand", "item_id": 9424, "inventory_type": 13},
		],
		"attachments": {"EquipmentMainHand": 26, "EquipmentOffHand": 27},
	},
	{
		"name": "ShaeDualDrawn", "race": 4, "sex": 0, "class": 4, "sheath": 1, "anim": 26, "time_ms": 0.0,
		"items": [
			{"slot": "MainHand", "item_id": 7961, "inventory_type": 13},
			{"slot": "OffHand", "item_id": 9424, "inventory_type": 13},
		],
		"attachments": {"EquipmentMainHand": 1, "EquipmentOffHand": 2},
	},
	{
		"name": "HarnTwoHand", "race": 6, "sex": 0, "class": 1, "sheath": 0, "anim": 0, "time_ms": 0.0,
		"items": [{"slot": "MainHand", "item_id": 870, "inventory_type": 17}],
		"attachments": {"EquipmentMainHand": 26},
	},
	{
		"name": "LiraBow", "race": 4, "sex": 1, "class": 3, "sheath": 1, "anim": 0, "time_ms": 0.0,
		"items": [{"slot": "MainHand", "item_id": 2504, "inventory_type": 15}],
		"attachments": {"EquipmentMainHand": 2},
	},
	{
		# Horde (26) and neutral (24) Pandaren share ChrModel 47/48 with Alliance (25).
		"name": "MeiHuojin", "race": 26, "sex": 1, "class": 10, "sheath": 0, "anim": 0, "time_ms": 0.0,
		"items": [], "attachments": {},
	},
	{
		"name": "TaoNeutral", "race": 24, "sex": 0, "class": 10, "sheath": 0, "anim": 4, "time_ms": 200.0,
		"items": [], "attachments": {},
	},
	{
		"name": "GromShoulders", "race": 2, "sex": 0, "class": 1, "sheath": 0, "anim": 0, "time_ms": 0.0,
		"items": [
			{"slot": "Shoulder", "item_id": 1445, "inventory_type": 3},
			{"slot": "Back", "item_id": 1190, "inventory_type": 16},
		],
		"attachments": {"EquipmentShoulderLeft": 6, "EquipmentShoulderRight": 5},
	},
]

var capture_dir := DEFAULT_CAPTURE_DIR
var failures: Array[String] = []

func _initialize() -> void:
	if not ClassDB.class_exists("WowAssetLoader"):
		push_error("WowAssetLoader not registered")
		quit(1)
		return
	run.call_deferred()

func run() -> void:
	if DisplayServer.get_name() == "headless":
		print("FIXTURE FAIL character fixtures require a rendered display")
		quit(1)
		return
	var configured := OS.get_environment("CHAREQUIP_CAPTURE_DIR")
	if configured != "":
		capture_dir = configured.path_join("")
	DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path(capture_dir))
	root.size = Vector2i(800, 800)
	var light := DirectionalLight3D.new()
	light.rotation = Vector3(-0.6, 0.4, 0.0)
	root.add_child(light)
	var camera := Camera3D.new()
	root.add_child(camera)
	camera.make_current()
	var only := OS.get_environment("CHAREQUIP_FIXTURES").split(",", false)
	var loader = ClassDB.instantiate("WowAssetLoader")
	for fixture in FIXTURES:
		if only.is_empty() or only.has(fixture.name):
			await check_fixture(loader, camera, fixture)
	if failures.is_empty():
		print("FIXTURE PASS all character fixtures")
		quit(0)
	else:
		for failure in failures:
			print("FIXTURE FAIL ", failure)
		quit(1)

func fail(fixture: Dictionary, message: String) -> void:
	failures.append("%s: %s" % [fixture.name, message])

func check_fixture(loader: Object, camera: Camera3D, fixture: Dictionary) -> void:
	var result: Dictionary = loader.load_player(fixture.race, fixture.sex, fixture["class"], fixture.items)
	if result.has("error"):
		fail(fixture, "load: " + str(result.error))
		return
	var model: Node3D = result.node
	root.add_child(model)
	var placed: String = loader.place_player_weapons(model, fixture.items, fixture.sheath)
	if placed != "":
		fail(fixture, "weapon placement: " + placed)
	pose(fixture, model)
	for side in [1.0, -1.0]:
		frame(camera, model, side)
		await RenderingServer.frame_post_draw
		await RenderingServer.frame_post_draw
		var image := root.get_texture().get_image()
		var path: String = capture_dir + fixture.name + ("" if side > 0.0 else "-back") + ".png"
		if image.save_png(path) != OK:
			fail(fixture, "could not save " + path)
	check_geosets(fixture, model)
	check_attachments(fixture, model)
	print("FIXTURE %s geosets=%s items=%s" % [fixture.name, visible_geosets(model), item_parents(model)])
	model.free()

func pose(fixture: Dictionary, model: Node3D) -> void:
	var animation := model.get_node_or_null("M2Animation") as WowAnimationPlayer
	if animation == null:
		fail(fixture, "no M2Animation")
		return
	animation.set_process(false)
	if not animation.play_animation(fixture.anim, true):
		fail(fixture, "model has no animation %d" % fixture.anim)
		return
	# Past the crossfade from the default Stand, so the pose is the clip's own.
	animation.advance_time_ms(1000.0 + fixture.time_ms)
	animation.set_paused(true)
	if animation.current_animation_id() != fixture.anim:
		fail(fixture, "plays %d, not %d" % [animation.current_animation_id(), fixture.anim])

func frame(camera: Camera3D, model: Node3D, side: float) -> void:
	# Framed on the posed body meshes; the model faces +X (WoW +X), so the camera looks at
	# its front from there (side 1) or its back (side -1).
	var bounds := AABB()
	var first := true
	for child in model.get_children():
		var mesh := child as MeshInstance3D
		if mesh != null and mesh.visible:
			bounds = mesh.get_aabb() if first else bounds.merge(mesh.get_aabb())
			first = false
	var center := model.global_transform * bounds.get_center()
	var height := maxf(bounds.size.y, maxf(bounds.size.x, bounds.size.z))
	camera.look_at_from_position(center + Vector3(height * 1.3, height * 0.15, height * 0.35) * side, center)

func visible_geosets(model: Node3D) -> Array[int]:
	var parts: Array[int] = []
	for child in model.get_children():
		var mesh := child as MeshInstance3D
		if mesh != null and mesh.visible and mesh.has_meta("m2_mesh_part"):
			var part: int = mesh.get_meta("m2_mesh_part")
			if not parts.has(part):
				parts.append(part)
	parts.sort()
	return parts

func check_geosets(fixture: Dictionary, model: Node3D) -> void:
	var parts := visible_geosets(model)
	for part in fixture.get("visible_geosets", []):
		if not parts.has(part):
			fail(fixture, "geoset %d hidden; visible %s" % [part, parts])
	for part in fixture.get("hidden_geosets", []):
		if parts.has(part):
			fail(fixture, "geoset %d visible" % part)

func item_parents(model: Node3D) -> Dictionary:
	var parents := {}
	for node in model.find_children("Equipment*", "Node3D", true, false):
		parents[String(node.name)] = String(node.get_parent().name)
	return parents

func check_attachments(fixture: Dictionary, model: Node3D) -> void:
	for item_name in fixture.attachments:
		var item := model.find_child(item_name, true, false) as Node3D
		var expected := "Attachment%d" % fixture.attachments[item_name]
		if item == null:
			fail(fixture, "%s missing" % item_name)
			continue
		var parent := item.get_parent() as Node3D
		if parent.name != expected:
			fail(fixture, "%s on %s, not %s" % [item_name, parent.name, expected])
			continue
		if not item.is_visible_in_tree():
			fail(fixture, "%s hidden" % item_name)
		if not has_visible_mesh(item):
			fail(fixture, "%s has no visible mesh" % item_name)
		# The item sits on its attachment point as it follows the posed bone.
		var expected_origin := parent.global_transform * item.transform.origin
		if item.global_position.distance_to(expected_origin) > EPSILON:
			fail(fixture, "%s at %s, attachment puts it at %s" % [item_name, item.global_position, expected_origin])

func has_visible_mesh(node: Node) -> bool:
	for child in node.find_children("*", "MeshInstance3D", true, false):
		var mesh := child as MeshInstance3D
		if mesh.is_visible_in_tree() and mesh.mesh != null:
			return true
	return false
