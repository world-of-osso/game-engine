// Retail scene fog: WebWowViewerCpp commonFogFunctions.slang makeFog2 (legacy exponential
// and artistic polynomial fog, height fog, end and height colours, sun scattering), in
// Godot world space (y up). Colours are linear RGB and mix in authored (gamma) space,
// as the reference's byte colours do. Rust fills the uniforms from the FogResult
// (lighting/mod.rs `TerrainLight::bind_model`, MapSceneRenderer.cpp:214-294).

// 0 off, 1 retail fog.
uniform int fog_mode = 0;
uniform float fog_opacity;
// FogColor (SkyFogColor), EndFogColor, FogHeightColor, EndFogHeightColor, SunFogColor.
uniform vec3 fog_color;
uniform vec3 fog_end_color;
uniform vec3 fog_height_color;
uniform vec3 fog_height_end_color;
uniform vec3 fog_sun_color;
// densityParams: start, end (yards), FogDensity and FogHeightDensity per yard.
uniform vec2 fog_range;
uniform float fog_density;
uniform float fog_height_density;
// World height of the height-fog plane (FogHeight) and FogHeightScaler.
uniform float fog_height;
uniform float fog_height_rate;
uniform float fog_z_scalar;
// mix(artistic, legacy, fog_legacy_scalar).
uniform float fog_legacy_scalar = 1.0;
// MainFogStartDist, MainFogEndDist; FogStartOffset, EndFogColorDistance.
uniform vec2 fog_main_range = vec2(0.0, 0.001);
uniform vec2 fog_color_range = vec2(0.0, 1000.0);
// Polynomials (x^3, x^2, x, 1): height factor, main and height-density artistic fog.
uniform vec4 fog_height_coefficients = vec4(1.0, 0.0, 0.0, 0.0);
uniform vec4 fog_main_coefficients;
uniform vec4 fog_height_density_coefficients;
// World direction toward the sun, SunFogAngle, SunAngleBlend * SunFogStrength.
uniform vec3 fog_sun_direction = vec3(0.0, 1.0, 0.0);
uniform float fog_sun_angle = 1.0;
uniform float fog_sun_percentage;

vec3 retail_fog_gamma(vec3 linear_color) {
	vec3 low = linear_color * 12.92;
	vec3 high = 1.055 * pow(max(linear_color, vec3(0.0)), vec3(1.0 / 2.4)) - 0.055;
	return mix(high, low, lessThanEqual(linear_color, vec3(0.0031308)));
}

// validateFogColor: blend modes 3/10/13 fog to black, 4 to white, 5 to grey.
vec3 fog_blend_color(vec3 linear_color, int fog_blend) {
	if (fog_blend == 3 || fog_blend == 10 || fog_blend == 13) {
		return vec3(0.0);
	} else if (fog_blend == 4) {
		return vec3(1.0);
	} else if (fog_blend == 5) {
		return vec3(0.5);
	}
	return retail_fog_gamma(linear_color);
}

float fog_poly(vec4 coefficients, float x) {
	return clamp(((coefficients.x * x + coefficients.y) * x + coefficients.z) * x + coefficients.w, 0.0, 1.0);
}

// Visibility of the scene through the fog (1 clear), and the height-fog factor.
vec2 retail_fog_visibility(vec3 relative, float world_height) {
	// calculateAdjustedViewPosition: FogZScalar pulls the point toward the eye's level.
	float up = relative.y;
	float pull = up > 0.0 ? clamp(fog_z_scalar, 0.0, up) : clamp(-fog_z_scalar, up, 0.0);
	float dist = length(relative - vec3(0.0, pull, 0.0));
	// calculateHeightFogFactor, on the unadjusted point.
	float height = world_height - fog_height;
	float height_factor = 1.0 - fog_poly(fog_height_coefficients, 1.0 - clamp(height * fog_height_rate, 0.0, 1.0));
	// calculateLegacyFog.
	float beyond = max(0.0, dist - fog_range.x);
	float legacy = mix(exp(-beyond * fog_density), exp(-beyond * fog_height_density), height_factor);
	legacy = min(legacy, clamp(1.42857146 * (1.0 - dist / fog_range.y), 0.0, 1.0));
	// calculateArtisticFog.
	float main_start = fog_main_range.x;
	float main_end = fog_main_range.y;
	float art_distance = clamp((dist - main_start) / (main_end - main_start), 0.0, 1.0);
	float engine_distance = clamp(dist / main_end, 0.0, 1.0);
	float artistic = mix(1.0 - fog_poly(fog_main_coefficients, art_distance),
		1.0 - fog_poly(fog_height_density_coefficients, art_distance), height_factor);
	float blend_start = clamp(fog_range.y / main_end, 0.0, 1.0) - 0.3;
	float at_blend_start = mix(fog_poly(fog_main_coefficients, blend_start),
		fog_poly(fog_height_density_coefficients, blend_start), height_factor);
	float end_blend = clamp((engine_distance - blend_start) / 0.3, 0.0, 1.0);
	float end_fog = clamp(1.0 - (end_blend * (1.0 - at_blend_start) + at_blend_start), 0.0, 1.0);
	artistic = min(artistic, mix(artistic, end_fog, end_blend));
	return vec2(mix(artistic, legacy, fog_legacy_scalar), height_factor);
}

// calculateFogColor, on the unadjusted distance.
vec3 retail_fog_color_at(vec3 relative, float height_factor, int fog_blend) {
	float dist = length(relative);
	float far = clamp((dist - fog_color_range.x) / fog_color_range.y, 0.0, 1.0);
	vec3 base = mix(fog_blend_color(fog_color, fog_blend), fog_blend_color(fog_end_color, fog_blend), far * far * far);
	vec3 high = mix(fog_blend_color(fog_height_color, fog_blend), fog_blend_color(fog_height_end_color, fog_blend), far);
	vec3 color = mix(base, high, height_factor);
	float sun = clamp(clamp(dot(relative / max(dist, 1e-6), fog_sun_direction), 0.0, 1.0) - fog_sun_angle, 0.0, 1.0);
	if (sun > 0.0) {
		vec3 sun_color = mix(color, fog_blend_color(fog_sun_color, fog_blend), fog_sun_percentage);
		float scatter = sun / (1.0 - fog_sun_angle);
		color = mix(color, sun_color, scatter * scatter * scatter);
	}
	return color;
}

// `color` (authored space) at `world_position` seen from `camera_position`, fogged.
vec3 retail_fog(vec3 color, vec3 world_position, vec3 camera_position, int fog_blend) {
	if (fog_mode != 1) {
		return color;
	}
	vec3 relative = world_position - camera_position;
	vec2 visibility = retail_fog_visibility(relative, world_position.y);
	vec3 fogged = retail_fog_color_at(relative, visibility.y, fog_blend);
	return mix(color, fogged, (1.0 - visibility.x) * fog_opacity);
}
