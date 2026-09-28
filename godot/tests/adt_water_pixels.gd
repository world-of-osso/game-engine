extends RefCounted

const TILE_PATH := "WorldTerrain/Tile32_48"
const SAMPLE := Vector2(-8558.0, 500.0)
const AUTHORED_HEIGHT := 143.98892
const VIEW_SIZE := 96
const VIEW_WIDTH := 12.0
const MIN_CHANGED_PIXELS := 64
const MIN_BACKING_RESPONSE := 0.05
const MAX_BACKING_RESPONSE := 0.95
const DIAGNOSTICS := "res://../data/diagnostics/godot-conversion/"

func check(flow: SceneTree, client: Node, output_filename: String = "swimming-water-isolated.png") -> String:
	var tile := client.get_node_or_null(TILE_PATH)
	if tile == null:
		return "Missing native ADT tile for rendered water: " + TILE_PATH
	var water := tile.get_node_or_null("Water")
	if water == null:
		return "Missing rendered ADT water: " + TILE_PATH + "/Water"
	var meshes: Array[MeshInstance3D] = []
	collect_meshes(water, meshes)
	var covering: Array[MeshInstance3D] = []
	for instance in meshes:
		if covers_authored_sample(instance):
			covering.append(instance)
	if covering.is_empty():
		return "Rendered ADT water has no triangle covering x=-8558 z=500 at authored y=143.98892"
	var ground = client.terrain_height_at(SAMPLE.x, SAMPLE.y)
	if ground == null or AUTHORED_HEIGHT - float(ground) < 3.0:
		return "Water render sample has no deep authored ground beneath it"
	if DisplayServer.get_name() == "headless":
		return "Rendered ADT water pixel assertion requires a display"
	var clock := flow.root.get_node("M2MaterialClock")
	var clock_processing := clock.is_processing()
	clock.set_process(false)
	var viewport := make_viewport(flow, float(ground))
	var water_root := Node3D.new()
	viewport.add_child(water_root)
	for source in covering:
		var clone := MeshInstance3D.new()
		clone.mesh = source.mesh
		clone.transform = source.global_transform
		for surface in source.mesh.get_surface_count():
			var override_material := source.get_surface_override_material(surface)
			if override_material != null:
				clone.set_surface_override_material(surface, override_material)
		water_root.add_child(clone)
	var backing := viewport.get_node("Backing") as MeshInstance3D
	var material := backing.material_override as StandardMaterial3D
	material.albedo_color = Color(0.8, 0.1, 0.1)
	water_root.visible = false
	var hidden_red := await capture(flow, viewport)
	water_root.visible = true
	var visible_red := await capture(flow, viewport)
	material.albedo_color = Color(0.1, 0.1, 0.8)
	var visible_blue := await capture(flow, viewport)
	water_root.visible = false
	var hidden_blue := await capture(flow, viewport)
	var error := evaluate_pixels(hidden_red, visible_red, visible_blue, hidden_blue)
	if error == "":
		water_root.visible = true
		var frozen := await capture(flow, viewport)
		var still_frozen := await capture(flow, viewport)
		if changed_pixels(frozen, still_frozen, 0.01) > 0:
			error = "Water animation advanced while shared material clock was stopped"
		clock.advance_time_ms(2000.0)
		var advanced := await capture(flow, viewport)
		if error == "" and changed_pixels(still_frozen, advanced, 0.003) < MIN_CHANGED_PIXELS:
			error = "Water normals did not animate after advancing shared clock by two seconds"
	clock.set_process(clock_processing)
	if error == "":
		if visible_red.save_png(DIAGNOSTICS + output_filename) != OK:
			error = "Could not save isolated ADT water screenshot"
	viewport.queue_free()
	return error

func collect_meshes(node: Node, output: Array[MeshInstance3D]) -> void:
	if node is MeshInstance3D:
		output.append(node)
	for child in node.get_children():
		collect_meshes(child, output)

func covers_authored_sample(instance: MeshInstance3D) -> bool:
	if instance.mesh == null:
		return false
	for surface in instance.mesh.get_surface_count():
		if instance.mesh.surface_get_primitive_type(surface) != Mesh.PRIMITIVE_TRIANGLES:
			continue
		var arrays := instance.mesh.surface_get_arrays(surface)
		var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
		var indices: PackedInt32Array = arrays[Mesh.ARRAY_INDEX]
		var count := indices.size() if not indices.is_empty() else vertices.size()
		for index in range(0, count - 2, 3):
			var a: Vector3 = instance.global_transform * vertices[indices[index] if not indices.is_empty() else index]
			var b: Vector3 = instance.global_transform * vertices[indices[index + 1] if not indices.is_empty() else index + 1]
			var c: Vector3 = instance.global_transform * vertices[indices[index + 2] if not indices.is_empty() else index + 2]
			var height := triangle_height_at_sample(a, b, c)
			if not is_nan(height) and absf(height - AUTHORED_HEIGHT) < 0.2:
				return true
	return false

func triangle_height_at_sample(a: Vector3, b: Vector3, c: Vector3) -> float:
	var ab := Vector2(b.x - a.x, b.z - a.z)
	var ac := Vector2(c.x - a.x, c.z - a.z)
	var point := SAMPLE - Vector2(a.x, a.z)
	var determinant := ab.cross(ac)
	if absf(determinant) < 0.00001:
		return NAN
	var u := point.cross(ac) / determinant
	var v := ab.cross(point) / determinant
	if u < -0.0001 or v < -0.0001 or u + v > 1.0001:
		return NAN
	return a.y + u * (b.y - a.y) + v * (c.y - a.y)

func make_viewport(flow: SceneTree, ground_height: float) -> SubViewport:
	var viewport := SubViewport.new()
	viewport.size = Vector2i(VIEW_SIZE, VIEW_SIZE)
	viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	viewport.own_world_3d = true
	flow.root.add_child(viewport)
	var environment := Environment.new()
	environment.background_mode = Environment.BG_COLOR
	environment.background_color = Color.BLACK
	environment.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	environment.ambient_light_color = Color.WHITE
	environment.ambient_light_energy = 1.0
	var world_environment := WorldEnvironment.new()
	world_environment.environment = environment
	viewport.add_child(world_environment)
	var camera := Camera3D.new()
	camera.projection = Camera3D.PROJECTION_ORTHOGONAL
	camera.size = VIEW_WIDTH
	camera.far = 100.0
	camera.position = Vector3(SAMPLE.x, AUTHORED_HEIGHT + 16.0, SAMPLE.y)
	camera.rotation.x = -PI / 2.0
	viewport.add_child(camera)
	camera.current = true
	var backing := MeshInstance3D.new()
	backing.name = "Backing"
	var plane := PlaneMesh.new()
	plane.size = Vector2(VIEW_WIDTH * 2.0, VIEW_WIDTH * 2.0)
	backing.mesh = plane
	backing.position = Vector3(SAMPLE.x, ground_height, SAMPLE.y)
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	backing.material_override = material
	viewport.add_child(backing)
	return viewport

func capture(flow: SceneTree, viewport: SubViewport) -> Image:
	await flow.process_frame
	await RenderingServer.frame_post_draw
	await flow.process_frame
	await RenderingServer.frame_post_draw
	return viewport.get_texture().get_image()

func evaluate_pixels(hidden_red: Image, visible_red: Image, visible_blue: Image, hidden_blue: Image) -> String:
	for image in [hidden_red, visible_red, visible_blue, hidden_blue]:
		if image == null or image.is_empty():
			return "ADT water isolated GPU readback unavailable"
	var changed := 0
	var translucent := 0
	for y in VIEW_SIZE:
		for x in VIEW_SIZE:
			var pixel := Vector2i(x, y)
			var baseline := hidden_red.get_pixelv(pixel)
			var shown := visible_red.get_pixelv(pixel)
			if color_distance(baseline, shown) < 0.08:
				continue
			changed += 1
			var baseline_difference := color_distance(baseline, hidden_blue.get_pixelv(pixel))
			if baseline_difference < 0.2:
				continue
			var backing_response := color_distance(shown, visible_blue.get_pixelv(pixel)) / baseline_difference
			if backing_response > MIN_BACKING_RESPONSE and backing_response < MAX_BACKING_RESPONSE:
				translucent += 1
	if changed < MIN_CHANGED_PIXELS:
		return "Rendered ADT water changed only %d isolated GPU pixels (need %d)" % [changed, MIN_CHANGED_PIXELS]
	if translucent < MIN_CHANGED_PIXELS:
		return "Rendered ADT water has only %d translucent GPU pixels (need %d)" % [translucent, MIN_CHANGED_PIXELS]
	print("PASS: ADT_WATER_PIXELS changed=", changed, " translucent=", translucent)
	return ""

func changed_pixels(first: Image, second: Image, threshold: float) -> int:
	var changed := 0
	for y in VIEW_SIZE:
		for x in VIEW_SIZE:
			if color_distance(first.get_pixel(x, y), second.get_pixel(x, y)) > threshold:
				changed += 1
	return changed

func color_distance(a: Color, b: Color) -> float:
	return absf(a.r - b.r) + absf(a.g - b.g) + absf(a.b - b.b)
