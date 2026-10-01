extends "res://tests/world_menu_flow.gd"

# Run only with native_input_fixture dev-ipc. The parent drives this client through the
# public game-engine-cli; this script only observes what those requests change:
# the hover tooltip, the hover leaving it, and the live camera after `camera set`.
const VENDOR := "Fixture Vendor"
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
	print("FIXTURE DEV_IPC_CAMERA forward=%s" % camera_forward())
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

func camera_forward() -> Vector3:
	var camera := root.get_viewport().get_camera_3d()
	return -camera.global_basis.z if camera != null else Vector3.ZERO
