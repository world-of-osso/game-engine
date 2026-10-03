extends SceneTree

const FIXTURE := "user://m2_loader_pixels_fixture"
const VIEW_SIZE := 64
const TOLERANCE := 0.025
const BASE := Color(102.0 / 255.0, 76.0 / 255.0, 51.0 / 255.0)
const SECOND := Color(51.0 / 255.0, 128.0 / 255.0, 178.0 / 255.0)

var viewport: SubViewport
var loaded: Node3D
var loader: Object

func _initialize() -> void:
	call_deferred("run_cases")

func put_u16(bytes: PackedByteArray, offset: int, value: int) -> void:
	bytes.encode_u16(offset, value)

func put_u32(bytes: PackedByteArray, offset: int, value: int) -> void:
	bytes.encode_u32(offset, value)

func put_float(bytes: PackedByteArray, offset: int, value: float) -> void:
	bytes.encode_float(offset, value)

func put_magic(bytes: PackedByteArray, offset: int, magic: String) -> void:
	for index in magic.length():
		bytes[offset + index] = magic.unicode_at(index)

func chunk(magic: String, payload: PackedByteArray) -> PackedByteArray:
	var bytes := PackedByteArray()
	bytes.resize(8)
	put_magic(bytes, 0, magic)
	put_u32(bytes, 4, payload.size())
	bytes.append_array(payload)
	return bytes

func make_m2(flags: int, blend_mode: int) -> PackedByteArray:
	var md20 := PackedByteArray()
	md20.resize(0x240)
	put_magic(md20, 0, "MD20")
	put_u32(md20, 4, 274) # Retail MD20 version; CASC M2s are 272 or 274.
	put_u32(md20, 0x10, 8) # Authored texture-combiner IDs.
	put_u32(md20, 0x3c, 4)
	put_u32(md20, 0x40, 0x140)
	put_u32(md20, 0x50, 2)
	put_u32(md20, 0x54, 0x210)
	put_u32(md20, 0x70, 1)
	put_u32(md20, 0x74, 0x200)
	put_u32(md20, 0x80, 2)
	put_u32(md20, 0x84, 0x230)
	var positions := [Vector2(-0.8, -0.8), Vector2(-0.8, 0.8), Vector2(0.8, 0.8), Vector2(0.8, -0.8)]
	for index in 4:
		var offset := 0x140 + index * 48
		put_float(md20, offset, positions[index].x)
		put_float(md20, offset + 4, positions[index].y)
		put_float(md20, offset + 28, 1.0) # WoW +Z becomes Godot +Y.
		put_float(md20, offset + 32, 0.5)
		put_float(md20, offset + 36, 0.5)
	put_u16(md20, 0x200, flags)
	put_u16(md20, 0x202, blend_mode)
	put_u16(md20, 0x230, 0)
	put_u16(md20, 0x232, 1)
	var txid := PackedByteArray()
	txid.resize(8)
	put_u32(txid, 0, 910001)
	put_u32(txid, 4, 910002)
	var model := chunk("MD21", md20)
	model.append_array(chunk("TXID", txid))
	return model

func make_skin(shader_id: int, texture_count: int) -> PackedByteArray:
	var skin := PackedByteArray()
	skin.resize(44 + 8 + 12 + 48 + 24)
	put_magic(skin, 0, "SKIN")
	put_u32(skin, 4, 4)
	put_u32(skin, 8, 44)
	put_u32(skin, 12, 6)
	put_u32(skin, 16, 52)
	put_u32(skin, 28, 1)
	put_u32(skin, 32, 64)
	put_u32(skin, 36, 1)
	put_u32(skin, 40, 112)
	for index in 4:
		put_u16(skin, 44 + index * 2, index)
	# Outward winding follows M2; native loader reverses it for Godot.
	var triangles := [0, 2, 1, 0, 3, 2]
	for index in triangles.size():
		put_u16(skin, 52 + index * 2, triangles[index])
	put_u16(skin, 64 + 4, 0)
	put_u16(skin, 64 + 6, 4)
	put_u16(skin, 64 + 8, 0)
	put_u16(skin, 64 + 10, 6)
	put_u16(skin, 112 + 2, shader_id)
	put_u16(skin, 112 + 4, 0) # Submesh 0.
	put_u16(skin, 112 + 10, 0) # Render flags/material 0.
	put_u16(skin, 112 + 14, texture_count)
	put_u16(skin, 112 + 16, 0) # Texture lookup starts at 0.
	return skin

func fixture_vertex_offset(model: PackedByteArray, skin: PackedByteArray, triangle_corner: int) -> int:
	var lookup_index := skin.decode_u16(52 + triangle_corner * 2)
	var vertex_index := skin.decode_u16(44 + lookup_index * 2)
	return 8 + model.decode_u32(8 + 0x40) + vertex_index * 48

func fixture_vector(model: PackedByteArray, offset: int) -> Vector3:
	return Vector3(model.decode_float(offset), model.decode_float(offset + 4), model.decode_float(offset + 8))

func assert_fixture_winding() -> bool:
	var model := make_m2(7, 0)
	var skin := make_skin(0x10, 1)
	for triangle_start in range(0, 6, 3):
		var vertices: Array[Vector3] = []
		for corner in 3:
			var offset := fixture_vertex_offset(model, skin, triangle_start + corner)
			vertices.append(fixture_vector(model, offset))
		var normal_offset := fixture_vertex_offset(model, skin, triangle_start) + 20
		var authored_normal := fixture_vector(model, normal_offset)
		var face_normal := (vertices[1] - vertices[0]).cross(vertices[2] - vertices[0])
		if face_normal.dot(authored_normal) <= 0.0:
			push_error("M2 fixture triangle %d faces away from its authored normal" % (triangle_start / 3))
			return false
	return true

func make_blp(color: Color) -> PackedByteArray:
	# BLP2 direct/raw3: 148-byte header, 256-entry palette, BGRA mip 0.
	var bytes := PackedByteArray()
	bytes.resize(148 + 1024 + 2 * 2 * 4)
	put_magic(bytes, 0, "BLP2")
	put_u32(bytes, 4, 1) # Direct content.
	bytes[8] = 3 # Raw3 compression (image-blp encoding 3).
	bytes[9] = 8 # Alpha bits.
	put_u32(bytes, 12, 2)
	put_u32(bytes, 16, 2)
	put_u32(bytes, 20, 1172)
	put_u32(bytes, 84, 16)
	for index in 4:
		var offset := 1172 + index * 4
		bytes[offset] = roundi(color.b * 255.0)
		bytes[offset + 1] = roundi(color.g * 255.0)
		bytes[offset + 2] = roundi(color.r * 255.0)
		bytes[offset + 3] = roundi(color.a * 255.0)
	return bytes

func write_fixture(path: String, bytes: PackedByteArray) -> bool:
	var file := FileAccess.open(path, FileAccess.WRITE)
	if file == null:
		push_error("Cannot write %s: %s" % [path, error_string(FileAccess.get_open_error())])
		return false
	file.store_buffer(bytes)
	file.close()
	return true

func prepare_fixture(flags: int, shader_id: int, texture_count: int, blend_mode: int = 0) -> bool:
	var dir := ProjectSettings.globalize_path(FIXTURE)
	var created := DirAccess.make_dir_recursive_absolute(dir + "/models")
	if created != OK:
		push_error("Cannot create M2 fixture: " + error_string(created))
		return false
	created = DirAccess.make_dir_recursive_absolute(dir + "/textures")
	if created != OK:
		push_error("Cannot create BLP fixture: " + error_string(created))
		return false
	return write_fixture(FIXTURE + "/models/quad.m2", make_m2(flags, blend_mode)) \
		and write_fixture(FIXTURE + "/models/quad00.skin", make_skin(shader_id, texture_count)) \
		and write_fixture(FIXTURE + "/textures/910001.blp", make_blp(BASE)) \
		and write_fixture(FIXTURE + "/textures/910002.blp", make_blp(SECOND))

func make_viewport() -> void:
	viewport = SubViewport.new()
	viewport.size = Vector2i(VIEW_SIZE, VIEW_SIZE)
	viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	viewport.own_world_3d = true
	get_root().add_child(viewport)
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
	camera.position = Vector3(0.0, 2.0, 0.0)
	camera.look_at_from_position(camera.position, Vector3.ZERO, Vector3(0.0, 0.0, -1.0))
	viewport.add_child(camera)
	camera.current = true
	var sun := DirectionalLight3D.new()
	sun.rotation.x = -PI / 2.0
	viewport.add_child(sun)

func read_center() -> Color:
	await process_frame
	await RenderingServer.frame_post_draw
	await process_frame
	await RenderingServer.frame_post_draw
	var image := viewport.get_texture().get_image()
	if image == null or image.is_empty():
		return Color(-1.0, -1.0, -1.0)
	return image.get_pixel(VIEW_SIZE / 2, VIEW_SIZE / 2)

func assert_pixel(label: String, expected: Color) -> bool:
	var actual := await read_center()
	if absf(actual.r - expected.r) > TOLERANCE or absf(actual.g - expected.g) > TOLERANCE or absf(actual.b - expected.b) > TOLERANCE:
		push_error("%s: expected %s, got %s" % [label, expected, actual])
		return false
	print("PASS: %s expected %s actual %s" % [label, expected, actual])
	return true

func load_quad() -> bool:
	if loaded != null:
		loaded.queue_free()
		await process_frame
	var result: Dictionary = loader.load_m2(FIXTURE + "/models/quad.m2")
	if result.has("error") or not result.get("node") is Node3D:
		push_error("M2 load: " + str(result.get("error", "no node")))
		return false
	if not result.get("missing_texture_fdids", PackedInt32Array()).is_empty():
		push_error("Fixture BLP lookup failed: " + str(result.missing_texture_fdids))
		return false
	loaded = result.node
	viewport.add_child(loaded)
	return true

func cleanup() -> void:
	if loaded != null and is_instance_valid(loaded):
		loaded.free()
	if viewport != null and is_instance_valid(viewport):
		viewport.free()
	for filename in ["models/quad.m2", "models/quad00.skin", "textures/910001.blp", "textures/910002.blp"]:
		DirAccess.remove_absolute(ProjectSettings.globalize_path(FIXTURE + "/" + filename))
	for folder in ["models", "textures", ""]:
		DirAccess.remove_absolute(ProjectSettings.globalize_path(FIXTURE + "/" + folder))

func run_cases() -> void:
	if not assert_fixture_winding():
		quit(1)
		return
	if not ClassDB.class_exists("WowAssetLoader"):
		push_error("WowAssetLoader not registered")
		quit(1)
		return
	loader = ClassDB.instantiate("WowAssetLoader")
	make_viewport()
	var passed := await run_pixels()
	cleanup()
	loader = null
	quit(0 if passed else 1)

func run_pixels() -> bool:
	if not prepare_fixture(7, 0x10, 1) or not await load_quad():
		return false
	var base_passed := await assert_pixel("unlit unfogged base", BASE)
	if not prepare_fixture(6, 0x10, 1) or not await load_quad():
		return false
	var lit_expected := Color(BASE.r * 1.1, BASE.g * 1.1, BASE.b * 1.1)
	var lit_passed := await assert_pixel("lit authored ambient", lit_expected)
	if not prepare_fixture(7, 0x4014, 2) or not await load_quad():
		return false
	var combined := Color(BASE.r * SECOND.r * 2.0, BASE.g * SECOND.g * 2.0, BASE.b * SECOND.b * 2.0)
	var combiner_passed := await assert_pixel("two-texture 0x4014", combined)
	if not prepare_fixture(7, 0x4014, 2, 2) or not await load_quad():
		return false
	var effect_passed := await assert_pixel("blend-2 two-texture 0x4014", combined)
	return base_passed and lit_passed and combiner_passed and effect_passed
