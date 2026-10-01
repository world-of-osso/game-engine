extends SceneTree

# Read-only production observer. MAIN runs each original fixed-time invocation.
# No renderer oracle, hide control, animation seek, or acceptance assertions.
# godot --path godot -s res://tests/skybox_render_state.gd -- --screen skyboxdebug
#   --skybox-fdid 5412968 --skybox-verify --skybox-time-ms 0
# Repeat with 100000 or the cloud's first opaque key at 200000; coastal uses FDID 525142.
const SOURCE_META := "m2_source_path"
const WAIT_MS := 30000
const SAMPLE_GRID := 4

var client: Node
var sky: Node3D
var camera: Camera3D
var failed: bool = false
var shots := ""
var time_arg := ""
var fdid_arg := ""
var texture_observations: Dictionary = {}
var emitted_shaders: Dictionary = {}

func _initialize() -> void:
	call_deferred("observe_production")

func observe_production() -> void:
	create_timer(float(WAIT_MS) / 1000.0).timeout.connect(on_timeout)
	root.size = Vector2i(1280, 720)
	if not validate_inputs():
		return
	if not mount_client():
		return
	if not await discover_render_state():
		return
	await RenderingServer.frame_post_draw
	if failed:
		return
	emit("scene", {
		"argv": Array(OS.get_cmdline_user_args()),
		"source": str(sky.get_meta(SOURCE_META)),
		"sky_path": str(sky.get_path()),
		"sky_transform": transform_data(sky.global_transform),
		"camera": camera_data(),
		"limit": "Observational data only; no feature test, root-cause conclusion or acceptance."
	})
	if not observe_batches():
		return
	if not save_viewport():
		return
	quit(0)

func validate_inputs() -> bool:
	shots = OS.get_environment("GODOT_SKYBOX_SCREENSHOTS")
	if shots.is_empty() or not DirAccess.dir_exists_absolute(shots):
		return reject("Requires existing GODOT_SKYBOX_SCREENSHOTS directory")
	var args := OS.get_cmdline_user_args()
	if first_value(args, "--screen") != "skyboxdebug" or not args.has("--skybox-verify"):
		return reject("Requires original --screen skyboxdebug --skybox-verify")
	fdid_arg = first_value(args, "--skybox-fdid")
	time_arg = first_value(args, "--skybox-time-ms")
	if not fdid_arg in ["5412968", "525142"]:
		return reject("Bounded sources require --skybox-fdid 5412968 or coastal 525142")
	if not time_arg in ["0", "100000", "200000"]:
		return reject("Requires --skybox-time-ms 0, 100000 or 200000")
	if args.has("--light-skybox-id"):
		return reject("Conflicting --light-skybox-id with forced FDID")
	return true

func first_value(args: PackedStringArray, flag: String) -> String:
	var index := args.find(flag)
	if index < 0 or index + 1 >= args.size():
		return ""
	return args[index + 1]

func mount_client() -> bool:
	if not ClassDB.class_exists("GameClient"):
		return reject("GameClient native class absent; inspect extension load errors")
	var packed := load("res://scenes/client.tscn") as PackedScene
	if packed == null:
		return reject("Cannot load production client.tscn")
	client = packed.instantiate()
	if client == null:
		return reject("Cannot instantiate production client.tscn")
	if not client.is_class("GameClient"):
		client.free()
		return reject("Production scene root is not GameClient")
	root.add_child(client)
	return true

func discover_render_state() -> bool:
	var deadline := Time.get_ticks_msec() + WAIT_MS
	while Time.get_ticks_msec() < deadline and not failed:
		await process_frame
		if not is_instance_valid(client) or client.is_queued_for_deletion():
			return reject("Production client disappeared during startup")
		var sources: Array[Node3D] = []
		for node in client.find_children("*", "Node3D", true, false):
			if node.has_meta(SOURCE_META):
				sources.append(node as Node3D)
		if sources.size() > 1:
			return reject("Ambiguous production source metadata: %d roots" % sources.size())
		if sources.is_empty():
			continue
		sky = sources[0]
		camera = root.get_camera_3d()
		if camera == null:
			continue
		var cameras := client.find_children("*", "Camera3D", true, false)
		if not cameras.has(camera):
			return reject("Active viewport camera is not a production Camera3D")
		var source := str(sky.get_meta(SOURCE_META))
		if source.is_empty() or not FileAccess.file_exists(source):
			return reject("Production M2 source absent: " + source)
		if sky.find_children("*", "MeshInstance3D", true, false).is_empty():
			continue
		return true
	return reject("30s startup bound: source metadata, actual sky mesh or active camera absent")

func observe_batches() -> bool:
	for node in sky.find_children("*", "MeshInstance3D", true, false):
		var batch := node as MeshInstance3D
		if batch.mesh == null or batch.mesh.get_surface_count() == 0:
			return reject("Missing batch mesh/surfaces: " + str(batch.get_path()))
		for surface in batch.mesh.get_surface_count():
			if not observe_surface(batch, surface):
				return false
	return true

func observe_surface(batch: MeshInstance3D, surface: int) -> bool:
	var material := batch.get_active_material(surface) as ShaderMaterial
	if material == null or material.shader == null:
		return reject("Missing actual ShaderMaterial/shader: " + str(batch.get_path()))
	var code := material.shader.code
	if code.is_empty():
		return reject("Actual shader code absent: " + str(batch.get_path()))
	var arrays := batch.mesh.surface_get_arrays(surface)
	if arrays.size() <= Mesh.ARRAY_VERTEX or not arrays[Mesh.ARRAY_VERTEX] is PackedVector3Array:
		return reject("Missing Godot vertex array: " + str(batch.get_path()))
	var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	if vertices.is_empty():
		return reject("Empty Godot vertex array: " + str(batch.get_path()))
	var stages := texture_stages(material)
	if failed:
		return false
	var shader_key := code.sha256_text()
	if not emitted_shaders.has(shader_key):
		emit("shader", {"sha256": shader_key, "resource": resource_data(material.shader), "code": code})
		emitted_shaders[shader_key] = true
	emit("batch", {
		"path": str(batch.get_path()), "surface": surface,
		"source": str(sky.get_meta(SOURCE_META)), "visible": batch.is_visible_in_tree(),
		"layers": batch.layers, "camera_cull_mask": camera.cull_mask,
		"local_aabb": aabb_data(batch.get_aabb()),
		"global_aabb": aabb_data(batch.global_transform * batch.get_aabb()),
		"vertices": vertex_bounds(vertices),
		"uv_bounds": uv_bounds(arrays[Mesh.ARRAY_TEX_UV]),
		"uv2_bounds": uv_bounds(arrays[Mesh.ARRAY_TEX_UV2]),
		"local_transform": transform_data(batch.transform),
		"global_transform": transform_data(batch.global_transform),
		"material": resource_data(material), "render_priority": material.render_priority,
		"shader_sha256": shader_key, "render_modes": render_modes(code),
		"parameters": material_parameters(material), "texture_stages": stages,
		"skeleton": skeleton_data(batch), "camera": camera_data()
	})
	return not failed

func material_parameters(material: ShaderMaterial) -> Dictionary:
	var result: Dictionary = {}
	for uniform in material.shader.get_shader_uniform_list():
		var name := str(uniform["name"])
		var value: Variant = material.get_shader_parameter(name)
		if value == null:
			value = RenderingServer.shader_get_parameter_default(material.shader.get_rid(), name)
		if not value is Texture:
			result[name] = json_value(value)
	return result

func texture_stages(material: ShaderMaterial) -> Array:
	var result: Array = []
	for name in ["base_texture", "second_texture", "third_texture", "fourth_texture"]:
		var texture := material.get_shader_parameter(name) as Texture2D
		if texture == null:
			reject("Missing resolved texture stage: " + name)
			return []
		var key := texture.get_instance_id()
		if not texture_observations.has(key):
			var image := texture.get_image()
			if image == null or image.is_empty():
				reject("Missing decoded Image for stage " + name)
				return []
			var original_format := image.get_format()
			# get_image returns a readback, never mutate the production texture.
			if image.is_compressed():
				var error := image.decompress()
				if error != OK:
					reject("Image decompression failed for %s: %s" % [name, error_string(error)])
					return []
			texture_observations[key] = {
				"resource": resource_data(texture), "width": image.get_width(),
				"height": image.get_height(), "original_format": original_format,
				"decoded_format": image.get_format(), "mipmaps": image.has_mipmaps(),
				"rgba_samples": sample_image(image),
				"path_note": "Empty path/metadata: no exposed disk provenance; no inferred FDID."
			}
		result.append({"uniform": name, "resolved": texture_observations[key]})
	return result

func sample_image(image: Image) -> Array:
	var result: Array = []
	for row in SAMPLE_GRID:
		for column in SAMPLE_GRID:
			var x := int(column * (image.get_width() - 1) / float(SAMPLE_GRID - 1))
			var y := int(row * (image.get_height() - 1) / float(SAMPLE_GRID - 1))
			result.append({"xy": [x, y], "rgba": color_data(image.get_pixel(x, y))})
	return result

func uv_bounds(values: PackedVector2Array) -> Dictionary:
	var low := Vector2(INF, INF)
	var high := Vector2(-INF, -INF)
	var finite_count := 0
	for value in values:
		if value.is_finite():
			low = low.min(value)
			high = high.max(value)
			finite_count += 1
	return {"count": values.size(), "finite_count": finite_count,
		"min": [low.x, low.y], "max": [high.x, high.y]}

func vertex_bounds(vertices: PackedVector3Array) -> Dictionary:
	var low := Vector3(INF, INF, INF)
	var high := Vector3(-INF, -INF, -INF)
	var finite_count := 0
	for vertex in vertices:
		if vertex.is_finite():
			low = low.min(vertex)
			high = high.max(vertex)
			finite_count += 1
	return {
		"count": vertices.size(), "finite_count": finite_count,
		"nonfinite_count": vertices.size() - finite_count,
		"finite_min": vector_data(low) if finite_count > 0 else null,
		"finite_max": vector_data(high) if finite_count > 0 else null
	}

func skeleton_data(batch: MeshInstance3D) -> Dictionary:
	var path := batch.skeleton
	var skeleton := batch.get_node_or_null(path) as Skeleton3D
	if skeleton == null or skeleton.get_bone_count() == 0:
		reject("Missing skeleton/bone0 for real batch: " + str(batch.get_path()))
		return {}
	var parent := skeleton.get_bone_parent(0)
	var result := {
		"path": str(skeleton.get_path()), "batch_skeleton_path": str(path),
		"bone_count": skeleton.get_bone_count(), "bone0_name": skeleton.get_bone_name(0),
		"bone0_parent": parent, "bone0_pose": transform_data(skeleton.get_bone_pose(0)),
		"bone0_global_pose": transform_data(skeleton.get_bone_global_pose(0)),
		"bone0_rest": transform_data(skeleton.get_bone_rest(0)),
		"global_transform": transform_data(skeleton.global_transform)
	}
	if parent >= 0:
		result["parent_pose"] = transform_data(skeleton.get_bone_pose(parent))
	return result

func camera_data() -> Dictionary:
	return {
		"path": str(camera.get_path()), "transform": transform_data(camera.global_transform),
		"fov": camera.fov, "near": camera.near, "far": camera.far,
		"projection": camera.projection, "keep_aspect": camera.keep_aspect,
		"viewport_extent": [root.size.x, root.size.y],
		"msaa_3d": root.msaa_3d, "msaa_2d": root.msaa_2d,
		"scaling_3d_scale": root.scaling_3d_scale, "use_taa": root.use_taa,
		"screen_space_aa": root.screen_space_aa,
		"vsync_mode": DisplayServer.window_get_vsync_mode()
	}

func render_modes(code: String) -> Array:
	var result: Array = []
	for line in code.split("\n"):
		if line.strip_edges().begins_with("render_mode "):
			result.append(line.strip_edges())
	return result

func resource_data(resource: Resource) -> Dictionary:
	var metadata: Dictionary = {}
	for key in resource.get_meta_list():
		metadata[str(key)] = json_value(resource.get_meta(key))
	return {
		"class": resource.get_class(), "instance_id": resource.get_instance_id(),
		"resource_path": resource.resource_path, "resource_name": resource.resource_name,
		"metadata": metadata
	}

func json_value(value: Variant) -> Variant:
	if value is Vector2:
		return [value.x, value.y]
	if value is Vector3:
		return vector_data(value)
	if value is Color:
		return color_data(value)
	if value == null or value is bool or value is int or value is float or value is String:
		return value
	return str(value)

func vector_data(value: Vector3) -> Array:
	return [value.x, value.y, value.z]

func color_data(value: Color) -> Array:
	return [value.r, value.g, value.b, value.a]

func transform_data(value: Transform3D) -> Dictionary:
	return {"origin": vector_data(value.origin), "basis_columns": [
		vector_data(value.basis.x), vector_data(value.basis.y), vector_data(value.basis.z)
	]}

func aabb_data(value: AABB) -> Dictionary:
	return {"position": vector_data(value.position), "size": vector_data(value.size)}

func save_viewport() -> bool:
	var image := root.get_texture().get_image()
	if image == null or image.is_empty():
		return reject("Viewport Image readback absent")
	if image.get_size() != Vector2i(1280, 720):
		return reject("Viewport readback is not requested 1280x720")
	var path := shots.path_join("skybox-render-state-%s-%s.png" % [fdid_arg, time_arg])
	var error := image.save_png(path)
	if error != OK:
		return reject("Cannot save lossless viewport PNG %s: %s" % [path, error_string(error)])
	emit("capture", {"png": path, "width": image.get_width(), "height": image.get_height()})
	return true

func emit(kind: String, data: Dictionary) -> void:
	print("SKYBOX_RENDER_STATE ", JSON.stringify({"kind": kind, "data": data}))

func reject(message: String) -> bool:
	failed = true
	emit("error", {"message": message})
	push_error("SKYBOX_RENDER_STATE: " + message)
	quit(1)
	return false

func on_timeout() -> void:
	if not failed:
		reject("30s diagnostic deadline exceeded; source/material/camera/image readiness incomplete")
