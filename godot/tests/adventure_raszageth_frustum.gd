extends "res://tests/adventure_zero_actor_display.gd"
# Read-only diagnostic: retain the source pose and its paints-zero product RED.
# Camera3D.get_frustum order is documented: near, far, left, top, right, bottom.
# https://docs.godotengine.org/en/latest/classes/class_camera3d.html#class-camera3d-method-get-frustum
const PLANE_NAMES := ["NEAR", "FAR", "LEFT", "TOP", "RIGHT", "BOTTOM"]

func run() -> void:
	completion = load("res://tests/completion.gd").new(self, 1)
	completion.finish(await run_frustum())

func vector_data(value: Vector3) -> Array:
	return [value.x, value.y, value.z]

func vector4_data(value: Vector4) -> Array:
	return [value.x, value.y, value.z, value.w]

func transform_data(value: Transform3D) -> Dictionary:
	return {"basis":[vector_data(value.basis.x),vector_data(value.basis.y),vector_data(value.basis.z)],"origin":vector_data(value.origin)}

func bounds_data(value: AABB) -> Dictionary:
	return {"position":vector_data(value.position),"size":vector_data(value.size),"end":vector_data(value.end)}

func collect_world_bounds(model: Node3D) -> Dictionary:
	var geometry := AABB()
	var cull := AABB()
	var first := true
	var meshes := []
	for mesh in model.find_children("*", "MeshInstance3D", true, false):
		if not mesh.is_visible_in_tree():
			continue
		var local_geometry: AABB = mesh.get_aabb()
		var custom: AABB = mesh.custom_aabb
		# The native custom_aabb contract uses all-zero to mean default bounds.
		var local_cull: AABB = local_geometry if custom == AABB() else custom
		var world_geometry: AABB = mesh.global_transform * local_geometry
		var world_cull: AABB = mesh.global_transform * local_cull.grow(mesh.extra_cull_margin)
		geometry = world_geometry if first else geometry.merge(world_geometry)
		cull = world_cull if first else cull.merge(world_cull)
		first = false
		meshes.append({"path":str(mesh.get_path()),"layers":mesh.layers,"local_geometry":bounds_data(local_geometry),"custom_aabb":bounds_data(custom),"extra_cull_margin":mesh.extra_cull_margin,"global_transform":transform_data(mesh.global_transform),"world_geometry":bounds_data(world_geometry),"candidate_world_cull":bounds_data(world_cull)})
	completion.require(not first, "Raszageth visible source meshes missing")
	return {"geometry":geometry,"candidate_cull":cull,"meshes":meshes}

func classify_planes(camera: Camera3D, bounds: AABB, probe: Vector3) -> Array:
	var planes := camera.get_frustum()
	if not completion.require(planes.size() == PLANE_NAMES.size(), "Native camera did not supply six frustum planes"):
		return []
	var out := []
	for index in planes.size():
		var plane: Plane = planes[index]
		var inside_distance := plane.distance_to(probe)
		if not completion.require(absf(inside_distance) > 0.000001, "Interior probe lies on a frustum plane"):
			return []
		var distances := []
		var all_outside := true
		for corner in 8:
			var distance := plane.distance_to(bounds.get_endpoint(corner))
			distances.append(distance)
			all_outside = all_outside and distance * signf(inside_distance) < -0.000001
		out.append({"name":PLANE_NAMES[index],"normal":vector_data(plane.normal),"d":plane.d,"interior_probe_distance":inside_distance,"corner_distances":distances,"whole_bounds_rejected":all_outside})
	return out

func verify_rendered_model(host: Control, model: Node3D) -> bool:
	var viewport := host.find_child("CharacterModelViewport", true, false) as SubViewport
	var camera := viewport.get_camera_3d() if viewport != null else null
	if not completion.require(camera != null, "Raszageth native viewport/current camera missing"):
		return false
	var before_camera: Transform3D = camera.global_transform
	var before_model := model.global_transform
	var collected := collect_world_bounds(model)
	if completion.failed:
		return false
	var bounds: AABB = collected.geometry
	var native_projection: Projection = camera.get_camera_projection()
	var effective_camera: Transform3D = camera.get_camera_transform()
	var probe: Vector3 = camera.project_position(Vector2(viewport.size) * 0.5, (camera.near + camera.far) * 0.5)
	var classified := classify_planes(camera, bounds, probe)
	var candidate_cull_planes := classify_planes(camera, collected.candidate_cull, probe)
	if completion.failed:
		return false
	var rejected := []
	for plane in classified:
		if plane.whole_bounds_rejected:
			rejected.append(plane.name)
	var projected_corners := []
	for index in 8:
		var point := bounds.get_endpoint(index)
		var screen: Vector2 = camera.unproject_position(point)
		projected_corners.append({"world":vector_data(point),"screen":[screen.x,screen.y],"inside_native_frustum":camera.is_position_in_frustum(point),"behind_camera":camera.is_position_behind(point)})
	await RenderingServer.frame_post_draw
	var image := viewport.get_texture().get_image()
	var painted := 0
	for y in image.get_height():
		for x in image.get_width():
			var pixel := image.get_pixel(x,y)
			if pixel.a > 0.5 and maxf(pixel.r,maxf(pixel.g,pixel.b)) > 0.08:
				painted += 1
	var result := {"gap":"JOURNAL_RASZAGETH_ZOOM05_Z5_PAINTS_ZERO","product_fixed":false,"model_transform":transform_data(model.global_transform),"camera_transform":transform_data(camera.global_transform),"effective_camera_transform":transform_data(effective_camera),"projection_mode":camera.projection,"keep_aspect":camera.keep_aspect,"fov":camera.fov,"near":camera.near,"far":camera.far,"h_offset":camera.h_offset,"v_offset":camera.v_offset,"frustum_offset":[camera.frustum_offset.x,camera.frustum_offset.y],"viewport_size":[viewport.size.x,viewport.size.y],"aspect":float(viewport.size.x)/float(viewport.size.y),"projection_columns":[vector4_data(native_projection.x),vector4_data(native_projection.y),vector4_data(native_projection.z),vector4_data(native_projection.w)],"interior_probe":vector_data(probe),"world_geometry_bounds":bounds_data(bounds),"candidate_world_cull_bounds":bounds_data(collected.candidate_cull),"geometry_planes":classified,"candidate_cull_planes":candidate_cull_planes,"rejecting_planes":rejected,"projected_corners":projected_corners,"meshes":collected.meshes,"painted_viewport_pixels":painted}
	print("JOURNAL_RASZAGETH_FRUSTUM ", JSON.stringify(result))
	var prefix := shots.path_join("%s-raszageth-frustum" % str(client.account_state().ui_skin).to_lower())
	var diagnostics := OS.get_environment("ADVENTURE_GUIDE_DIAGNOSTICS")
	if not completion.require(not diagnostics.is_empty(), "Explicit private diagnostics directory required"):
		return false
	var receipt := diagnostics.path_join("%s-raszageth-frustum.json" % str(client.account_state().ui_skin).to_lower())
	var file := FileAccess.open(receipt, FileAccess.WRITE)
	if not completion.require(file != null, "Raszageth frustum receipt open failed"):
		return false
	file.store_string(JSON.stringify(result, "\t"))
	file.close()
	if not completion.require(image.save_png(prefix + "-viewport.png") == OK, "Raszageth direct viewport capture save failed"):
		return false
	if not completion.require(camera.global_transform == before_camera and model.global_transform == before_model, "Read-only diagnostic changed authored camera/model transform"):
		return false
	if not completion.require(rejected.has("TOP") and not rejected.has("NEAR") and not rejected.has("FAR"), "Native rejection differs from TOP-only analytical hypothesis"):
		return false
	if not completion.require(painted == 0, "Native paints-zero gap was not reproduced"):
		return false
	print("JOURNAL_RASZAGETH_TOP_REJECTION_CONFIRMED product_gap_not_fixed")
	return true

func run_frustum() -> bool:
	var server := OS.get_environment("ADVENTURE_GUIDE_SERVER")
	var account := OS.get_environment("ADVENTURE_GUIDE_ACCOUNT")
	shots = OS.get_environment("ADVENTURE_GUIDE_SHOTS")
	if not completion.require(server == "127.0.0.1:5771" and account.begins_with("fb_") and not shots.is_empty(), "Explicit private5771/fb_* account and shots required"):
		return false
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var deadline := Time.get_ticks_msec() + 180000
	while client.account_state().assets_starting and Time.get_ticks_msec() < deadline:
		await process_frame
	if not completion.require(not client.account_state().assets_starting, "Asset startup timeout"):
		return false
	var error: String = client.connect_account(server, account, OS.get_environment("ADVENTURE_GUIDE_PASSWORD"), false)
	if not completion.require(error.is_empty(), error) or not completion.step(await enter_world_when_ready(), "genuine world readiness"):
		return false
	await journal_key()
	deadline = Time.get_ticks_msec() + 90000
	while not client.adventure_guide_state().loaded and Time.get_ticks_msec() < deadline:
		await process_frame
	if not completion.require(client.adventure_guide_state().loaded, "Journal source catalog not ready"):
		return false
	if not completion.require(client.account_state().ui_skin == OS.get_environment("ADVENTURE_GUIDE_EXPECTED_SKIN"), "Expected skin mismatch"):
		return false
	var row: Array = CASES[2]
	return await select_zero_actor(row) and await verify_scene(row[3],row[6],row[7],row[8])
