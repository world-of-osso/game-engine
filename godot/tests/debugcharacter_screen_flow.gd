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
