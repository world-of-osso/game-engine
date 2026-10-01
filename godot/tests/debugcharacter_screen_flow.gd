extends "res://tests/debug_screen_flow_base.gd"

# `--screen debugcharacter` (original `src/scenes/geoset_debug/mod.rs`): two human
# warriors 3.4 yd apart. Left: plate helm 1128, texture-only cloth glove 510, belt
# 109162 with its runtime buckle, hybrid legs 159629. Right: hood 685129, leather glove
# 154616 with its runtime model, geoset-only legs 73783. Needs no server.
# Run: native_debug_screen_fixture debugcharacter
const MIN_MODEL_PIXELS := 1500

var scene: Node

func check_live() -> bool:
	scene = await wait_node("DebugCharacter")
	if scene == null:
		return false
	var login := client.get_node_or_null("LoginUI") as CanvasLayer
	if login == null or login.visible:
		fail("Login UI is missing or still visible over the geoset debug scene")
		return false
	if client.account_state().reply_received:
		fail("Geoset debug scene contacted a server")
		return false
	var left := scene.get_node_or_null("DebugCharacterGeoset") as Node3D
	var right := scene.get_node_or_null("DebugCharacterM2") as Node3D
	if left == null or right == null:
		fail("Debug characters missing")
		return false
	if not left.position.is_equal_approx(Vector3(-1.7, 0, 0)) or not right.position.is_equal_approx(Vector3(1.7, 0, 0)) or absf(left.rotation.y + PI / 2.0) > 0.001:
		fail("Debug characters placed at %s %s yaw %.3f" % [left.position, right.position, left.rotation.y])
		return false
	# Runtime item models attach only where the display has one.
	for expected in [[left, "EquipmentHead", true], [right, "EquipmentHead", true], [left, "EquipmentHands", false], [right, "EquipmentHands", true], [left, "EquipmentWaist", true], [left, "EquipmentLegs", true], [right, "EquipmentLegs", false]]:
		var found: bool = expected[0].find_child(expected[1], true, false) != null
		if found != expected[2]:
			fail("%s %s attached=%s, expected %s" % [expected[0].name, expected[1], found, expected[2]])
			return false
	var camera := scene.get_node("Camera") as Camera3D
	await settle(300)
	if not camera.current or camera.global_position.distance_to(Vector3(0, 1.8, 6)) > 0.01:
		fail("Camera current=%s at %s, expected (0, 1.8, 6)" % [camera.current, camera.global_position])
		return false
	return true

func check_cli() -> bool:
	if not check_export():
		return false
	var semantic := tree_reply("scene")
	for expected in ["Camera \"Camera\" fov=45", "Light \"Light\" DirectionalLight3D", "Object \"Ground\" MeshInstance3D", "Model \"DebugCharacterGeoset\" @ (-1.7, 0.0, 0.0)", "Model \"DebugCharacterM2\" @ (1.7, 0.0, 0.0)"]:
		if not semantic.contains(expected):
			fail("CLI dump-scene lacks %s:\n%s" % [expected, semantic])
			return false
	var tree := tree_reply("tree")
	if not tree.contains("EquipmentHead (") or not tree.contains("EquipmentHands ("):
		fail("CLI dump-tree lacks the attached equipment")
		return false
	var shot := screenshot()
	if shot == null:
		return false
	# Each character shows in the CLI frame: hiding it changes its pixels.
	for name in ["DebugCharacterGeoset", "DebugCharacterM2"]:
		var model := scene.get_node(name) as Node3D
		model.visible = false
		var hidden := await capture()
		model.visible = true
		var shown := await capture()
		var live_pixels := changed_pixels(shown, hidden)
		var cli_pixels := shared_changed_pixels(shot, shown, hidden)
		print("FIXTURE DEBUGCHARACTER_PIXELS %s cli=%d live=%d" % [name, cli_pixels, live_pixels])
		if live_pixels < MIN_MODEL_PIXELS or cli_pixels < live_pixels * 0.8:
			fail("CLI screenshot shows %d of %s's %d live pixels" % [cli_pixels, name, live_pixels])
			return false
	return true

# Default displays (DebugCharacterConfig): shared shoulder 148865, back 181925, chest
# 175942, then each side's head, hands, waist, legs and feet.
const SLOT_DISPLAYS := {
	"DebugCharacterGeoset": [["Head", 1128], ["ShoulderLeft", 148865], ["ShoulderRight", 148865], ["Back", 181925], ["Chest", 175942], ["Hands", 510], ["Waist", 109162], ["Legs", 159629], ["Feet", 154620]],
	"DebugCharacterM2": [["Head", 685129], ["ShoulderLeft", 148865], ["ShoulderRight", 148865], ["Back", 181925], ["Chest", 175942], ["Hands", 154616], ["Waist", 160997], ["Legs", 73783], ["Feet", 154620]],
}

# `export-scene` writes the original DebugCharacterScene (src/scenes/geoset_debug):
# both characters, sorted by label, as human males with one slot per display, then the
# camera, light and ground.
func check_export() -> bool:
	var exported := exported_scene()
	if exported.is_empty():
		return false
	if exported.label != "DebugCharacterScene" or exported.props != "Scene":
		fail("Exported root %s %s" % [exported.label, exported.props])
		return false
	if child_labels(exported) != ["DebugCharacterGeoset", "DebugCharacterM2", "Camera", "Light", "Ground"]:
		fail("Exported DebugCharacterScene children: %s" % [child_labels(exported)])
		return false
	for name in SLOT_DISPLAYS:
		if not check_character(scene_child(exported, name), scene.get_node(name) as Node3D):
			return false
	return true

func check_character(exported: Dictionary, model: Node3D) -> bool:
	var props := props_of(exported, "Character")
	var source := str(model.get_meta("m2_source_path"))
	if props.model != source.get_file() or props.race != "Human" or props.gender != "Male" or props.name != null or props.character_id != null:
		fail("Exported %s character %s, loader input %s" % [model.name, props, source])
		return false
	if not transform_matches(exported, model.transform):
		return false
	var slots: Array = SLOT_DISPLAYS[model.name]
	if child_labels(exported) != slots.map(func(slot): return "Slot:" + slot[0]):
		fail("Exported %s slots %s" % [model.name, child_labels(exported)])
		return false
	var attached := 0
	for i in slots.size():
		var slot: Dictionary = exported.children[i]
		var equipment := props_of(slot, "EquipmentSlot")
		var item := model.find_child("Equipment" + slots[i][0], true, false) as Node3D
		if equipment.slot != slots[i][0] or equipment.model != "display:%d" % slots[i][1]:
			fail("Exported %s %s" % [model.name, slot])
			return false
		if item == null:
			if equipment.anchor != null or equipment.attachment != null or slot.transform != null:
				fail("Exported %s %s names an item the scene lacks" % [model.name, slot])
				return false
			continue
		attached += 1
		var anchor := str(item.get_parent().name)
		if equipment.attachment != str(item.name) or equipment.anchor != anchor or equipment.attachment_anchor != anchor:
			fail("Exported %s %s, live parent %s" % [model.name, equipment, anchor])
			return false
		if not transform_matches(slot, model.global_transform.affine_inverse() * item.global_transform):
			return false
	print("FIXTURE DEBUGCHARACTER_EXPORT %s slots=%d attached=%d" % [model.name, slots.size(), attached])
	return attached > 0
