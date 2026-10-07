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
		# Material tiles attach a few per frame under the resource budget (terrain/material.rs).
		var built := client.get_node_or_null("WorldTerrain")
		if built == null or built.get_child_count() != terrain.parsed_tiles.size():
			continue
		for tile in terrain.parsed_tiles:
			if tile.chunk_count <= 0 or not FileAccess.file_exists(tile.root_path):
				fail("Parsed tile lacks actual terrain geometry or cache file: " + str(tile))
				return
		# The loading gate also waits for nearby scenery (79c98f01), not terrain alone.
		if state.screen != "InWorld":
			continue
		if not await inspect_material_tiles(client, terrain.parsed_tiles):
			return
		var loading_ui := client.get_node_or_null("LoadingUI")
		if loading_ui == null:
			fail("Loading screen replacement lost its stable node name")
			return
		if loading_ui.visible:
			fail("Completed loading gate retained visible loading screen")
			return
		var error = client.connect_account(server, account, password, false)
		if error != "":
			fail("Reconnect failed: " + error)
			return
		var reset: Dictionary = client.account_state().terrain
		if client.get_node_or_null("WorldTerrain") != null or client.get_node_or_null("WorldLighting") != null:
			fail("Reconnect retained previous map material or lighting nodes")
			return
		if not reset.map.is_empty() or not reset.parsed_tiles.is_empty() or reset.pending_count != 0:
			fail("Reconnect retained previous map asset state: " + str(reset))
			return
		if not await inspect_reset(client):
			return
		print("PASS: real LoadTerrain projects authored terrain, completes original loading gate, and reconnect clears the world")
		client.free()
		quit(0)
		return
	fail("Timed out waiting for server-requested terrain assets: " + str(client.account_state()))

func inspect_reset(_client: Node) -> bool:
	return true

func inspect_material_tiles(client: Node, parsed_tiles: Array) -> bool:
	var root := client.get_node_or_null("WorldTerrain")
	if root == null or root.get_child_count() != parsed_tiles.size():
		fail("Parsed terrain has no corresponding native material tiles")
		return false
	for tile in root.get_children():
		var chunks := terrain_chunks(tile)
		if chunks.is_empty():
			fail("Material tile has no terrain chunks")
			return false
		for instance in chunks:
			var material := instance.get_surface_override_material(0) as ShaderMaterial
			if material == null or material.shader == null:
				fail("Terrain chunk lacks native authored shader material")
				return false
			var config: Vector4 = material.get_shader_parameter("config")
			if config.x < 1 or config.x > 4 or config.z < 8:
				fail("Terrain material has invalid authored layer count/repeat")
				return false
			for slot in range(int(config.x)):
				var diffuse := material.get_shader_parameter("ground_%d" % slot) as Texture2D
				if diffuse == null or diffuse.get_width() <= 1 or diffuse.get_height() <= 1:
					fail("Terrain material lacks decoded authored diffuse")
					return false
			var alpha := material.get_shader_parameter("alpha_packed") as Texture2D
			if alpha == null or alpha.get_width() != 64 or alpha.get_height() != 64:
				fail("Terrain material lacks 64x64 MCAL texture")
				return false
	return true

## A tile's terrain chunk meshes; a tile with liquid also holds a Water node first.
func terrain_chunks(tile: Node) -> Array[MeshInstance3D]:
	var chunks: Array[MeshInstance3D] = []
	for child in tile.get_children():
		if child is MeshInstance3D and child.name.begins_with("Chunk"):
			chunks.append(child)
	return chunks
