extends "res://tests/m2_blend_pixels.gd"

# One pooled M2 particle quad per blend type over a known background: WebWowViewerCpp's
# ParticleBlendingModeToEGxBlendEnum and GL factors (the M2 batch table), the particle
# colour multiplying the texel in authored space, and the placement fade.
const PARTICLE_FDID := 910001
const TINT := Color(0.5, 1.0, 0.25, 1.0)
const FADE := 0.5

var particle: Node3D
var probe: Object

func run_cases() -> void:
	if not ClassDB.class_exists("WowParticleProbe"):
		push_error("WowParticleProbe not registered")
		quit(1)
		return
	probe = ClassDB.instantiate("WowParticleProbe")
	if not prepare_fixture(7, 0x10, 1):
		quit(1)
		return
	make_viewport()
	add_background()
	var passed := await run_particle_pixels()
	if particle != null and is_instance_valid(particle):
		particle.free()
	cleanup()
	probe = null
	quit(0 if passed else 1)

func show_quad(blend: int, color: Color, fade: float) -> bool:
	if particle != null:
		particle.queue_free()
		await process_frame
	var node = probe.quad_node(blend, FIXTURE + "/textures", PARTICLE_FDID, color, fade, 1.5)
	if not node is Node3D:
		push_error("particle quad %d: %s" % [blend, node])
		return false
	particle = node
	viewport.add_child(particle)
	return true

func faded_expectations() -> Dictionary:
	var dst := linear(BACKGROUND)
	var a := BLEND_TEXEL.a
	var texel := Color(BLEND_TEXEL.r, BLEND_TEXEL.g, BLEND_TEXEL.b)
	var src := linear(texel)
	var scaled := linear(texel * FADE)
	var toward_white := linear(Color.WHITE.lerp(texel, FADE))
	return {
		2: encoded(src * a * FADE + dst * (1.0 - a * FADE)),
		3: encoded(scaled + dst),
		4: encoded(src * a * FADE + dst),
		5: encoded(toward_white * dst),
		7: encoded(scaled + dst * (1.0 - a * FADE)),
	}

func run_particle_pixels() -> bool:
	var passed := true
	var expected := blend_expectations()
	expected[1] = expected[0] # AlphaKey: texel alpha 128/255 passes the 0.502 test.
	for mode in expected:
		if not await show_quad(mode, Color.WHITE, 1.0):
			return false
		passed = await assert_pixel("particle blend %d" % mode, expected[mode]) and passed
	if not await show_quad(2, TINT, 1.0):
		return false
	var tinted_src := linear(Color(BLEND_TEXEL.r * TINT.r, BLEND_TEXEL.g * TINT.g, BLEND_TEXEL.b * TINT.b))
	var a := BLEND_TEXEL.a
	passed = await assert_pixel("particle colour tints the texel", encoded(tinted_src * a + linear(BACKGROUND) * (1.0 - a))) and passed
	var faded := faded_expectations()
	for mode in faded:
		if not await show_quad(mode, Color.WHITE, FADE):
			return false
		passed = await assert_pixel("particle blend %d at fade %.1f" % [mode, FADE], faded[mode]) and passed
	return passed
