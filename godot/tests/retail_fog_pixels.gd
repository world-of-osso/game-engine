extends "res://tests/m2_material_pixels.gd"

# makeFog2 (WebWowViewerCpp commonFogFunctions.slang) in shaders/retail_fog.gdshaderinc,
# through the unlit M2 material: a (0.4, 0.3, 0.2) floor 2 yd below the camera, looking
# straight down. Each case fixes the fog uniforms so the expected colour is exact.

const FLOOR := Color(0.4, 0.3, 0.2)
# exp(-2 yd * ln 2 / 2): half the floor shows through legacy fog at the fixture distance.
const HALF_DENSITY := 0.34657359

func fog_inputs() -> void:
	base_inputs()
	material.set_shader_parameter("base_texture", texture_color(FLOOR))
	material.set_shader_parameter("second_texture", texture_color(Color.WHITE))
	material.set_shader_parameter("shader_id", 0x10)
	material.set_shader_parameter("render_flags", 1)
	material.set_shader_parameter("fog_mode", 1)
	material.set_shader_parameter("fog_opacity", 1.0)
	material.set_shader_parameter("fog_range", Vector2(0.0, 1000.0))
	material.set_shader_parameter("fog_density", HALF_DENSITY)
	material.set_shader_parameter("fog_height_density", 0.0)
	material.set_shader_parameter("fog_height", -10000.0)
	material.set_shader_parameter("fog_height_rate", 0.0)
	material.set_shader_parameter("fog_z_scalar", 0.0)
	material.set_shader_parameter("fog_legacy_scalar", 1.0)
	material.set_shader_parameter("fog_main_range", Vector2(0.0, 0.001))
	material.set_shader_parameter("fog_color_range", Vector2(0.0, 10000.0))
	material.set_shader_parameter("fog_height_coefficients", Vector4(1.0, 0.0, 0.0, 0.0))
	material.set_shader_parameter("fog_main_coefficients", Vector4.ZERO)
	material.set_shader_parameter("fog_height_density_coefficients", Vector4.ZERO)
	material.set_shader_parameter("fog_sun_direction", Vector3.UP)
	material.set_shader_parameter("fog_sun_angle", 1.0)
	material.set_shader_parameter("fog_sun_percentage", 0.0)
	fog_colors(Color.WHITE, Color.WHITE, Color.WHITE)

# Authored colours, uploaded linear as Rust binds them.
func fog_colors(base: Color, end: Color, sun: Color) -> void:
	material.set_shader_parameter("fog_color", base.srgb_to_linear())
	material.set_shader_parameter("fog_end_color", end.srgb_to_linear())
	material.set_shader_parameter("fog_height_color", base.srgb_to_linear())
	material.set_shader_parameter("fog_height_end_color", end.srgb_to_linear())
	material.set_shader_parameter("fog_sun_color", sun.srgb_to_linear())

func fogged(fog: float, fog_color: float) -> Color:
	return FLOOR.lerp(Color(fog_color, fog_color, fog_color), fog)

func run_cases() -> void:
	material = ShaderMaterial.new()
	material.shader = load(SHADER_PATH)
	fixture()
	viewport.remove_child(sun)
	fog_inputs()
	if not await assert_pixel("legacy fog halves the floor at 2 yd", fogged(0.5, 1.0)):
		return
	# Height fog: 10 yd above the plane (rate 1) the height factor is 1 - (1 - 1)^3 = 1 and
	# the height density (0) applies; 10 yd below it, the scene density.
	material.set_shader_parameter("fog_height_rate", 1.0)
	material.set_shader_parameter("fog_height", -10.0)
	if not await assert_pixel("above the height plane the height density clears the fog", FLOOR):
		return
	material.set_shader_parameter("fog_height", 10.0)
	if not await assert_pixel("below the height plane the scene density fogs", fogged(0.5, 1.0)):
		return
	fog_inputs()
	# FogZScalar 1 pulls the point 1 yd toward the eye's level: exp(-1 * ln 2 / 2).
	material.set_shader_parameter("fog_z_scalar", 1.0)
	if not await assert_pixel("FogZScalar shortens the fog distance", fogged(1.0 - 0.70710678, 1.0)):
		return
	fog_inputs()
	# Artistic fog: 1 - poly(x) with poly(x) = x over MainFog 0..8 yd: 0.25 at 2 yd.
	material.set_shader_parameter("fog_legacy_scalar", 0.0)
	material.set_shader_parameter("fog_main_coefficients", Vector4(0.0, 0.0, 1.0, 0.0))
	material.set_shader_parameter("fog_main_range", Vector2(0.0, 8.0))
	if not await assert_pixel("artistic polynomial fog", fogged(0.25, 1.0)):
		return
	fog_inputs()
	# End colour: (2 yd / 4 yd)^3 of the way from black to white.
	fog_colors(Color.BLACK, Color.WHITE, Color.BLACK)
	material.set_shader_parameter("fog_color_range", Vector2(0.0, 4.0))
	if not await assert_pixel("fog colour leans to the end colour by distance cubed", fogged(0.5, 0.125)):
		return
	fog_inputs()
	# Sun: view direction . sun = 0.75, angle 0.5: ((0.75 - 0.5) / 0.5)^3 = 0.125 of
	# the sun colour mixed at percentage 0.5.
	fog_colors(Color.BLACK, Color.BLACK, Color.WHITE)
	material.set_shader_parameter("fog_sun_direction", Vector3(0.66143783, -0.75, 0.0))
	material.set_shader_parameter("fog_sun_angle", 0.5)
	material.set_shader_parameter("fog_sun_percentage", 0.5)
	if not await assert_pixel("sun scattering toward the sun", fogged(0.5, 0.0625)):
		return
	material.set_shader_parameter("fog_sun_direction", Vector3(0.0, 1.0, 0.0))
	if not await assert_pixel("no sun scattering away from the sun", fogged(0.5, 0.0)):
		return
	sun.free()
	quit(0)
