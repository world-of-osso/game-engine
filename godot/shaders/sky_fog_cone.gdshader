shader_type spatial;
render_mode unshaded, fog_disabled, cull_disabled, blend_mix, depth_draw_never, shadows_disabled;

// LightSkybox flag 0x4 fog cone: skyConus.vert/.frag.slang with `skyMeshMat0x4` colours.
// The end fog colour at the vertex alpha, scattered toward SunFogColor along the sun as
// the dome (sky_dome.gdshader); colours mix in authored space.

uniform vec3 fog_color = vec3(0.0);
uniform vec3 fog_sun_direction = vec3(0.0, 1.0, 0.0);
uniform float fog_sun_angle = 1.0;
uniform float fog_sun_percentage;
uniform vec3 fog_sun_color;
varying vec3 direction;

vec3 linear_to_gamma(vec3 linear_color) {
	vec3 low = linear_color * 12.92;
	vec3 high = 1.055 * pow(max(linear_color, vec3(0.0)), vec3(1.0 / 2.4)) - 0.055;
	return mix(high, low, lessThanEqual(linear_color, vec3(0.0031308)));
}

vec3 gamma_to_linear(vec3 gamma_color) {
	vec3 low = gamma_color / 12.92;
	vec3 high = pow((max(gamma_color, vec3(0.0)) + 0.055) / 1.055, vec3(2.4));
	return mix(high, low, lessThanEqual(gamma_color, vec3(0.04045)));
}

void vertex() {
	direction = VERTEX;
	POSITION = PROJECTION_MATRIX * MODELVIEW_MATRIX * vec4(VERTEX, 1.0);
	// Reverse-Z far plane, as the other sky models: behind every world surface.
	POSITION.z = POSITION.w * 0.000001;
}

void fragment() {
	vec3 color = linear_to_gamma(fog_color);
	float n_dot_sun = clamp(dot(normalize(direction), fog_sun_direction) - fog_sun_angle, 0.0, 1.0);
	if (n_dot_sun > 0.0) {
		float scatter = n_dot_sun / (1.0 - fog_sun_angle);
		scatter = scatter * scatter * scatter;
		color = mix(color, linear_to_gamma(fog_sun_color), clamp(scatter * fog_sun_percentage, 0.0, 1.0));
	}
	ALBEDO = gamma_to_linear(color);
	ALPHA = COLOR.a;
}
