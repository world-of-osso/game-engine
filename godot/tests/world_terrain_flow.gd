extends "res://tests/world_units_flow.gd"

func select_second_character(client: Node) -> void:
	if not client.account_state().has("terrain"):
		fail("Native host does not expose server-requested terrain asset state")
		return
	var ui = client.get_node("CharacterSelectUI")
	await click_control(ui.find_child("CharCard_1", true, false))
	await click_control(ui.find_child("EnterWorld", true, false))
	var deadline := Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		var terrain: Dictionary = state.terrain
		if terrain.map.is_empty() or terrain.wdt_path.is_empty():
			continue
		if not terrain.failures.is_empty():
			fail("Server-requested terrain assets failed: " + str(terrain))
			return
		if terrain.pending_count != 0 or terrain.parsed_tiles.is_empty():
			continue
		for tile in terrain.parsed_tiles:
			if tile.chunk_count <= 0 or not FileAccess.file_exists(tile.root_path):
				fail("Parsed tile lacks actual terrain geometry or cache file: " + str(tile))
				return
		if state.screen != "Loading":
			fail("Parsed assets alone must not establish rendered world readiness")
			return
		var error = client.connect_account("127.0.0.1:5000", "admin", "admin", false)
		if error != "":
			fail("Reconnect failed: " + error)
			return
		var reset: Dictionary = client.account_state().terrain
		if not reset.map.is_empty() or not reset.parsed_tiles.is_empty() or reset.pending_count != 0:
			fail("Reconnect retained previous map asset state: " + str(reset))
			return
		print("PASS: real LoadTerrain reads local map/tile assets asynchronously; reconnect clears them without fabricated readiness")
		client.free()
		quit(0)
		return
	fail("Timed out waiting for server-requested terrain assets: " + str(client.account_state()))
