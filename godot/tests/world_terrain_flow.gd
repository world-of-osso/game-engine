extends "res://tests/world_units_flow.gd"
const TerrainChunkInputs := preload("res://tests/terrain_chunk_inputs.gd")

func select_second_character(client: Node) -> void:
	if not client.account_state().has("terrain"):
		fail("Native host does not expose server-requested terrain asset state")
		return
	var ui = client.get_node("CharacterSelectUI")
	var expected_name: String = ui.find_child("CharCard_1Name", true, false).text
	await click_control(ui.find_child("CharCard_1", true, false))
	if not await CharacterSelectReveal.wait_for_reveal(self, client, "CharCard_1", expected_name):
		fail("Second character preview did not reveal selection: " + str(client.account_state()))
		return
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
		var meshes := terrain_chunks(tile)
		var chunks := TerrainChunkInputs.tile_chunks(tile)
		if meshes.size() != 1 or chunks.is_empty():
			fail("Material tile lacks its terrain mesh or chunks: %d meshes, %d chunks" % [meshes.size(), chunks.size()])
			return false
		var material := meshes[0].get_surface_override_material(0) as ShaderMaterial
		if material == null or material.shader == null:
			fail("Terrain tile lacks native authored shader material")
			return false
		var data_texture := material.get_shader_parameter("chunk_data") as Texture2D
		var alpha := material.get_shader_parameter("alpha_atlas") as Texture2D
		if data_texture == null or alpha == null or alpha.get_width() != 1024 or alpha.get_height() != 1024:
			fail("Terrain material lacks chunk data or the 16x16 64x64 MCAL atlas")
			return false
		var data := data_texture.get_image()
		for index in chunks:
			var config := TerrainChunkInputs.chunk_row(data, index, 0)
			if config.x < 1 or config.x > 4 or config.z < 8:
				fail("Terrain chunk %s has invalid authored layer count/repeat" % index)
				return false
			var ground := TerrainChunkInputs.chunk_row(data, index, 9)
			for slot in range(int(config.x)):
				var layers := TerrainChunkInputs.layer_array(material, ground[slot]) if ground[slot] >= 0.0 else null
				if layers == null or layers.get_width() <= 1 or layers.get_height() <= 1:
					fail("Terrain chunk %s lacks decoded authored diffuse" % index)
					return false
	return true

## A tile's terrain mesh: every chunk in one draw, beside its Water node and collision bodies.
func terrain_chunks(tile: Node) -> Array[MeshInstance3D]:
	var chunks: Array[MeshInstance3D] = []
	for child in tile.get_children():
		if child is MeshInstance3D and child.name == "Terrain":
			chunks.append(child)
	return chunks
