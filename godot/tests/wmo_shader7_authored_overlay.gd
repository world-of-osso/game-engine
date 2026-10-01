extends RefCounted

# Streamed WMO 108238 group 38 material 58 is MOMT 7 (MapObjTwoLayerEnvMetal). Its
# layers stay separate GPU textures at their authored sizes (948125 512x512, 922678
# 128x128, no texture 3) and the shader blends them per WebWowViewerCpp
# caclWMOFragMat case 7: diffuse = mix(t1, t2, 1 - MOCV2.a). Unlit rendering of the
# bound material shows t1 at MOCV2.a = 1 and t2 at MOCV2.a = 0.

const DIAGNOSTICS := "res://../data/diagnostics/godot-conversion/"
const RENDER_PNG := DIAGNOSTICS + "wmo-shader7-authored-render.png"
const TOLERANCE := 0.02
const WAIT_MS := 120000
const RENDER_SIZE := 64

func check(tree: SceneTree, client: Node) -> String:
	var material := await wait_for_authored_material(tree, client)
	if material == null:
		return "WMO 108238 group 38 lacks a streamed MOMT 7 material with 512x512 and 128x128 layers"
	var base := texel(material, "base_texture", Vector2i.ZERO)
	var second := texel(material, "second_texture", Vector2i.ZERO)
	var first_render := await render_bound_material(tree, material, 1.0)
	var second_render := await render_bound_material(tree, material, 0.0)
	if first_render == null or second_render == null:
		return "Authored WMO shader 7 GPU readback unavailable"
	var error := first_render.save_png(RENDER_PNG)
	if error != OK:
		return "Could not save authored WMO render: " + error_string(error)
	for pair in [[first_render, base, "MOCV2.a=1 shows texture 1"], [second_render, second, "MOCV2.a=0 shows texture 2"]]:
		var actual: Color = pair[0].get_pixel(RENDER_SIZE / 2, RENDER_SIZE / 2)
		var expected: Color = pair[1]
		if absf(actual.r - expected.r) > TOLERANCE or absf(actual.g - expected.g) > TOLERANCE or absf(actual.b - expected.b) > TOLERANCE:
			return "Authored WMO shader 7 %s: expected %s, got %s" % [pair[2], expected, actual]
	print("PASS: authored WMO 108238 group 38 MOMT 7 layers ", base, " / ", second, " screenshot=", RENDER_PNG)
	return ""

func layer_size(material: ShaderMaterial, slot: String) -> Vector2:
	var texture := material.get_shader_parameter(slot) as Texture2D
	return texture.get_size() if texture != null else Vector2.ZERO

# Authored texel from the bound (possibly block-compressed) texture.
func texel(material: ShaderMaterial, slot: String, pixel: Vector2i) -> Color:
	var image := (material.get_shader_parameter(slot) as Texture2D).get_image()
	if image.is_compressed():
		image.decompress()
	return image.get_pixelv(pixel)

func wait_for_authored_material(tree: SceneTree, client: Node) -> ShaderMaterial:
	var deadline := Time.get_ticks_msec() + WAIT_MS
	var observed := {}
	while Time.get_ticks_msec() < deadline:
		var objects := client.get_node_or_null("WorldObjects")
		if objects != null:
			for child in objects.find_children("*", "MeshInstance3D", true, false):
				var mesh := child as MeshInstance3D
				var material := mesh.get_active_material(0) as ShaderMaterial
				if material == null or material.get_shader_parameter("pixel_shader") != 7:
					continue
				var sizes := [layer_size(material, "base_texture"), layer_size(material, "second_texture"), layer_size(material, "third_texture")]
				if not observed.has(mesh.get_instance_id()):
					observed[mesh.get_instance_id()] = true
					print("OVERLAY_CANDIDATE path=", mesh.get_path(), " sizes=", sizes)
				var authored_group := mesh.name.begins_with("Group38_Batch") and mesh.get_parent().name == "Wmo373730"
				if authored_group and sizes == [Vector2(512, 512), Vector2(128, 128), Vector2(1, 1)]:
					return material
		await tree.process_frame
	var objects := client.get_node_or_null("WorldObjects")
	print("OVERLAY_TIMEOUT objects=", objects, " descendants=", objects.find_children("*", "MeshInstance3D", true, false).size() if objects != null else -1, " shader7_candidates=", observed.size())
	return null

# The bound material on a quad whose UV/UV2 hit texel (0,0) of each layer, unlit and
# unfogged, with the given second MOCV alpha.
func render_bound_material(tree: SceneTree, authored: ShaderMaterial, second_alpha: float) -> Image:
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
	var uv2s := PackedVector2Array()
	var alphas := PackedFloat32Array()
	for vertex in arrays[Mesh.ARRAY_VERTEX].size():
		uvs.append(Vector2(0.5 / 512.0, 0.5 / 512.0))
		uv2s.append(Vector2(0.5 / 128.0, 0.5 / 128.0))
		alphas.append(second_alpha)
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	arrays[Mesh.ARRAY_TEX_UV2] = uv2s
	arrays[Mesh.ARRAY_CUSTOM2] = alphas
	var quad := ArrayMesh.new()
	quad.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays, [], {}, Mesh.ARRAY_CUSTOM_R_FLOAT << Mesh.ARRAY_FORMAT_CUSTOM2_SHIFT)
	var surface := MeshInstance3D.new()
	surface.mesh = quad
	var material := authored.duplicate() as ShaderMaterial
	material.set_shader_parameter("unlit", true)
	material.set_shader_parameter("unfogged", true)
	material.set_shader_parameter("emissive", Vector3.ZERO)
	material.set_shader_parameter("base_color", Color.WHITE)
	material.set_shader_parameter("has_uv2", true)
	material.set_shader_parameter("has_second_mocv", true)
	surface.material_override = material
	viewport.add_child(surface)
	for frame in 2:
		await RenderingServer.frame_post_draw
	var rendered := viewport.get_texture().get_image()
	viewport.free()
	return rendered
