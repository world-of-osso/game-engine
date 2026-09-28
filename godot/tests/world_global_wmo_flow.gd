extends "res://tests/world_transfer_flow.gd"

const STOCKADE := Vector3(103.0, -34.5, -76.0)
const FLOOR_Y := -34.9
const GROUP_COUNT := 27

var initial_loading_seen := false
var initial_loading_error := ""

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if not server.begins_with("127.0.0.1:") or server.ends_with(":0"):
		fail("Fixture requires its owned ephemeral loopback UDP endpoint")
		return
	var client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	client.screen_requested.connect(on_initial_screen_requested.bind(client))
	var error = client.connect_account(server, "fixture", "fixture", false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await wait_for_screen(client, "CharacterSelect", 15000):
		return
	var ui = client.get_node_or_null("CharacterSelectUI")
	var card = ui.find_child("CharCard_0", true, false) if ui != null else null
	var enter = ui.find_child("EnterWorld", true, false) if ui != null else null
	if not card is Control or not enter is Button or not card.visible or not enter.visible:
		fail("Authenticated character card and Enter World action missing")
		return
	await click_control(card)
	await click_control(enter)
	if not await wait_for_stockade_metadata(client, 45000):
		return
	print("FIXTURE GLOBAL_WMO_METADATA")
	if not await wait_for_stockade_scene(client, 70000):
		return
	print("FIXTURE GLOBAL_WMO_READY")
	var reconnect_error = client.connect_account(server, "fixture", "fixture", false)
	if reconnect_error != "":
		fail("Fixture reset connection: " + reconnect_error)
		return
	var reset: Dictionary = client.account_state()
	var terrain: Dictionary = reset.terrain
	if client.get_node_or_null("WorldWmos") != null or client.get_node_or_null("WorldTerrain") != null:
		fail("connect_account retained previous native world geometry")
		return
	if not terrain.map.is_empty() or not terrain.wdt_path.is_empty() or terrain.global_wmo_fdid != null or not terrain.parsed_tiles.is_empty() or terrain.pending_count != 0:
		fail("connect_account retained Stockade map state: " + str(terrain))
		return
	print("FIXTURE GLOBAL_WMO_RESET")
	client.free()
	quit(0)

func on_initial_screen_requested(screen: String, client: Node) -> void:
	if screen != "Loading" or initial_loading_seen:
		return
	initial_loading_seen = true
	var loading_ui = client.get_node_or_null("LoadingUI")
	if loading_ui == null or not loading_ui.visible or client.get_node_or_null("WorldWmos") != null:
		initial_loading_error = "LoadingUI must appear before global WMO attachment"

func wait_for_stockade_metadata(client: Node, timeout_ms: int) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if initial_loading_error != "":
			fail(initial_loading_error)
			return false
		var state: Dictionary = client.account_state()
		var terrain: Dictionary = state.terrain
		if not terrain.failures.is_empty():
			fail("Stockade WDT/resource failure: " + str(terrain))
			return false
		if terrain.map != "stormwindjail" or terrain.wdt_path.is_empty() or terrain.global_wmo_fdid == null:
			continue
		if not initial_loading_seen or terrain.global_wmo_fdid != 108631 or not FileAccess.file_exists(terrain.wdt_path):
			fail("Initial Loading or Stockade FDID108631 WDT metadata missing: " + str(state))
			return false
		if terrain.wdt_flags == null or (int(terrain.wdt_flags) & 1) == 0 or terrain.pending_count != 0 or not terrain.parsed_tiles.is_empty():
			fail("Stockade must be global-WMO-only with no requested ADT tiles: " + str(terrain))
			return false
		return true
	fail("Timed out waiting for parsed Stockade WDT/resource metadata: " + str(client.account_state()))
	return false

func wait_for_stockade_scene(client: Node, timeout_ms: int) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		var terrain: Dictionary = state.terrain
		if not terrain.failures.is_empty():
			fail("Stockade native WMO resource failure: " + str(terrain))
			return false
		var wmos = client.get_node_or_null("WorldWmos")
		if state.screen == "InWorld" and (wmos == null or not has_authored_groups(wmos)):
			fail("InWorld before actual Stockade group MeshInstance3D surfaces: " + str(state))
			return false
		if state.screen != "InWorld":
			if state.screen != "Loading" or not client.get_node("LoadingUI").visible:
				fail("Loading ended before global WMO attachment: " + str(state))
				return false
			continue
		if wmos == null or not has_authored_groups(wmos):
			return false
		var player = client.get_node_or_null("WorldUnits/Transfer Fixture") as Node3D
		if state.selected_character_name != "Transfer Fixture" or state.unit_count != 1 or player == null:
			fail("Selected player absent from native Stockade world: " + str(state))
			return false
		if client.get_node_or_null("WorldTerrain") != null or not terrain.parsed_tiles.is_empty():
			fail("Global WMO map spawned ADT terrain")
			return false
		if client.get_node("LoadingUI").visible:
			fail("LoadingUI remained visible after global WMO readiness")
			return false
		if absf(player.position.x - STOCKADE.x) > 0.5 or absf(player.position.z - STOCKADE.z) > 0.5:
			fail("Local player moved away from authored Stockade entry: " + str(player.position))
			return false
		# The world camera follows the player (15 yd from the eye) on a map without ADT tiles.
		var camera := client.get_viewport().get_camera_3d()
		if absf(player.position.y - FLOOR_Y) <= 0.2 and camera != null \
				and camera.global_position.distance_to(player.global_position) < 20.0:
			return true
	var camera := client.get_viewport().get_camera_3d()
	fail("Timed out waiting for 27 authored Stockade groups, grounded player near -34.9 and its camera (%s): %s" % [
		camera.global_position if camera != null else "none", client.account_state()])
	return false

## Every one of the 27 authored groups draws at least one mesh batch ("Group<g>_Batch<i>").
func has_authored_groups(wmos: Node) -> bool:
	if wmos.get_child_count() != 1:
		return false
	var placement = wmos.get_node_or_null("GlobalWmo108631") as Node3D
	if placement == null:
		return false
	var drawn := {}
	for instance in placement.find_children("Group*_Batch*", "MeshInstance3D", true, false):
		if instance.mesh != null and instance.mesh.get_surface_count() > 0:
			drawn[str(instance.name).get_slice("_", 0)] = true
	for index in GROUP_COUNT:
		if not drawn.has("Group%d" % index):
			print("FIXTURE GLOBAL_WMO_MISSING_GROUP ", index, " drawn=", drawn.size())
			return false
	return true
