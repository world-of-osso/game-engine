extends SceneTree

const DATA := "res://../data/"

func fail(message: String) -> void:
	push_error(message)
	quit(1)

func _initialize() -> void:
	if not ClassDB.class_exists("WowAssetLoader"):
		fail("WowAssetLoader not registered")
		return
	var loader = ClassDB.instantiate("WowAssetLoader")
	var hd = loader.load_m2(DATA + "models/humanmale_hd.m2")
	if hd.has("error"):
		fail("HD load: " + hd.error)
		return
	var root: Node3D = hd.node
	if not root is Node3D:
		fail("HD load did not return a Node3D")
		return
	get_root().add_child(root)
	if not root.get_node_or_null("M2Animation") is WowAnimationPlayer:
		fail("HD model missing native M2 animation player")
		return
	var skeleton := root.get_node_or_null("Skeleton3D") as Skeleton3D
	if skeleton == null or skeleton.get_bone_count() != 216:
		fail("HD skeleton must contain 216 authored bones")
		return
	# Animation starts at sequence 0; inspect authored rest instead of the animated pose.
	skeleton.reset_bone_poses()
	var nonzero_pivots := 0
	for bone in skeleton.get_bone_count():
		var global_pose := skeleton.get_bone_global_pose(bone)
		var palette := global_pose * (root.get_node("Batch0") as MeshInstance3D).skin.get_bind_pose(bone)
		if global_pose.origin.length() > 0.01:
			nonzero_pivots += 1
		if palette.origin.length() > 0.001:
			fail("rest palette not identity at bone %d" % bone)
			return
	if nonzero_pivots == 0:
		fail("HD fixture missing nonzero pivots")
		return
	var mesh_instances := root.find_children("*", "MeshInstance3D", true, false)
	if mesh_instances.is_empty():
		fail("HD model has no rendered batches")
		return
	var vertex_count := 0
	var index_count := 0
	var weighted := false
	for instance: MeshInstance3D in mesh_instances:
		if instance.skin == null or instance.skeleton != NodePath("../Skeleton3D"):
			fail("mesh missing skeleton skin binding")
			return
		for surface in instance.mesh.get_surface_count():
			var arrays := instance.mesh.surface_get_arrays(surface)
			vertex_count += arrays[Mesh.ARRAY_VERTEX].size()
			index_count += arrays[Mesh.ARRAY_INDEX].size()
			if arrays[Mesh.ARRAY_BONES].size() == arrays[Mesh.ARRAY_VERTEX].size() * 4 and arrays[Mesh.ARRAY_WEIGHTS].size() == arrays[Mesh.ARRAY_BONES].size():
				weighted = true
	if mesh_instances.size() != 113 or vertex_count != 37813 or index_count != 147966 or not weighted:
		fail("HD batches mismatch authored indexed skinned geometry: %d batches %d vertices %d indices" % [mesh_instances.size(), vertex_count, index_count])
		return
	var torch = loader.load_m2(DATA + "models/club_1h_torch_a_01.m2")
	if torch.has("error") or not torch.node is Node3D:
		fail("torch fixture failed to load: " + str(torch.get("error", "no node")))
		return
	var textured := false
	var outward_triangles := 0
	for instance: MeshInstance3D in torch.node.find_children("*", "MeshInstance3D", true, false):
		var material := instance.get_active_material(0) as ShaderMaterial
		if material != null and material.get_shader_parameter("base_texture") != null:
			textured = true
		var arrays := instance.mesh.surface_get_arrays(0)
		var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
		var normals: PackedVector3Array = arrays[Mesh.ARRAY_NORMAL]
		var indices: PackedInt32Array = arrays[Mesh.ARRAY_INDEX]
		for triangle in range(0, indices.size(), 3):
			var a := indices[triangle]
			var b := indices[triangle + 1]
			var c := indices[triangle + 2]
			var face_normal := (vertices[b] - vertices[a]).cross(vertices[c] - vertices[a])
			var signed_area := face_normal.dot(normals[a])
			if absf(signed_area) > 0.00001:
				outward_triangles += 1
				if signed_area > 0.0:
					fail("Godot clockwise front face opposes authored normal")
					return
	if outward_triangles == 0:
		fail("torch fixture lacks winding evidence")
		return
	if not textured:
		fail("torch missing authored FDID texture")
		return
	var image_result = loader.load_blp(DATA + "textures/145513.blp")
	if image_result.has("error") or not image_result.image is Image:
		fail("BLP failed: " + str(image_result.get("error", "no image")))
		return
	var image: Image = image_result.image
	if image.get_width() <= 0 or image.get_height() <= 0 or image.get_data().is_empty():
		fail("BLP has no decoded pixels")
		return
	if not loader.load_m2(DATA + "models/absent.m2").has("error"):
		fail("missing model succeeded")
		return
	var malformed := FileAccess.open("user://invalid_asset.m2", FileAccess.WRITE)
	malformed.store_buffer(PackedByteArray([1, 2, 3]))
	malformed.close()
	var malformed_skin := FileAccess.open("user://invalid_asset00.skin", FileAccess.WRITE)
	malformed_skin.store_buffer(PackedByteArray([1, 2, 3]))
	malformed_skin.close()
	var invalid_result = loader.load_m2("user://invalid_asset.m2")
	DirAccess.remove_absolute(ProjectSettings.globalize_path("user://invalid_asset.m2"))
	DirAccess.remove_absolute(ProjectSettings.globalize_path("user://invalid_asset00.skin"))
	if not invalid_result.has("error"):
		fail("malformed M2 succeeded")
		return
	if not loader.decode_blp(PackedByteArray([1, 2, 3])).has("error"):
		fail("invalid BLP succeeded")
		return
	if not loader.load_blp(DATA + "textures/absent.blp").has("error"):
		fail("missing BLP succeeded")
		return
	print("PASS: HD %d bones, %d vertices, %d indices; torch and BLP decoded" % [skeleton.get_bone_count(), vertex_count, index_count])
	torch.node.free()
	root.free()
	hd.clear()
	torch.clear()
	image_result.clear()
	loader = null
	quit(0)
