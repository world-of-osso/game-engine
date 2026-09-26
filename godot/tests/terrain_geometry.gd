extends SceneTree

const TILE := "res://../data/terrain/azeroth_32_48.adt"

func fail(message: String) -> void:
	push_error(message)
	quit(1)

func _initialize() -> void:
	if not ClassDB.class_exists("WowTerrainLoader"):
		fail("WowTerrainLoader not registered")
		return
	var loader = ClassDB.instantiate("WowTerrainLoader")
	var result: Dictionary = loader.load_adt_geometry(TILE, 48, 32)
	if result.has("error") or not result.get("node") is Node3D:
		fail("ADT fixture: " + str(result.get("error", "missing Node3D")))
		return
	var terrain: Node3D = result.node
	get_root().add_child(terrain)
	if terrain.get_child_count() != 256:
		fail("Expected 256 rendered chunks, got %d" % terrain.get_child_count())
		return
	var first := terrain.get_node_or_null("Chunk0_0") as MeshInstance3D
	var holed := terrain.get_node_or_null("Chunk3_3") as MeshInstance3D
	if first == null or holed == null:
		fail("Missing authored chunk instances")
		return
	var arrays := first.mesh.surface_get_arrays(0)
	var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var normals: PackedVector3Array = arrays[Mesh.ARRAY_NORMAL]
	var colors: PackedColorArray = arrays[Mesh.ARRAY_COLOR]
	var uvs: PackedVector2Array = arrays[Mesh.ARRAY_TEX_UV]
	var indices: PackedInt32Array = arrays[Mesh.ARRAY_INDEX]
	if vertices.size() != 145 or normals.size() != 145 or colors.size() != 145 or uvs.size() != 145 or indices.size() != 768:
		fail("Chunk missing original vertex channels or center-fan topology")
		return
	if not vertices[0].is_equal_approx(Vector3(0.0, 233.5927734375, 8533.333984375)) or not is_equal_approx(vertices[1].y, 228.68878) or not uvs[9].is_equal_approx(Vector2(0.0625, 0.0625)):
		fail("Fixture header/height/UV samples differ from authored terrain")
		return
	if indices[0] != 0 or indices[1] != 1 or indices[2] != 9 or indices[3] != 1 or indices[4] != 18 or indices[5] != 9:
		fail("Godot front-face winding did not reverse Bevy triangle indices")
		return
	var face_normal := (vertices[indices[1]] - vertices[indices[0]]).cross(vertices[indices[2]] - vertices[indices[0]]).normalized()
	if face_normal.dot(normals[0]) >= -0.5:
		fail("Front-face winding does not face authored terrain normal")
		return
	var hole_indices: PackedInt32Array = holed.mesh.surface_get_arrays(0)[Mesh.ARRAY_INDEX]
	if hole_indices.size() != 672 or hole_indices.has(115):
		fail("High-resolution hole must omit eight quads (96 indices)")
		return
	if not loader.load_adt_geometry("res://../data/terrain/absent.adt", -1, -1).has("error"):
		fail("Missing ADT succeeded")
		return
	if not loader.load_adt_geometry(TILE, -2, 32).has("error"):
		fail("Invalid tile coordinate succeeded")
		return
	var malformed := FileAccess.open("user://invalid_terrain.adt", FileAccess.WRITE)
	malformed.store_buffer(PackedByteArray([1, 2, 3]))
	malformed.close()
	var invalid_result: Dictionary = loader.load_adt_geometry("user://invalid_terrain.adt", -1, -1)
	DirAccess.remove_absolute(ProjectSettings.globalize_path("user://invalid_terrain.adt"))
	if not invalid_result.has("error") or invalid_result.has("node"):
		fail("Malformed ADT succeeded or returned a partial node")
		return
	print("PASS: 256 native terrain chunks, authored heights/UVs/colors, hole topology and Godot winding")
	quit(0)
