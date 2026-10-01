extends "res://tests/m2_loader_pixels.gd"

const CYCLE_MS := 1000.0
const QUADRANTS := [
	Color(0.8, 0.1, 0.1), Color(0.1, 0.8, 0.1),
	Color(0.1, 0.1, 0.8), Color(0.8, 0.8, 0.1),
]

var first_model: Node3D
var animated := false

func make_m2(flags: int, blend_mode: int) -> PackedByteArray:
	var original := super.make_m2(flags, blend_mode)
	var md20 := original.slice(8, 8 + original.decode_u32(4))
	md20.resize(0x320)
	put_u32(md20, 0x14, 1) # One global sequence, duration in milliseconds.
	put_u32(md20, 0x18, 0x240)
	put_u32(md20, 0x60, 1) # One authored UV transform.
	put_u32(md20, 0x64, 0x244)
	put_u32(md20, 0x98, 1) # Batch texture_animation_id 0 resolves to track 0.
	put_u32(md20, 0x9c, 0x300)
	put_u32(md20, 0x240, int(CYCLE_MS))
	put_u16(md20, 0x300, 0 if animated else 0xffff)
	for vertex in 4:
		var offset := 0x140 + vertex * 48
		put_float(md20, offset + 32, 0.125)
		put_float(md20, offset + 36, 0.125)
	# Translation AnimBlock: linear interpolation, global sequence 0, sequence 0 keys.
	put_u16(md20, 0x244, 1)
	put_u16(md20, 0x246, 0)
	put_u32(md20, 0x248, 1)
	put_u32(md20, 0x24c, 0x280)
	put_u32(md20, 0x250, 1)
	put_u32(md20, 0x254, 0x288)
	put_u32(md20, 0x280, 5)
	put_u32(md20, 0x284, 0x290)
	put_u32(md20, 0x288, 5)
	put_u32(md20, 0x28c, 0x2a4)
	var offsets := [Vector2.ZERO, Vector2(0.5, 0.0), Vector2(0.5, 0.5), Vector2(0.0, 0.5), Vector2.ZERO]
	for index in 5:
		put_u32(md20, 0x290 + index * 4, index * 250)
		put_float(md20, 0x2a4 + index * 12, offsets[index].x)
		put_float(md20, 0x2a8 + index * 12, offsets[index].y)
	var model := chunk("MD21", md20)
	model.append_array(original.slice(8 + original.decode_u32(4))) # TXID from base fixture.
	return model

func make_pattern_blp() -> PackedByteArray:
	var bytes := PackedByteArray()
	bytes.resize(1172 + 4 * 4 * 4)
	put_magic(bytes, 0, "BLP2")
	put_u32(bytes, 4, 1)
	bytes[8] = 3
	bytes[9] = 8
	put_u32(bytes, 12, 4)
	put_u32(bytes, 16, 4)
	put_u32(bytes, 20, 1172)
	put_u32(bytes, 84, 64)
	for y in 4:
		for x in 4:
			var color: Color = QUADRANTS[floori(y / 2.0) * 2 + floori(x / 2.0)]
			var offset := 1172 + (y * 4 + x) * 4
			bytes[offset] = roundi(color.b * 255.0)
			bytes[offset + 1] = roundi(color.g * 255.0)
			bytes[offset + 2] = roundi(color.r * 255.0)
			bytes[offset + 3] = 255
	return bytes

func prepare_fixture(flags: int, shader_id: int, texture_count: int, blend_mode: int = 0) -> bool:
	if not super.prepare_fixture(flags, shader_id, texture_count, blend_mode):
		return false
	return write_fixture(FIXTURE + "/textures/910001.blp", make_pattern_blp()) \
		and write_fixture(FIXTURE + "/textures/910002.blp", make_blp(Color.WHITE))

func set_phase_ms(target: float) -> void:
	var clock := get_root().get_node_or_null("M2MaterialClock")
	if clock == null:
		return # Pre-implementation RED still observes the unanimated pixel.
	clock.set_process(false)
	var phase := fposmod(float(clock.call("elapsed_time_ms")), CYCLE_MS)
	clock.call("advance_time_ms", fposmod(target - phase, CYCLE_MS))

func run_cases() -> void:
	if not assert_fixture_winding() or not ClassDB.class_exists("WowAssetLoader"):
		push_error("UV fixture winding or WowAssetLoader unavailable")
		quit(1)
		return
	loader = ClassDB.instantiate("WowAssetLoader")
	make_viewport()
	var passed := await run_uv_pixels()
	if first_model != null and is_instance_valid(first_model):
		first_model.free()
	cleanup()
	loader = null
	quit(0 if passed else 1)

func run_uv_pixels() -> bool:
	if not prepare_fixture(7, 0x10, 1) or not await load_quad():
		return false
	set_phase_ms(250.0)
	var control_u := await assert_pixel("plain single-texture unchanged at 250ms", QUADRANTS[0])
	set_phase_ms(500.0)
	var control_v := await assert_pixel("plain single-texture unchanged at 500ms", QUADRANTS[0])
	# Diffuse_T1 applies texture matrix 0 to a single texture too (calcM2VertexMat).
	animated = true
	if not prepare_fixture(7, 0x10, 1) or not await load_quad():
		return false
	set_phase_ms(250.0)
	var single_u := await assert_pixel("single-texture authored +U translation", QUADRANTS[1])
	# Shader 0x10 over two textures multiplies the pattern by the white second texture.
	if not prepare_fixture(7, 0x10, 2, 2) or not await load_quad():
		return false
	set_phase_ms(0.0)
	var effect_base := await assert_pixel("effect baseline at 0ms", QUADRANTS[0])
	set_phase_ms(250.0)
	var u_passed := await assert_pixel("authored +U translation", QUADRANTS[1])
	set_phase_ms(500.0)
	var uv_passed := await assert_pixel("authored +U+V translation", QUADRANTS[3])
	first_model = loaded
	loaded = null
	first_model.visible = false
	if not await load_quad():
		return false
	var shared_phase := await assert_pixel("new model shares global 500ms phase", QUADRANTS[3])
	set_phase_ms(750.0)
	var v_passed := await assert_pixel("authored +V translation", QUADRANTS[2])
	set_phase_ms(1000.0)
	var wrap_passed := await assert_pixel("global duration wraps to first UV", QUADRANTS[0])
	var automatic := await assert_automatic_pixels()
	return control_u and control_v and single_u and effect_base and u_passed and uv_passed and shared_phase and v_passed and wrap_passed and automatic

func assert_automatic_pixels() -> bool:
	var clock := get_root().get_node_or_null("M2MaterialClock")
	if clock == null:
		push_error("Automatic material clock missing")
		return false
	clock.set_process(true)
	for attempt in 60:
		var color := await read_center()
		if color.g > 0.5 or color.b > 0.5:
			print("PASS: automatic clock changes rendered effect pixel: ", color)
			return true
	push_error("Automatic clock did not move the rendered effect texture")
	return false
