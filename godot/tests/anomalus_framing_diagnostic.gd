extends RefCounted

# Fixture-only: observe without changing camera, actor, skeleton, material or guard.
static func sample_surface(mesh: MeshInstance3D, surface: int, camera: Camera3D, viewport: Rect2, posed_mesh: Mesh = null) -> Dictionary:
	if posed_mesh == null:
		posed_mesh = mesh.bake_mesh_from_current_skeleton_pose() if mesh.skin != null else mesh.mesh
	var arrays: Array = posed_mesh.surface_get_arrays(surface)
	var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var indices: PackedInt32Array = arrays[Mesh.ARRAY_INDEX] if arrays[Mesh.ARRAY_INDEX] != null else PackedInt32Array()
	var count := vertices.size() if indices.is_empty() else indices.size()
	assert(count % 3 == 0, "diagnostic requires complete triangles")
	var triangles: Array[Dictionary] = []
	for first in range(0, count, 3):
		var projected: Array = []
		var referenced: Array = []
		var depths: Array = []
		var bounds := Rect2()
		var outside := 0
		var behind := 0
		for corner in range(3):
			var index: int = first + corner if indices.is_empty() else indices[first + corner]
			assert(index >= 0 and index < vertices.size(), "diagnostic index out of bounds")
			var world: Vector3 = mesh.global_transform * vertices[index]
			var point := camera.unproject_position(world)
			bounds = Rect2(point, Vector2.ZERO) if corner == 0 else bounds.expand(point)
			outside += int(not viewport.has_point(point))
			behind += int(camera.is_position_behind(world))
			referenced.append(index)
			projected.append([point.x, point.y])
			depths.append(-camera.to_local(world).z)
		triangles.append({"triangle": first / 3, "indices": referenced, "projected_vertices": projected, "camera_depths": depths, "projected_rect": rect_values(bounds), "outside_vertices": outside, "behind_vertices": behind})
	return {"surface": surface, "vertex_count": vertices.size(), "index_count": indices.size(), "triangle_count": triangles.size(), "triangles": triangles, "material": material_state(mesh.get_active_material(surface))}

static func rect_values(rect: Rect2) -> Array:
	return [rect.position.x, rect.position.y, rect.size.x, rect.size.y]

static func vector_values(value: Vector3) -> Array:
	return [value.x, value.y, value.z]

static func transform_values(value: Transform3D) -> Dictionary:
	return {"origin": vector_values(value.origin), "basis": [vector_values(value.basis.x), vector_values(value.basis.y), vector_values(value.basis.z)]}

static func serializable(value: Variant) -> Variant:
	if value is Color:
		return [value.r, value.g, value.b, value.a]
	if value is Vector2:
		return [value.x, value.y]
	if value is Vector3:
		return vector_values(value)
	if value is Basis:
		return [vector_values(value.x), vector_values(value.y), vector_values(value.z)]
	if value is Resource:
		return {"class": value.get_class(), "path": value.resource_path, "instance_id": value.get_instance_id()}
	if value == null or value is bool or value is int or value is float or value is String:
		return value
	return {"type": type_string(typeof(value)), "value": str(value)}

static func material_state(material: Material) -> Dictionary:
	if material == null:
		return {"class": "none"}
	var result := {"class": material.get_class(), "instance_id": material.get_instance_id(), "path": material.resource_path, "render_priority": material.render_priority}
	if material is BaseMaterial3D:
		result.merge({"albedo_color": serializable(material.albedo_color), "transparency": material.transparency, "cull_mode": material.cull_mode, "shading_mode": material.shading_mode, "alpha_scissor_threshold": material.alpha_scissor_threshold, "albedo_texture": serializable(material.albedo_texture)})
	if material is ShaderMaterial:
		result.shader_parameters = {}
		result.shader_code = material.shader.code if material.shader != null else ""
		if material.shader != null:
			for parameter in material.shader.get_shader_uniform_list():
				var name: String = parameter.name
				result.shader_parameters[name] = serializable(material.get_shader_parameter(name))
	result.next_pass = material_state(material.next_pass) if material.next_pass != null else {}
	return result

static func collect(scene: Node3D) -> Dictionary:
	var viewport := scene.get_viewport()
	var camera := viewport.get_camera_3d()
	assert(camera != null, "diagnostic requires the actual battle camera")
	var rect := viewport.get_visible_rect()
	var models: Array[Dictionary] = []
	for model in scene.get_children():
		var meshes: Array[Dictionary] = []
		for node in model.find_children("*", "MeshInstance3D", true, false):
			var mesh := node as MeshInstance3D
			var state := {"path": str(mesh.get_path()), "visible": mesh.is_visible_in_tree(), "transform": transform_values(mesh.global_transform), "transparency": mesh.transparency, "cast_shadow": mesh.cast_shadow, "skin": serializable(mesh.skin), "skeleton_path": str(mesh.skeleton), "surfaces": []}
			if mesh.mesh != null and mesh.is_visible_in_tree():
				var posed: Mesh = mesh.bake_mesh_from_current_skeleton_pose() if mesh.skin != null else mesh.mesh
				for surface in range(posed.get_surface_count()):
					assert(posed.surface_get_primitive_type(surface) == Mesh.PRIMITIVE_TRIANGLES, "diagnostic only accepts triangle surfaces")
					state.surfaces.append(sample_surface(mesh, surface, camera, rect, posed))
			meshes.append(state)
		var visual: Node3D = model.get_child(0)
		models.append({"name": str(model.name), "transform": transform_values(model.global_transform), "visual_transform": transform_values(visual.transform), "display_id": visual.get_meta("creature_display_id"), "pet_instance_scale": visual.get_meta("pet_instance_scale"), "meshes": meshes})
	var projection := camera.get_camera_projection()
	return {"schema": 1, "process_frame": Engine.get_process_frames(), "draw_frame": Engine.get_frames_drawn(), "camera": {"path": str(camera.get_path()), "transform": transform_values(camera.global_transform), "projection_type": camera.projection, "size": camera.size, "fov": camera.fov, "near": camera.near, "far": camera.far, "keep_aspect": camera.keep_aspect, "projection": [str(projection.x), str(projection.y), str(projection.z), str(projection.w)]}, "viewport": rect_values(rect), "models": models, "limits": "All indexed posed triangles, not proof that every projected fragment survives alpha/culling/shader displacement. Original framing guard unchanged."}

static func capture(root: Window, scene: Node3D, prefix: String) -> Dictionary:
	if scene == null:
		return {"error": "PetBattleActiveModels absent"}
	await RenderingServer.frame_post_draw
	# No await/state mutation between geometry and both image reads.
	var report := collect(scene)
	var viewport := scene.get_viewport()
	var image := viewport.get_texture().get_image()
	var screen := root.get_texture().get_image()
	report.viewport_host_rect = rect_values(viewport.get_parent().get_global_rect())
	report.images = []
	var size := image.get_size()
	var width := mini(64, size.x)
	var height := mini(64, size.y)
	var crops := {"left": Rect2i(0, 0, width, size.y), "right": Rect2i(size.x - width, 0, width, size.y), "top": Rect2i(0, 0, size.x, height), "bottom": Rect2i(0, size.y - height, size.x, height)}
	for edge in crops:
		var path: String = prefix + "-" + str(edge) + ".png"
		var error := image.get_region(crops[edge]).save_png(path)
		assert(error == OK, "failed diagnostic edge image: " + path)
		report.images.append({"edge": edge, "rect": rect_values(Rect2(crops[edge])), "path": path})
	assert(image.save_png(prefix + "-viewport.png") == OK)
	assert(screen.save_png(prefix + "-screen.png") == OK)
	report.screen_image = prefix + "-screen.png"
	report.viewport_image = prefix + "-viewport.png"
	var output := FileAccess.open(prefix + ".json", FileAccess.WRITE)
	assert(output != null, "failed diagnostic report file")
	output.store_string(JSON.stringify(report))
	output.close()
	return {"report": prefix + ".json", "process_frame": report.process_frame, "draw_frame": report.draw_frame, "images": report.images, "viewport_image": report.viewport_image, "screen_image": report.screen_image}
