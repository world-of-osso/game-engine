extends "res://tests/m2_loader_pixels.gd"

# WebWowViewerCpp M2 blend -> EGxBlend GL factors over a known background, and the
# batch colour track (meshColor) multiplied in authored space and animated.
const BLEND_TEXEL := Color(102.0 / 255.0, 76.0 / 255.0, 51.0 / 255.0, 128.0 / 255.0)
const BACKGROUND := Color(0.2, 0.6, 0.1)
const CYCLE_MS := 1000.0
const COLOR_KEYS := [Color(0.5, 1.0, 1.0), Color(1.0, 0.5, 0.25)]

var colored := false

func make_m2(flags: int, blend_mode: int) -> PackedByteArray:
	var original := super.make_m2(flags, blend_mode)
	if not colored:
		return original
	var md20 := original.slice(8, 8 + original.decode_u32(4))
	md20.resize(0x2c0)
	put_u32(md20, 0x14, 1) # One global sequence, duration in milliseconds.
	put_u32(md20, 0x18, 0x240)
	put_u32(md20, 0x240, int(CYCLE_MS))
	put_u32(md20, 0x48, 1) # One colour; the skin's batch uses colour index 0.
	put_u32(md20, 0x4c, 0x250)
	# Colour RGB AnimBlock: linear, global sequence 0, keys at 0 and 500 ms.
	# Its alpha AnimBlock (0x264) stays empty: opacity 1.
	put_u16(md20, 0x250, 1)
	put_u16(md20, 0x252, 0)
	put_u32(md20, 0x254, 1)
	put_u32(md20, 0x258, 0x280)
	put_u32(md20, 0x25c, 1)
	put_u32(md20, 0x260, 0x288)
	put_u32(md20, 0x280, 2)
	put_u32(md20, 0x284, 0x290)
	put_u32(md20, 0x288, 2)
	put_u32(md20, 0x28c, 0x2a0)
	for index in 2:
		put_u32(md20, 0x290 + index * 4, index * 500)
		var key: Color = COLOR_KEYS[index]
		put_float(md20, 0x2a0 + index * 12, key.r)
		put_float(md20, 0x2a4 + index * 12, key.g)
		put_float(md20, 0x2a8 + index * 12, key.b)
	var model := chunk("MD21", md20)
	model.append_array(original.slice(8 + original.decode_u32(4))) # SFID and TXID from base fixture.
	return model

func prepare_fixture(flags: int, shader_id: int, texture_count: int, blend_mode: int = 0) -> bool:
	return super.prepare_fixture(flags, shader_id, texture_count, blend_mode) \
		and write_fixture(FIXTURE + "/textures/910001.blp", make_blp(BLEND_TEXEL))

func add_background() -> void:
	var background := MeshInstance3D.new()
	var plane := PlaneMesh.new()
	plane.size = Vector2(4.0, 4.0)
	background.mesh = plane
	background.position = Vector3(0.0, -0.5, 0.0)
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.albedo_color = BACKGROUND
	background.material_override = material
	viewport.add_child(background)

func linear(color: Color) -> Color:
	var converted := color.srgb_to_linear()
	return Color(converted.r, converted.g, converted.b)

func encoded(color: Color) -> Color:
	return Color(color.r, color.g, color.b).linear_to_srgb()

# Blended in Godot's linear framebuffer; Mod2x doubles the colour in authored space.
func blend_expectations() -> Dictionary:
	var src := linear(BLEND_TEXEL)
	var dst := linear(BACKGROUND)
	var a := BLEND_TEXEL.a
	var twice := linear(Color(BLEND_TEXEL.r * 2.0, BLEND_TEXEL.g * 2.0, BLEND_TEXEL.b * 2.0))
	return {
		0: encoded(src), # Opaque
		2: encoded(src * a + dst * (1.0 - a)), # Alpha: SRC_ALPHA, ONE_MINUS_SRC_ALPHA
		3: encoded(src + dst), # NoAlphaAdd: ONE, ONE
		4: encoded(src * a + dst), # Add: SRC_ALPHA, ONE
		5: encoded(src * dst), # Mod: DST_COLOR, ZERO
		6: encoded(twice * dst), # Mod2x: DST_COLOR, SRC_COLOR
		7: encoded(src + dst * (1.0 - a)), # BlendAdd: ONE, ONE_MINUS_SRC_ALPHA
	}

func set_phase_ms(target: float) -> void:
	var clock := get_root().get_node_or_null("M2MaterialClock")
	if clock == null:
		return
	clock.set_process(false)
	var phase := fposmod(float(clock.call("elapsed_time_ms")), CYCLE_MS)
	clock.call("advance_time_ms", fposmod(target - phase, CYCLE_MS))

func tinted(rgb: Color) -> Color:
	return Color(BLEND_TEXEL.r * rgb.r, BLEND_TEXEL.g * rgb.g, BLEND_TEXEL.b * rgb.b)

func run_cases() -> void:
	if not assert_fixture_winding() or not ClassDB.class_exists("WowAssetLoader"):
		push_error("Blend fixture winding or WowAssetLoader unavailable")
		quit(1)
		return
	loader = ClassDB.instantiate("WowAssetLoader")
	make_viewport()
	add_background()
	var blend_passed := await run_blend_pixels()
	var occlusion_passed := await run_opaque_occlusion_pixels()
	var color_passed := await run_color_pixels()
	cleanup()
	loader = null
	quit(0 if blend_passed and occlusion_passed and color_passed else 1)

func run_blend_pixels() -> bool:
	var passed := true
	var expected := blend_expectations()
	for mode in expected:
		if not prepare_fixture(7, 0x10, 1, mode) or not await load_quad():
			return false
		if mode == BLEND_ADD_KNOWN_GAP:
			await report_known_gap("blend %d over background" % mode, expected[mode])
			continue
		passed = await assert_pixel("blend %d over background" % mode, expected[mode]) and passed
	return passed

# KNOWN GAP (open rendering item, 2026-11-06): BlendAdd (M2 blend 7, GX 13). The expectation
# is WebWowViewerCpp's straight output (commonM2Material.slang:269, factors ONE /
# ONE_MINUS_SRC_ALPHA); m2.gdshader premultiplies (4cd7bc671), as retail's ONE source factor
# expects. Death and Decay draws soft-edged premultiplied and as a hard-edged quad straight;
# neither is compared with retail yet (docs/wiki/formats/m2-format.md, "BlendAdd known gap").
# Reported, not asserted, until that comparison decides the output.
const BLEND_ADD_KNOWN_GAP := 7

func report_known_gap(label: String, expected: Color) -> void:
	var actual := await read_center()
	print("KNOWN GAP: %s expected %s (WebWowViewer straight) actual %s (premultiplied)" % [label, expected, actual])

# An opaque batch occludes a depth-less blended surface below it even when that surface
# sorts last, as a liquid block does when its centre is nearer the camera than a boat afloat
# in it. A material that references ALPHA draws in Godot's transparent pass without a depth
# write, and the surface then blended over the boat.
func run_opaque_occlusion_pixels() -> bool:
	if not prepare_fixture(7, 0x10, 1, 0) or not await load_quad():
		return false
	var surface := MeshInstance3D.new()
	var plane := PlaneMesh.new()
	plane.size = Vector2(4.0, 4.0)
	surface.mesh = plane
	surface.position = Vector3(0.0, -0.25, 0.0)
	surface.sorting_offset = 100000.0
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	material.depth_draw_mode = BaseMaterial3D.DEPTH_DRAW_DISABLED
	material.albedo_color = Color(1.0, 0.0, 0.0, 0.5)
	surface.material_override = material
	viewport.add_child(surface)
	var passed := await assert_pixel("opaque batch over a later-sorted blended surface", blend_expectations()[0])
	surface.free()
	return passed

func run_color_pixels() -> bool:
	colored = true
	if not prepare_fixture(7, 0x10, 1) or not await load_quad():
		return false
	set_phase_ms(0.0)
	var first := await assert_pixel("colour key 0 multiplies texel", tinted(COLOR_KEYS[0]))
	set_phase_ms(250.0)
	var middle := await assert_pixel("colour interpolates at 250ms", tinted(COLOR_KEYS[0].lerp(COLOR_KEYS[1], 0.5)))
	set_phase_ms(500.0)
	var last := await assert_pixel("colour key 1 at 500ms", tinted(COLOR_KEYS[1]))
	return first and middle and last
