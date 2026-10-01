extends "res://tests/world_menu_flow.gd"

# Run only with native_input_fixture dev-ipc. The parent drives this client through the
# public game-engine-cli; this script only observes what those requests change:
# the hover tooltip, the hover leaving it, and the live camera after `camera set`.
const VENDOR := "Fixture Vendor"
const REMOTE := "Remote Fixture"
const OBSERVE_MS := 60000
# `camera set --yaw-degrees 90 --pitch-degrees -20`: the orbit direction the camera
# looks along, Quat::from_euler(YXZ, yaw, pitch, 0) * -Z (camera_follow_data.rs).
const CAMERA_YAW := deg_to_rad(90.0)
const CAMERA_PITCH := deg_to_rad(-20.0)

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	if not endpoint.begins_with("127.0.0.1:") or endpoint.ends_with(":0"):
		fail("Dev IPC fixture requires its owned loopback endpoint")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_screen(client, "Loading", 180000):
		return
	print("FIXTURE DEV_IPC_LOADING")
	var tiles := await wait_world_and_vendor(client)
	if tiles < 0:
		return
	print("FIXTURE DEV_IPC_READY tiles=%d" % tiles)
	if not await observe(client, "hover tooltip", func(): return tooltip_title(client) == VENDOR):
		return
	print("FIXTURE DEV_IPC_HOVER_NPC " + VENDOR)
	if not await observe(client, "hover leaving the vendor", func(): return tooltip_title(client) == ""):
		return
	print("FIXTURE DEV_IPC_HOVER_POINT")
	var expected := Vector3(-sin(CAMERA_YAW) * cos(CAMERA_PITCH), sin(CAMERA_PITCH), -cos(CAMERA_YAW) * cos(CAMERA_PITCH))
	if not await observe(client, "camera direction", func(): return camera_forward().dot(expected) > 0.999):
		return
	# The camera eases into place; the parent's next request reads where it settles.
	if not await camera_settled():
		return
	print("FIXTURE DEV_IPC_CAMERA forward=%s" % camera_forward())
	if not await check_export(client):
		return
	if not await observe(client, "vendor removal", func(): return client.get_node_or_null("WorldUnits/" + VENDOR) == null):
		return
	print("FIXTURE DEV_IPC_DONE")
	client.free()
	quit(0)

# Loaded terrain tile count once the player, remote player and vendor are in the world.
func wait_world_and_vendor(client: Node) -> int:
	var deadline := Time.get_ticks_msec() + WORLD_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		var terrain: Dictionary = state.terrain
		if state.screen == "InWorld" and state.unit_count == 3 and terrain.map == "azeroth" and terrain.pending_count == 0 and terrain.failures.is_empty() and not terrain.parsed_tiles.is_empty() and client.get_node_or_null("WorldUnits/" + NAME) != null and client.get_node_or_null("WorldUnits/" + VENDOR) != null and state.unit_visuals_pending == 0:
			return terrain.parsed_tiles.size()
	fail("Timed out waiting for world and vendor: " + str(client.account_state()))
	return -1

func observe(client: Node, what: String, predicate: Callable) -> bool:
	var deadline := Time.get_ticks_msec() + OBSERVE_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call():
			return true
	fail("Timed out observing %s: tooltip %s" % [what, client.tooltip_state()])
	return false

func tooltip_title(client: Node) -> String:
	var tooltip: Dictionary = client.tooltip_state()
	return tooltip.title if tooltip.visible else ""

func camera_settled() -> bool:
	var still := 0
	var last := Transform3D()
	var deadline := Time.get_ticks_msec() + OBSERVE_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var current := root.get_viewport().get_camera_3d().global_transform
		still = still + 1 if current.is_equal_approx(last) else 0
		last = current
		if still >= 30:
			return true
	fail("Camera did not settle after camera set")
	return false

func camera_forward() -> Vector3:
	var camera := root.get_viewport().get_camera_3d()
	return -camera.global_basis.z if camera != null else Vector3.ZERO

# `export-scene` (the parent's CLI call) writes the original InWorldScene
# (src/scenes/inworld_tree.rs): both players, the vendor as `template_1213` with display
# 26, the camera and the sun, from the live nodes.
func check_export(client: Node) -> bool:
	var path := ProjectSettings.globalize_path("res://").path_join("../data/diagnostics/dev-ipc-%d/scene-export.json" % OS.get_process_id()).simplify_path()
	if not await observe(client, "the exported scene", func(): return FileAccess.file_exists(path) and not FileAccess.get_file_as_string(path).is_empty()):
		return false
	var value: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	if not value is Dictionary or not value.has("root"):
		fail("Exported scene is not a snapshot: " + FileAccess.get_file_as_string(path))
		return false
	var scene: Dictionary = value.root
	var labels: Array = scene.children.map(func(child): return child.label)
	labels.sort()
	if scene.label != "InWorldScene" or scene.props != "Scene" or scene.transform != null or labels != ["Camera", "EnvironmentSun", "Npc", "Player", "Player"]:
		fail("Exported in-world scene %s %s %s" % [scene.label, scene.props, labels])
		return false
	for child in scene.children:
		var checked := false
		match child.label:
			"Player":
				checked = check_player(client, child)
			"Npc":
				checked = check_npc(client, child)
			"Camera":
				var camera := root.get_viewport().get_camera_3d()
				checked = absf(child.props.Camera.fov - camera.fov) < 0.001 and matches(child, camera.global_transform)
				if not checked:
					print("live camera fov=%.3f %s" % [camera.fov, camera.global_transform])
			"EnvironmentSun":
				var sun := client.find_child("Sun", true, false) as DirectionalLight3D
				checked = sun != null and child.props.Light.kind == "DirectionalLight3D" and absf(child.props.Light.intensity - sun.light_energy) < 0.0001 and matches(child, sun.global_transform)
		if not checked:
			fail("Exported %s does not match the live scene: %s" % [child.label, child])
			return false
	print("FIXTURE DEV_IPC_EXPORT")
	return true

func check_player(client: Node, exported: Dictionary) -> bool:
	var props: Dictionary = exported.props.Player
	var unit := client.get_node_or_null("WorldUnits/" + props.name) as Node3D
	if unit == null or props.is_local != (props.name == NAME) or props.display_scale != null:
		return false
	var model := unit.get_node_or_null("PlayerModel") as Node3D
	return model != null and props.model_path == model.get_meta("m2_source_path") and props.skin_path == str(props.model_path).trim_suffix(".m2") + "00.skin" and matches(exported, unit.global_transform)

func check_npc(client: Node, exported: Dictionary) -> bool:
	var props: Dictionary = exported.props.Npc
	var unit := client.get_node_or_null("WorldUnits/" + VENDOR) as Node3D
	if unit == null or props.name != "template_1213" or props.display_id != 26:
		return false
	var visual := unit.get_node_or_null("NpcVisualRoot") as Node3D
	var model := unit.get_node_or_null("NpcVisualRoot/NpcModel") as Node3D
	return model != null and props.model_path == model.get_meta("m2_source_path") and absf(props.display_scale - visual.scale.x) < 0.0001 and props.skin_path == str(props.model_path).trim_suffix(".m2") + "00.skin" and matches(exported, unit.global_transform)

func matches(exported: Dictionary, expected: Transform3D) -> bool:
	var t: Variant = exported.transform
	if not t is Dictionary:
		return false
	var q := expected.basis.get_rotation_quaternion()
	var translation := Vector3(t.translation[0], t.translation[1], t.translation[2])
	var rotation := Quaternion(t.rotation[0], t.rotation[1], t.rotation[2], t.rotation[3])
	return translation.distance_to(expected.origin) < 0.01 and absf(rotation.dot(q)) > 0.9999
