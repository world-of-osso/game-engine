extends SceneTree

# Actual Zephras sky triplet, local Forever M2 7345733 / shader 0x8012.
const FDIDS := [8164017, 8164018, 8164019]
var source_images: Array[Image] = []
var material: ShaderMaterial

func _initialize() -> void:
	call_deferred("run")

func read_texture(fdid: int) -> Image:
	var bytes := FileAccess.get_file_as_bytes("res://../data/textures/%d.blp" % fdid)
	assert(bytes.slice(0, 4).get_string_from_ascii() == "BLP2")
	var width := bytes.decode_u32(12)
	var height := bytes.decode_u32(16)
	var offset := bytes.decode_u32(20)
	var size := bytes.decode_u32(84)
	return Image.create_from_data(width, height, false, Image.FORMAT_DXT5, bytes.slice(offset, offset + size))

func run() -> void:
	var world := Node3D.new()
	root.add_child(world)
	var camera := Camera3D.new()
	camera.position.z = 2.0
	camera.projection = Camera3D.PROJECTION_ORTHOGONAL
	camera.size = 2.0
	world.add_child(camera)
	camera.make_current()
	material = ShaderMaterial.new()
	material.shader = load("res://shaders/sky_m2.gdshader")
	material.set_shader_parameter("combine_mode", 0x8012)
	material.set_shader_parameter("has_second_texture", true)
	material.set_shader_parameter("has_third_texture", true)
	material.set_shader_parameter("zephras_dual_crossfade", true)
	for i in range(FDIDS.size()):
		var image := read_texture(FDIDS[i])
		material.set_shader_parameter(["base_texture", "second_texture", "third_texture"][i], ImageTexture.create_from_image(image))
		assert(image.decompress() == OK)
		source_images.append(image)
	var mesh := MeshInstance3D.new()
	mesh.mesh = QuadMesh.new()
	mesh.mesh.size = Vector2(2.0, 2.0)
	mesh.material_override = material
	world.add_child(mesh)
	var weights := [Vector3(1, 0, 0), Vector3(1, 1, 0), Vector3(1, 0, 1)]
	for i in range(weights.size()):
		material.set_shader_parameter("zephras_texture_weights", weights[i])
		for frame in range(10):
			await process_frame
			await RenderingServer.frame_post_draw
		var rendered := root.get_texture().get_image()
		var observed := rendered.get_pixel(rendered.get_width() / 2, rendered.get_height() / 2)
		var image := source_images[i]
		var expected := image.get_pixel(image.get_width() / 2, image.get_height() / 2)
		var error := maxf(absf(observed.r - expected.r), maxf(absf(observed.g - expected.g), absf(observed.b - expected.b)))
		print("ZEPHRAS_SKY_CROSSFADE endpoint=", i, " observed=", observed, " expected=", expected, " error=", error)
		if error > 0.04:
			quit(1)
			return
	print("ZEPHRAS_SKY_CROSSFADE PASS")
	quit(0)
