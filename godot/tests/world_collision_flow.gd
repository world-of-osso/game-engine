extends "res://tests/world_height_flow.gd"

var collision_sample := Vector3.ZERO

func intersect_surface(client: Node3D, point: Vector3) -> Dictionary:
	var query := PhysicsRayQueryParameters3D.create(point + Vector3.UP, point - Vector3.UP)
	return client.get_world_3d().direct_space_state.intersect_ray(query)

func inspect_material_tiles(client: Node, parsed_tiles: Array) -> bool:
	if not super.inspect_material_tiles(client, parsed_tiles):
		return false
	await physics_frame
	await process_frame
	var terrain := client.get_node("WorldTerrain")
	var holes := 0
	for tile in terrain.get_children():
		var chunk: MeshInstance3D = terrain_chunks(tile)[0]
		var arrays := chunk.mesh.surface_get_arrays(0)
		var positions: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
		var indices: PackedInt32Array = arrays[Mesh.ARRAY_INDEX]
		collision_sample = chunk.to_global((positions[indices[0]] + positions[indices[1]] + positions[indices[2]]) / 3.0)
		var hit := intersect_surface(client, collision_sample)
		if hit.is_empty() or (hit.position as Vector3).distance_to(collision_sample) > 0.02:
			fail("Native collision misses rendered terrain at %s: %s" % [collision_sample, hit])
			return false
		var hole_count := inspect_holes(client, chunk)
		if hole_count < 0:
			return false
		holes += hole_count
	if holes == 0:
		fail("Real terrain collision fixture did not exercise authored holes")
		return false
	print("PASS: native collision matches %d terrain tiles and leaves %d authored holes open" % [terrain.get_child_count(), holes])
	return true

# The tile mesh holds each chunk's 145 vertices in turn; a hole omits its quad's centre.
func inspect_holes(client: Node3D, chunk: MeshInstance3D) -> int:
	var arrays := chunk.mesh.surface_get_arrays(0)
	var positions: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var indexed := {}
	for index in arrays[Mesh.ARRAY_INDEX]:
		indexed[index] = true
	var holes := 0
	for base in range(0, positions.size(), 145):
		for row in range(8):
			for col in range(8):
				var center := base + row * 17 + 9 + col
				if indexed.has(center):
					continue
				var point := chunk.to_global(positions[center])
				# A hole leaves room for WMO floors: only terrain collision may not seal it.
				var hit := intersect_surface(client, point)
				if not hit.is_empty() and hit.collider.get_parent() == chunk.get_parent():
					fail("Terrain collision %s sealed an authored hole at %s" % [hit.collider.name, point])
					return -1
				holes += 1
	return holes

func inspect_reset(client: Node) -> bool:
	if not super.inspect_reset(client):
		return false
	await physics_frame
	await process_frame
	if not intersect_surface(client, collision_sample).is_empty():
		fail("Reconnect retained previous terrain collision")
		return false
	print("PASS: reconnect removes terrain collision")
	return true
