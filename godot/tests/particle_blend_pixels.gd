extends "res://tests/m2_blend_pixels.gd"

# One pooled M2 particle quad per blend type over a known background: WebWowViewerCpp's
# ParticleBlendingModeToEGxBlendEnum and GL factors (the M2 batch table), the particle
# colour multiplying the texel in authored space, the placement fade, and the scene fog
# (m2ParticleShader.frag.slang makeFog2 with the particle's blend mode). The factors
# apply to authored (gamma) values: WWV writes the fragment colour unconverted into an
# R8G8B8A8_UNORM target (GFrameBufferVLK.cpp:21), so every expectation below is the raw
# equation on the displayed 8-bit values, never a linear-light blend.
const PARTICLE_FDID := 910001
const TINT := Color(0.5, 1.0, 0.25, 1.0)
const FADE := 0.5
# Legacy fog at exp(-2 yd * ln 2 / 2): half fogged at the camera's 2 yd.
const HALF_DENSITY := 0.34657359

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

func gamma(color: Color) -> Color:
	return Color(clampf(color.r, 0.0, 1.0), clampf(color.g, 0.0, 1.0), clampf(color.b, 0.0, 1.0))

func texel_rgb() -> Color:
	return Color(BLEND_TEXEL.r, BLEND_TEXEL.g, BLEND_TEXEL.b)

## GDeviceGL20.cpp:21-35 factors on authored values; AlphaKey shares Opaque's output.
func gamma_expectations() -> Dictionary:
	var src := texel_rgb()
	var dst := BACKGROUND
	var a := BLEND_TEXEL.a
	return {
		0: gamma(src),
		1: gamma(src),
		2: gamma(src * a + dst * (1.0 - a)),
		3: gamma(src + dst),
		4: gamma(src * a + dst),
		5: gamma(src * dst),
		6: gamma(src * dst * 2.0),
		7: gamma(src + dst * (1.0 - a)),
	}

func faded_expectations() -> Dictionary:
	var dst := BACKGROUND
	var a := BLEND_TEXEL.a
	var texel := texel_rgb()
	return {
		2: gamma(texel * a * FADE + dst * (1.0 - a * FADE)),
		3: gamma(texel * FADE + dst),
		4: gamma(texel * a * FADE + dst),
		5: gamma(Color.WHITE.lerp(texel, FADE) * dst),
		7: gamma(texel * FADE + dst * (1.0 - a * FADE)),
	}

func run_particle_pixels() -> bool:
	var passed := true
	# AlphaKey: texel alpha 128/255 passes the 0.502 test.
	var expected := gamma_expectations()
	for mode in expected:
		if not await show_quad(mode, Color.WHITE, 1.0):
			return false
		passed = await assert_pixel("particle blend %d" % mode, expected[mode]) and passed
	if not await show_quad(2, TINT, 1.0):
		return false
	var tinted_src := Color(BLEND_TEXEL.r * TINT.r, BLEND_TEXEL.g * TINT.g, BLEND_TEXEL.b * TINT.b)
	var a := BLEND_TEXEL.a
	passed = await assert_pixel("particle colour tints the texel", gamma(tinted_src * a + BACKGROUND * (1.0 - a))) and passed
	var faded := faded_expectations()
	for mode in faded:
		if not await show_quad(mode, Color.WHITE, FADE):
			return false
		passed = await assert_pixel("particle blend %d at fade %.1f" % [mode, FADE], faded[mode]) and passed
	return await run_fog_pixels() and passed

## The scene fog every pool reads once `TerrainLight::bind_scene_fog` enables it (global
## uniforms, `TerrainLight::bind_scene`): white legacy fog, half at the camera's distance.
func bind_half_fog() -> void:
	var material := (particle as GeometryInstance3D).material_override as ShaderMaterial
	material.set_shader_parameter("fog_mode", 1)
	material.set_shader_parameter("fog_opacity", 1.0)
	RenderingServer.global_shader_parameter_set("fog_range", Vector2(0.0, 1000.0))
	RenderingServer.global_shader_parameter_set("fog_density", HALF_DENSITY)
	RenderingServer.global_shader_parameter_set("fog_height_density", 0.0)
	RenderingServer.global_shader_parameter_set("fog_height", -10000.0)
	RenderingServer.global_shader_parameter_set("fog_height_rate", 0.0)
	RenderingServer.global_shader_parameter_set("fog_z_scalar", 0.0)
	RenderingServer.global_shader_parameter_set("fog_legacy_scalar", 1.0)
	RenderingServer.global_shader_parameter_set("fog_main_range", Vector2(0.0, 0.001))
	RenderingServer.global_shader_parameter_set("fog_color_range", Vector2(0.0, 10000.0))
	RenderingServer.global_shader_parameter_set("fog_height_coefficients", Vector4(1.0, 0.0, 0.0, 0.0))
	RenderingServer.global_shader_parameter_set("fog_main_coefficients", Vector4.ZERO)
	RenderingServer.global_shader_parameter_set("fog_height_density_coefficients", Vector4.ZERO)
	RenderingServer.global_shader_parameter_set("fog_sun_angle", 1.0)
	RenderingServer.global_shader_parameter_set("fog_sun_percentage", 0.0)
	for name in ["fog_color", "fog_end_color", "fog_height_color", "fog_height_end_color", "fog_sun_color"]:
		RenderingServer.global_shader_parameter_set(name, Vector3.ONE)

## Alpha particles fog toward the fog colour; additive ones toward black
## (validateFogColor), so their contribution halves.
func run_fog_pixels() -> bool:
	var passed := true
	var a := BLEND_TEXEL.a
	var texel := texel_rgb()
	var dst := BACKGROUND
	var expected := {
		2: gamma(texel.lerp(Color.WHITE, 0.5) * a + dst * (1.0 - a)),
		4: gamma(texel * 0.5 * a + dst),
	}
	for mode in expected:
		if not await show_quad(mode, Color.WHITE, 1.0):
			return false
		bind_half_fog()
		passed = await assert_pixel("particle blend %d in half fog" % mode, expected[mode]) and passed
	return passed
