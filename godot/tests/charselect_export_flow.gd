extends "res://tests/world_menu_flow.gd"

# Run only with native_input_fixture charselect-export. Waits for the campsite and the
# roster's first character (Input Fixture, human male paladin, id 17, starter gear given
# by item only), lets the parent call the public `export-scene`, then checks the written
# snapshot against the live scene: the original CharSelectScene
# (src/scenes/char_select/scene_tree.rs) with Background (its WMOs and skybox),
# Character (one slot per equipment entry), Camera and EnvironmentSun.
const STABLE_MS := 3000
const SCENE_WAIT_MS := 90000
# starter_equipment() in native_input_fixture.rs, in entry order.
const SLOTS := ["MainHand", "Shirt", "Legs", "Feet", "OffHand"]

var scene: Node3D

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if not server.begins_with("127.0.0.1:") or server.ends_with(":0"):
		fail("Char-select export fixture requires its owned loopback endpoint")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_screen(client, "CharacterSelect", 15000):
		return
	if not await wait_campsite(client):
		return
	print("FIXTURE CHARSELECT_EXPORT_READY")
	var path := ProjectSettings.globalize_path("res://").path_join("../data/diagnostics/charselect-export-%d/scene-export.json" % OS.get_process_id()).simplify_path()
	var deadline := Time.get_ticks_msec() + 30000
	while not (FileAccess.file_exists(path) and not FileAccess.get_file_as_string(path).is_empty()):
		if Time.get_ticks_msec() > deadline:
			fail("No exported scene at " + path)
			return
		await process_frame
	if not check_export(FileAccess.get_file_as_string(path)):
		return
	print("FIXTURE CHARSELECT_EXPORT_DONE")
	client.free()
	quit(0)

# The campsite once the character, sky and its objects stop changing for STABLE_MS.
func wait_campsite(client: Node) -> bool:
	var deadline := Time.get_ticks_msec() + SCENE_WAIT_MS
	var last := ""
	var since := Time.get_ticks_msec()
	while Time.get_ticks_msec() < deadline:
		await process_frame
		scene = client.find_child("CharacterSelectScene", true, false) as Node3D
		if scene == null or scene.get_node_or_null("SelectedCharacter") == null or sky_node() == null or client.find_child("Sun", true, false) == null:
			continue
		var objects := scene.get_node_or_null("CampsiteObjects")
		var signature := "%d/%d" % [doodads().size(), wmos().size()] if objects != null else ""
		if signature != last:
			last = signature
			since = Time.get_ticks_msec()
		elif not signature.is_empty() and Time.get_ticks_msec() - since >= STABLE_MS:
			# Hold the character's pose so attached items stay where `export-scene` reads them.
			for animation in scene.get_node("SelectedCharacter").find_children("M2Animation", "", true, false):
				animation.process_mode = Node.PROCESS_MODE_DISABLED
			await process_frame
			print("FIXTURE CHARSELECT_EXPORT_OBJECTS %s" % signature)
			return true
	fail("Campsite did not settle: scene=%s objects=%s" % [scene, last])
	return false

func sky_node() -> Node3D:
	for child in scene.get_children():
		if child is Node3D and child.has_meta("m2_source_path") and child.name.begins_with("AuthoredSky"):
			return child
	return null

func doodads() -> Array:
	return scene.get_node("CampsiteObjects").get_children().filter(func(node): return node.name.begins_with("Doodad"))

func wmos() -> Array:
	return scene.get_node("CampsiteObjects").get_children().filter(func(node): return node.has_meta("wmo_model"))

func check_export(text: String) -> bool:
	var value: Variant = JSON.parse_string(text)
	if not value is Dictionary or not value.has("root"):
		fail("Exported scene is not a snapshot: " + text)
		return false
	var exported: Dictionary = value.root
	var labels: Array = exported.children.map(func(child): return child.label)
	if exported.label != "CharSelectScene" or exported.props != "Scene" or exported.transform != null or labels != ["Background", "Character", "Camera", "EnvironmentSun"]:
		fail("Exported char-select scene %s %s %s" % [exported.label, exported.props, labels])
		return false
	return check_background(exported.children[0]) and check_character(exported.children[1]) and check_camera(exported.children[2]) and check_sun(exported.children[3])

func check_background(exported: Dictionary) -> bool:
	var props: Dictionary = exported.props.get("Background", {})
	var model := str(props.get("model", ""))
	var tile := RegEx.create_from_string("^terrain:[A-Za-z0-9]+_\\d+_\\d+$")
	if tile.search(model) == null or props.doodad_count != doodads().size() or exported.transform != null:
		fail("Exported background %s, live doodads %d" % [props, doodads().size()])
		return false
	var live_wmos := wmos()
	var objects: Array = exported.children.filter(func(child): return child.label == "Object")
	if objects.size() != live_wmos.size() or exported.children.size() != live_wmos.size() + 1:
		fail("Exported background children %s, live WMOs %d" % [exported.children.map(func(child): return child.label), live_wmos.size()])
		return false
	for i in live_wmos.size():
		var wmo: Node3D = live_wmos[i]
		var object: Dictionary = objects[i].props.Object
		if object.kind != "WMO" or object.model != wmo.get_meta("wmo_model") or not matches(objects[i], scene.global_transform.affine_inverse() * wmo.global_transform):
			fail("Exported WMO %s, live %s %s" % [objects[i], wmo.name, wmo.get_meta("wmo_model")])
			return false
	var sky := sky_node()
	var skybox: Dictionary = exported.children[-1]
	if skybox.label != "Skybox" or skybox.props.Object.kind != "Skybox" or skybox.props.Object.model != sky.get_meta("m2_source_path") or not matches(skybox, scene.global_transform.affine_inverse() * sky.global_transform):
		fail("Exported skybox %s, live %s" % [skybox, sky.get_meta("m2_source_path")])
		return false
	print("FIXTURE CHARSELECT_EXPORT_BACKGROUND %s doodads=%d wmos=%d" % [model, props.doodad_count, live_wmos.size()])
	return true

func check_character(exported: Dictionary) -> bool:
	var model := scene.get_node("SelectedCharacter") as Node3D
	var props: Dictionary = exported.props.get("Character", {})
	var source := str(model.get_meta("m2_source_path"))
	if props.get("model") != source.get_file() or props.race != "Human" or props.gender != "Male" or props.name != NAME or props.character_id != 17 or not matches(exported, model.transform):
		fail("Exported character %s, loader input %s" % [exported, source])
		return false
	if exported.children.map(func(child): return child.label) != SLOTS.map(func(slot): return "Slot:" + slot):
		fail("Exported character slots %s" % [exported.children.map(func(child): return child.label)])
		return false
	var attached := 0
	for i in SLOTS.size():
		var slot: Dictionary = exported.children[i]
		var equipment: Dictionary = slot.props.EquipmentSlot
		var item := model.find_child("Equipment" + SLOTS[i], true, false) as Node3D
		if equipment.slot != SLOTS[i] or equipment.model != null:
			fail("Exported %s" % slot)
			return false
		if item == null:
			if equipment.attachment != null or slot.transform != null:
				fail("Exported %s names an item the character lacks" % slot)
				return false
			continue
		attached += 1
		var anchor := str(item.get_parent().name)
		if equipment.attachment != str(item.name) or equipment.anchor != anchor or equipment.attachment_anchor != anchor or not matches(slot, model.global_transform.affine_inverse() * item.global_transform):
			fail("Exported %s, live parent %s" % [slot, anchor])
			return false
	print("FIXTURE CHARSELECT_EXPORT_CHARACTER %s attached=%d" % [props.model, attached])
	return attached > 0

func check_camera(exported: Dictionary) -> bool:
	var camera := scene.get_node("Camera") as Camera3D
	if absf(exported.props.Camera.fov - camera.fov) > 0.001 or not matches(exported, camera.transform):
		fail("Exported camera %s, live fov %.3f at %s" % [exported, camera.fov, camera.transform])
		return false
	return true

func check_sun(exported: Dictionary) -> bool:
	var sun := scene.find_child("Sun", true, false) as DirectionalLight3D
	var light: Dictionary = exported.props.Light
	if sun == null or light.kind != "DirectionalLight3D" or absf(light.intensity - sun.light_energy) > 0.0001 or not matches(exported, scene.global_transform.affine_inverse() * sun.global_transform):
		fail("Exported sun %s, live %s" % [exported, sun])
		return false
	return true

func matches(exported: Dictionary, expected: Transform3D) -> bool:
	var t: Variant = exported.transform
	if not t is Dictionary:
		return false
	var translation := Vector3(t.translation[0], t.translation[1], t.translation[2])
	var rotation := Quaternion(t.rotation[0], t.rotation[1], t.rotation[2], t.rotation[3])
	var scale := Vector3(t.scale[0], t.scale[1], t.scale[2])
	return translation.distance_to(expected.origin) < 0.01 and absf(rotation.dot(expected.basis.get_rotation_quaternion())) > 0.9999 and scale.distance_to(expected.basis.get_scale()) < 0.001
