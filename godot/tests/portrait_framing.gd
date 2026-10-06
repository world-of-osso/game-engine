extends SceneTree

# Real detached HD heads, production camera and materials; no server.
var fixture: Node
var failed := false
var output := OS.get_environment("GODOT_CAPTURE_PATH").get_base_dir()

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	fixture = ClassDB.instantiate("PartyPortraitFixture")
	root.add_child(fixture)
	check(fixture.initialize(false).is_empty(), "initialize fixture")
	var names: Array[String] = ["Theron", "Jaina", "Valeera", "Uther"]
	check(fixture.set_members(names, false, true).is_empty(), "request four race/sex heads")
	for attempt in range(1200):
		check(fixture.tick().is_empty(), "load real models")
		await process_frame
		if fixture.ready_heads() == 4:
			break
	check(fixture.ready_heads() == 4, "four heads loaded")
	await settle()
	for name in names:
		var state: Dictionary = fixture.portrait_state(name)
		var host := fixture.find_child(state.frame, true, false)
		var camera := host.find_child("PortraitCamera", true, false) as Camera3D
		var viewport := host.find_child("PortraitViewport", true, false) as SubViewport
		var image := viewport.get_texture().get_image()
		check(image.save_png(output.path_join(name + "-raw.png")) == OK, "save raw head")
		# Head key-bone 6 indices from these four real SKB1 skeletons.
		var head_bones := {"Theron": 41, "Jaina": 42, "Valeera": 42, "Uther": 39}
		var bounds := project_head(host, camera, head_bones[name])
		var metrics := measure_face(image, bounds)
		var head_height := bounds.size.y / image.get_height()
		print("PORTRAIT_FACE ", name, " fov=", camera.fov, " head_height=", head_height, " bounds=", bounds, " metrics=", metrics)
		check(head_height >= 0.60 and head_height <= 1.10, "head fills portrait within tolerance: " + name)
		check(metrics.visible > 20 and metrics.mean > 0.12, "visible lit face pixels: " + name)
		var meshes := host.find_children("*", "MeshInstance3D", true, false)
		for index in range(meshes.size()):
			if not meshes[index].visible:
				continue
			var material := meshes[index].get_active_material(0) as ShaderMaterial
			var texture := material.get_shader_parameter("base_texture") as Texture2D
			if texture != null:
				texture.get_image().save_png(output.path_join(name + "-texture-%d.png" % index))
			print("PORTRAIT_MATERIAL ", name, " batch=", index, " part=", meshes[index].get_meta("m2_mesh_part", -1), " flags=", material.get_shader_parameter("render_flags"))
	if not failed:
		print("PASS portrait_framing_four_real_heads")
	fixture.free()
	await settle()
	quit(1 if failed else 0)

func project_head(host: Node, camera: Camera3D, head: int) -> Rect2:
	var skeleton := host.find_child("Skeleton3D", true, false) as Skeleton3D
	var minimum := Vector2(INF, INF)
	var maximum := Vector2(-INF, -INF)
	for mesh in host.find_children("*", "MeshInstance3D", true, false):
		if not mesh.visible:
			continue
		var arrays: Array = mesh.mesh.surface_get_arrays(0)
		var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
		var bones: PackedInt32Array = arrays[Mesh.ARRAY_BONES]
		var weights: PackedFloat32Array = arrays[Mesh.ARRAY_WEIGHTS]
		for index in range(vertices.size()):
			var head_weight := 0.0
			var posed := Vector3.ZERO
			for influence in range(4):
				var slot := index * 4 + influence
				var bind := bones[slot]
				var bone := mesh.skin.get_bind_bone(bind)
				var weight := weights[slot]
				var ancestor := bone
				while ancestor >= 0 and ancestor != head:
					ancestor = skeleton.get_bone_parent(ancestor)
				if ancestor == head:
					head_weight += weight
				var transform: Transform3D = skeleton.get_bone_global_pose(bone) * mesh.skin.get_bind_pose(bind)
				posed += (transform * vertices[index]) * weight
			if head_weight >= 0.9:
				var pixel := camera.unproject_position(mesh.global_transform * posed)
				minimum = minimum.min(pixel)
				maximum = maximum.max(pixel)
	return Rect2(minimum, maximum - minimum)

func measure_face(image: Image, bounds: Rect2) -> Dictionary:
	var visible := 0
	var total := 0.0
	# Central face area excludes shoulders, ring art and portrait status tint.
	var face := Rect2(bounds.position + bounds.size * Vector2(0.35, 0.35), bounds.size * Vector2(0.30, 0.35))
	face = face.intersection(Rect2(Vector2.ZERO, image.get_size()))
	for y in range(int(face.position.y), int(face.end.y)):
		for x in range(int(face.position.x), int(face.end.x)):
			var color := image.get_pixel(x, y)
			if color.a > 0.9:
				visible += 1
				total += (color.r + color.g + color.b) / 3.0
	return {"visible": visible, "mean": total / maxf(visible, 1)}

func settle() -> void:
	for frame in range(4):
		await process_frame
		await RenderingServer.frame_post_draw

func check(condition: bool, message: String) -> void:
	if not condition:
		push_error(message)
		failed = true
