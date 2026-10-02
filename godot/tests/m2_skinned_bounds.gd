extends SceneTree

# Skinned M2 batches cull by the M2 header bounding box (WebWowViewerCpp
# M2Object::createAABB), so Godot never re-derives their AABB from bones.
# The box must still hold every posed vertex of every authored sequence.
const DATA := "res://../data/models/boar.m2"
# boar.m2 MD20 bounding_box, WoW (x,y,z) -> Godot (x,z,-y).
const HEADER := AABB(Vector3(-2.3635879, -1.1234875, -2.1709828), Vector3(5.3628049, 3.5954743, 3.5137058))
const STEP_MS := 100.0
const STEPS := 20

func fail(message: String) -> void:
	push_error(message)
	quit(1)

func skinned_batches(root: Node3D) -> Array[MeshInstance3D]:
	var batches: Array[MeshInstance3D] = []
	for child in root.get_children():
		if child is MeshInstance3D and child.skin != null:
			batches.append(child)
	return batches

func posed_vertices(batch: MeshInstance3D, skeleton: Skeleton3D) -> PackedVector3Array:
	var arrays := batch.mesh.surface_get_arrays(0)
	var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var bones: PackedInt32Array = arrays[Mesh.ARRAY_BONES]
	var weights: PackedFloat32Array = arrays[Mesh.ARRAY_WEIGHTS]
	var per_vertex := bones.size() / vertices.size()
	var palette: Array[Transform3D] = []
	for bind in batch.skin.get_bind_count():
		palette.append(skeleton.get_bone_global_pose(batch.skin.get_bind_bone(bind)) * batch.skin.get_bind_pose(bind))
	var posed := PackedVector3Array()
	for v in vertices.size():
		var position := Vector3.ZERO
		for k in per_vertex:
			var weight := weights[v * per_vertex + k]
			if weight > 0.0:
				position += palette[bones[v * per_vertex + k]] * vertices[v] * weight
		posed.append(position)
	return posed

func _initialize() -> void:
	var loaded: Dictionary = ClassDB.instantiate("WowAssetLoader").load_m2(DATA)
	if loaded.has("error"):
		fail("boar: " + loaded.error)
		return
	var root: Node3D = loaded.node
	get_root().add_child(root)
	var skeleton := root.get_node("Skeleton3D") as Skeleton3D
	var player := root.get_node("M2Animation")
	var batches := skinned_batches(root)
	if batches.is_empty():
		fail("boar has no skinned batches")
		return
	var rest := AABB()
	for batch in batches:
		if not batch.custom_aabb.is_equal_approx(HEADER):
			fail("%s custom AABB %s, expected header box %s" % [batch.name, batch.custom_aabb, HEADER])
			return
		rest = batch.mesh.get_aabb() if rest.size == Vector3.ZERO else rest.merge(batch.mesh.get_aabb())
	var sequences := 0
	var beyond_rest := 0.0
	while player.play_sequence(sequences, true):
		for step in STEPS:
			player.advance_time_ms(STEP_MS)
			for batch in batches:
				for position in posed_vertices(batch, skeleton):
					if not HEADER.grow(0.001).has_point(position):
						fail("sequence %d at %d ms: %s vertex %s outside header box" % [sequences, (step + 1) * STEP_MS, batch.name, position])
						return
					beyond_rest = max(beyond_rest, position.distance_to(position.clamp(rest.position, rest.end)))
		sequences += 1
	if sequences != 17:
		fail("expected 17 boar sequences, played %d" % sequences)
		return
	if beyond_rest < 0.05:
		fail("boar animation never left its rest-pose AABB (max %.3f)" % beyond_rest)
		return
	print("PASS: %d skinned batches use the header box; %d sequences stay inside it, reaching %.3f beyond the rest AABB" % [batches.size(), sequences, beyond_rest])
	quit(0)
