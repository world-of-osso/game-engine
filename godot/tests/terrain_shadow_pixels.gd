extends "res://tests/terrain_material_pixels.gd"

const SHADOW_PIXEL := Vector2i(32, 32)
const LIT_PIXEL := Vector2i(8, 32)

func sample_at(pixel: Vector2i) -> Color:
	await process_frame
	await RenderingServer.frame_post_draw
	await process_frame
	await RenderingServer.frame_post_draw
	var image := viewport.get_texture().get_image()
	if image == null or image.is_empty():
		fail("GPU pixel readback unavailable")
		return Color.BLACK
	return image.get_pixelv(pixel)

func assert_at(label: String, pixel: Vector2i, expected: Color) -> bool:
	var actual := await sample_at(pixel)
	if absf(actual.r - expected.r) > TOLERANCE or absf(actual.g - expected.g) > TOLERANCE or absf(actual.b - expected.b) > TOLERANCE:
		fail("%s: expected %s, got %s" % [label, expected, actual])
		return false
	print("PASS: ", label, " ", actual)
	return true

func _run() -> void:
	if not ResourceLoader.exists(SHADER_PATH):
		fail("Terrain shader missing: " + SHADER_PATH)
		return
	material = ShaderMaterial.new()
	material.shader = load(SHADER_PATH)
	fixture_scene()
	set_base_inputs()

	var camera := viewport.get_camera_3d()
	camera.near = 0.05
	camera.far = 5.0
	var sun := viewport.get_child(2) as DirectionalLight3D
	sun.shadow_enabled = true
	sun.directional_shadow_mode = DirectionalLight3D.SHADOW_PARALLEL_4_SPLITS
	sun.directional_shadow_split_1 = 0.25
	sun.directional_shadow_split_2 = 0.5
	sun.directional_shadow_split_3 = 0.75
	sun.directional_shadow_max_distance = 5.0

	material.set_shader_parameter("config", Vector4(1.0, 0.0, 1.0, 0.0))
	material.set_shader_parameter("ground_0", solid_texture(Color(0.25, 0.25, 0.25, 0.5)))
	material.set_shader_parameter("ambient", Vector3(0.25, 0.25, 0.25))
	material.set_shader_parameter("horizon_ambient", Vector3(0.25, 0.25, 0.25))
	material.set_shader_parameter("ground_ambient", Vector3(0.25, 0.25, 0.25))
	material.set_shader_parameter("direct", Vector3(0.25, 0.25, 0.25))
	var diffuse := 0.25 * (0.5 * 255.0 / 127.0)
	var ambient := diffuse * (0.25 * 1.1)
	var direct := diffuse * 0.25
	var specular := 0.5 * 0.25
	var lit := Color(ambient + direct + specular, ambient + direct + specular, ambient + direct + specular)
	var shadowed := Color(ambient, ambient, ambient)
	if not await assert_at("terrain lit before caster", SHADOW_PIXEL, lit):
		return
	var outside_before := await sample_at(LIT_PIXEL)

	var control := StandardMaterial3D.new()
	control.albedo_color = Color.WHITE
	control.roughness = 1.0
	terrain_instance.material_override = control
	var box := MeshInstance3D.new()
	var box_mesh := BoxMesh.new()
	box_mesh.size = Vector3(0.4, 0.6, 0.4)
	box.mesh = box_mesh
	box.position.y = 0.7
	box.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_SHADOWS_ONLY
	viewport.add_child(box)
	if not await assert_at("control outside shadow", LIT_PIXEL, Color.WHITE):
		return
	if not await assert_at("control cast shadow", SHADOW_PIXEL, Color.BLACK):
		return

	terrain_instance.material_override = material
	if not await assert_at("terrain outside shadow unchanged", LIT_PIXEL, outside_before):
		return
	if not await assert_at("terrain ambient survives shadow; direct and specular removed", SHADOW_PIXEL, shadowed):
		return
	box.queue_free()
	if not await assert_at("terrain direct and specular return after caster removal", SHADOW_PIXEL, lit):
		return
	quit(0)
