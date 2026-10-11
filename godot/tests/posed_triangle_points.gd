extends RefCounted

# Shared fixture sampling of actual posed geometry, matching UnitPicker's palette.
const TRIANGLE_SAMPLES := 128

static func bone_palette(mesh: MeshInstance3D) -> Array[Transform3D]:
	var palette: Array[Transform3D] = []
	var skeleton := mesh.get_node_or_null(mesh.skeleton) as Skeleton3D
	if mesh.skin == null or skeleton == null:
		return palette
	for bind in range(mesh.skin.get_bind_count()):
		palette.append(skeleton.get_bone_global_pose(mesh.skin.get_bind_bone(bind)) * mesh.skin.get_bind_pose(bind))
	return palette

static func centroids(mesh: MeshInstance3D, surface: int) -> Array[Vector3]:
	var palette := bone_palette(mesh)
	var centers: Array[Vector3] = []
	var arrays := mesh.mesh.surface_get_arrays(surface)
	var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	if vertices.is_empty():
		return centers
	var indices: PackedInt32Array = arrays[Mesh.ARRAY_INDEX] if arrays[Mesh.ARRAY_INDEX] != null else PackedInt32Array()
	var bones: PackedInt32Array = arrays[Mesh.ARRAY_BONES] if arrays[Mesh.ARRAY_BONES] != null else PackedInt32Array()
	var weights: PackedFloat32Array = arrays[Mesh.ARRAY_WEIGHTS] if arrays[Mesh.ARRAY_WEIGHTS] != null else PackedFloat32Array()
	var triangles := floori(float(vertices.size() if indices.is_empty() else indices.size()) / 3.0)
	var samples := mini(triangles, TRIANGLE_SAMPLES)
	for sample in range(samples):
		var triangle := floori(float(sample) * triangles / samples)
		var center := Vector3.ZERO
		for corner in range(3):
			var slot := triangle * 3 + corner
			var index := slot if indices.is_empty() else indices[slot]
			center += posed_vertex(vertices, bones, weights, palette, index)
		centers.append(mesh.global_transform * (center / 3.0))
	return centers

static func posed_vertex(vertices: PackedVector3Array, bones: PackedInt32Array, weights: PackedFloat32Array, palette: Array[Transform3D], index: int) -> Vector3:
	var vertex := vertices[index]
	if palette.is_empty() or bones.is_empty():
		return vertex
	var influences := floori(float(bones.size()) / vertices.size())
	var posed := Vector3.ZERO
	for slot in range(index * influences, (index + 1) * influences):
		var bone := bones[slot]
		if bone >= 0 and bone < palette.size() and slot < weights.size() and weights[slot] > 0.0:
			posed += (palette[bone] * vertex) * weights[slot]
	return posed
