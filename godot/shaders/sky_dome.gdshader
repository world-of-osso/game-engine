shader_type sky;

// Retail exterior sky dome (WebWowViewerCpp map.cpp skyConusVBO/skyConusIBO with
// skyConus.vert/frag.slang): a camera-centred cone whose rings carry SkyTop, SkyMiddle,
// SkyBand1, SkyBand2, SkySmog and SkyFog colours, interpolated across each cone face,
// with the sun-fog scattering of skyConus.frag. The ring profile is
// `sky_cubemap_data::sky_dome_profile` (the same points as skyConusVBO); colours are
// linear RGB and mix in authored (gamma) space, as the reference's vertex colours do.

// Linear RGB, top pole first; the last stop is the bottom pole (SkyFogColor again).
uniform vec3 sky_stops[7];
// Dome points top pole first: horizontal radius (x) and height above the eye (y).
uniform vec2 dome_points[7];
// World direction toward the sun, SunFogAngle, SunAngleBlend * SunFogStrength and
// SunFogColor (linear), as in retail_fog.gdshaderinc.
uniform vec3 fog_sun_direction = vec3(0.0, 1.0, 0.0);
uniform float fog_sun_angle = 1.0;
uniform float fog_sun_percentage;
uniform vec3 fog_sun_color;

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

// Where the view ray at elevation (sin, cos) crosses the face between two dome points,
// as a fraction from `upper` to `lower` (sky_cubemap_data::edge_fraction).
float edge_fraction(vec2 upper, vec2 lower, float s, float c) {
	vec2 d = lower - upper;
	float denominator = d.x * s - d.y * c;
	if (abs(denominator) <= 1.1920929e-7) {
		return 0.0;
	}
	return clamp((upper.y * c - upper.x * s) / denominator, 0.0, 1.0);
}

// Authored colour of the dome along `direction` (sky_band_at_elevation).
vec3 dome_color(vec3 direction) {
	float s = direction.y;
	float c = length(direction.xz);
	float elevation = atan(s, c);
	for (int index = 0; index < 6; index++) {
		vec2 upper = dome_points[index];
		vec2 lower = dome_points[index + 1];
		if (elevation > atan(lower.y, lower.x) || index == 5) {
			float t = edge_fraction(upper, lower, s, c);
			return mix(linear_to_gamma(sky_stops[index]), linear_to_gamma(sky_stops[index + 1]), t);
		}
	}
	return linear_to_gamma(sky_stops[6]);
}

void sky() {
	vec3 color = dome_color(EYEDIR);
	float n_dot_sun = clamp(dot(EYEDIR, fog_sun_direction) - fog_sun_angle, 0.0, 1.0);
	if (n_dot_sun > 0.0) {
		float scatter = n_dot_sun / (1.0 - fog_sun_angle);
		scatter = scatter * scatter * scatter;
		color = mix(color, linear_to_gamma(fog_sun_color), clamp(scatter * fog_sun_percentage, 0.0, 1.0));
	}
	COLOR = gamma_to_linear(color);
}
