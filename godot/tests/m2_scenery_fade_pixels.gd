extends "res://tests/m2_blend_pixels.gd"

# Retail scenery distance fade over a known background: an opaque batch draws its
# authored opaque variant at full opacity and blends by `scenery_opacity` alone
# (ignoring texel alpha) through its fade variant; an alpha-blended batch multiplies
# its own alpha by it.

func batch_material() -> ShaderMaterial:
	var mesh := loaded.get_node_or_null("Batch0") as MeshInstance3D
	return mesh.get_active_material(0) as ShaderMaterial if mesh != null else null

func fade_shader(material: ShaderMaterial) -> Shader:
	return material.get_meta("scenery_fade_shader") if material.has_meta("scenery_fade_shader") else null

func run_cases() -> void:
	if not assert_fixture_winding() or not ClassDB.class_exists("WowAssetLoader"):
		push_error("Fade fixture winding or WowAssetLoader unavailable")
		quit(1)
		return
	loader = ClassDB.instantiate("WowAssetLoader")
	make_viewport()
	add_background()
	var passed := await run_fade_pixels()
	cleanup()
	loader = null
	quit(0 if passed else 1)

func run_fade_pixels() -> bool:
	var src := linear(BLEND_TEXEL)
	var dst := linear(BACKGROUND)
	var a := BLEND_TEXEL.a
	if not prepare_fixture(7, 0x10, 1, 0) or not await load_quad():
		return false
	var opaque := batch_material()
	var fade := fade_shader(opaque) if opaque != null else null
	if fade == null:
		push_error("Opaque M2 batch has no scenery fade shader")
		return false
	var passed := await assert_pixel("opaque authored variant ignores texel alpha", encoded(src))
	opaque.shader = fade
	for opacity in [0.5, 0.25]:
		opaque.set_shader_parameter("scenery_opacity", opacity)
		passed = await assert_pixel("opaque fade %.2f" % opacity, encoded(src * opacity + dst * (1.0 - opacity))) and passed
	if not prepare_fixture(7, 0x10, 1, 2) or not await load_quad():
		return false
	var blended := batch_material()
	if blended == null or fade_shader(blended) != null:
		push_error("Alpha-blended M2 batch must fade through its authored variant")
		return false
	blended.set_shader_parameter("scenery_opacity", 0.5)
	var faded_alpha := a * 0.5
	passed = await assert_pixel("alpha blend fade 0.50", encoded(src * faded_alpha + dst * (1.0 - faded_alpha))) and passed
	return passed
