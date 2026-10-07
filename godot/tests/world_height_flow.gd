extends "res://tests/world_lighting_flow.gd"

var sampled_position := Vector3.ZERO

func inspect_material_tiles(client: Node, parsed_tiles: Array) -> bool:
	if not super.inspect_material_tiles(client, parsed_tiles):
		return false
	if not client.has_method("terrain_height_at"):
		fail("Native world cannot query authored terrain height")
		return false
	var terrain := client.get_node("WorldTerrain")
	for tile in terrain.get_children():
		var chunk: MeshInstance3D = terrain_chunks(tile)[0]
		var arrays := chunk.mesh.surface_get_arrays(0)
		var positions: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
		var indices: PackedInt32Array = arrays[Mesh.ARRAY_INDEX]
		var center := (positions[indices[0]] + positions[indices[1]] + positions[indices[2]]) / 3.0
		sampled_position = chunk.to_global(center)
		var height = client.terrain_height_at(sampled_position.x, sampled_position.z)
		if height == null or absf(float(height) - sampled_position.y) > 0.02:
			fail("Authored height differs from rendered triangle at %s: %s" % [sampled_position, height])
			return false
	if client.terrain_height_at(100000000.0, 100000000.0) != null:
		fail("Unloaded terrain must report absent height, not a ground plane")
		return false
	print("PASS: retained terrain heights match triangle centers across %d rendered tiles" % terrain.get_child_count())
	return true

func inspect_reset(client: Node) -> bool:
	if not super.inspect_reset(client):
		return false
	if client.terrain_height_at(sampled_position.x, sampled_position.z) != null:
		fail("Reconnect retained previous terrain height")
		return false
	print("PASS: reconnect removes terrain height queries")
	return true
