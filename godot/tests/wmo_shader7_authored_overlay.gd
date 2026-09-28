extends RefCounted

const DIAGNOSTICS := "res://../data/diagnostics/godot-conversion/"
const COMPOSITE_PNG := DIAGNOSTICS + "wmo-shader7-authored-composite.png"
const RENDER_PNG := DIAGNOSTICS + "wmo-shader7-authored-render.png"
const EXPECTED := Color8(27, 32, 38, 255)
const TOLERANCE := 0.02
const WAIT_MS := 15000
const RENDER_SIZE := 64

func check(tree: SceneTree, client: Node) -> String:
	var material := await wait_for_authored_material(tree, client)
	if material == null:
		return "WMO 108238 group 38 lacks a streamed shader 7 material with authored 512x512 composite pixel"
	var texture := material.get_shader_parameter("base_texture") as ImageTexture
	if texture == null or texture.get_size() != Vector2(512, 512):
		return "WMO 108238 group 38 shader 7 lacks bound 512x512 composite"
	var composite := texture.get_image()
	if composite == null or composite.is_empty() or composite.get_pixel(0, 0) != EXPECTED:
		return "WMO 108238 group 38 shader 7 composite pixel (0,0) differs from " + str(EXPECTED)
	var error := composite.save_png(COMPOSITE_PNG)
	if error != OK:
		return "Could not save authored WMO composite: " + error_string(error)
	var rendered := await render_bound_material(tree, material)
	if rendered == null or rendered.is_empty():
		return "Authored WMO shader 7 GPU readback unavailable"
	error = rendered.save_png(RENDER_PNG)
	if error != OK:
		return "Could not save authored WMO render: " + error_string(error)
	var actual := rendered.get_pixel(RENDER_SIZE / 2, RENDER_SIZE / 2)
	if absf(actual.r - EXPECTED.r) > TOLERANCE or absf(actual.g - EXPECTED.g) > TOLERANCE or absf(actual.b - EXPECTED.b) > TOLERANCE:
		return "Authored WMO shader 7 rendered texel expected %s, got %s" % [EXPECTED, actual]
	print("PASS: authored WMO 108238 group 38 shader 7 composite and GPU texel ", actual, " screenshots=", COMPOSITE_PNG, ", ", RENDER_PNG)
	return ""

func wait_for_authored_material(tree: SceneTree, client: Node) -> ShaderMaterial:
	var deadline := Time.get_ticks_msec() + WAIT_MS
	var observed := {}
	while Time.get_ticks_msec() < deadline:
		var objects := client.get_node_or_null("WorldObjects")
		if objects != null:
			for child in objects.find_children("Group38_Batch*", "MeshInstance3D", true, false):
				if not child is MeshInstance3D or not child.name.begins_with("Group38_Batch"):
					continue
				var material := child.get_surface_override_material(0) as ShaderMaterial
				if material == null or material.get_shader_parameter("two_layer_shader") != 7:
					continue
				var texture := material.get_shader_parameter("base_texture") as ImageTexture
				if texture == null or texture.get_size() != Vector2(512, 512):
					continue
				var image := texture.get_image()
				if image != null and not image.is_empty() and not observed.has(child.get_instance_id()):
					observed[child.get_instance_id()] = true
					print("OVERLAY_CANDIDATE ", child.get_path(), " pixel=", image.get_pixel(0, 0), " expected=", EXPECTED)
				if image != null and not image.is_empty() and image.get_pixel(0, 0) == EXPECTED:
					return material
		await tree.process_frame
	var objects := client.get_node_or_null("WorldObjects")
	print("OVERLAY_TIMEOUT objects=", objects, " batches=", objects.find_children("Group38_Batch*", "MeshInstance3D", true, false).size() if objects != null else -1)
	return null

func render_bound_material(tree: SceneTree, authored: ShaderMaterial) -> Image:
	var viewport := SubViewport.new()
	viewport.size = Vector2i(RENDER_SIZE, RENDER_SIZE)
	viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	viewport.own_world_3d = true
	tree.root.add_child(viewport)
	var environment := Environment.new()
	environment.background_mode = Environment.BG_COLOR
	environment.background_color = Color.BLACK
	environment.ambient_light_source = Environment.AMBIENT_SOURCE_DISABLED
	environment.reflected_light_source = Environment.REFLECTION_SOURCE_DISABLED
	environment.tonemap_mode = Environment.TONE_MAPPER_LINEAR
	var world_environment := WorldEnvironment.new()
	world_environment.environment = environment
	viewport.add_child(world_environment)
	var camera := Camera3D.new()
	camera.projection = Camera3D.PROJECTION_ORTHOGONAL
	camera.size = 2.0
	camera.look_at_from_position(Vector3(0, 2, 0), Vector3.ZERO, Vector3(0, 0, -1))
	viewport.add_child(camera)
	camera.current = true
	var arrays := PlaneMesh.new().surface_get_arrays(0)
	var uvs := PackedVector2Array()
	for vertex in arrays[Mesh.ARRAY_VERTEX].size():
		uvs.append(Vector2(0.5 / 512.0, 0.5 / 512.0))
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	var quad := ArrayMesh.new()
	quad.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	var surface := MeshInstance3D.new()
	surface.mesh = quad
	var material := authored.duplicate() as ShaderMaterial
	material.set_shader_parameter("unlit", true)
	material.set_shader_parameter("unfogged", true)
	material.set_shader_parameter("emissive", Vector3.ZERO)
	material.set_shader_parameter("base_color", Color.WHITE)
	surface.material_override = material
	viewport.add_child(surface)
	for frame in 2:
		await RenderingServer.frame_post_draw
	var rendered := viewport.get_texture().get_image()
	viewport.free()
	return rendered
