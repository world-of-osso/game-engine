shader_type spatial;
// Direct port of assets/shaders/sky.wgsl: original ring-band interpolation,
// spherical two-layer clouds, halo/highlight and far reverse-Z output.
// Geometry/profile and CPU cloud map come from rendering/skybox, not Godot Sky.
render_mode unshaded, fog_disabled, cull_back, blend_mix, depth_draw_never, shadows_disabled;

uniform vec3 sky_top;
uniform vec3 sky_middle;
uniform vec3 sky_band1;
uniform vec3 sky_band2;
uniform vec3 sky_smog;
uniform vec3 sky_fog;
uniform vec3 sun_color;
uniform vec3 sun_halo_color;
uniform vec3 cloud_emissive_color;
uniform vec3 cloud_layer1_ambient_color;
uniform vec3 cloud_layer2_ambient_color;
uniform vec3 sun_direction;
uniform vec3 cloud_params;
// Generated Rgba8Unorm, NOT source_color (cloud samples are linear density).
uniform sampler2D cloud_texture : filter_linear, repeat_enable;
varying vec3 world_position;

vec3 sample_gradient(float band) {
	vec3 stops[7] = vec3[7](sky_top, sky_middle, sky_band1, sky_band2, sky_smog, sky_fog, sky_fog);
	float b = clamp(band, 0.0, 6.0);
	int i = min(int(floor(b)), 5);
	return mix(stops[i], stops[i + 1], b - float(i));
}

void vertex() {
	world_position = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	POSITION = PROJECTION_MATRIX * MODELVIEW_MATRIX * vec4(VERTEX, 1.0);
	POSITION.z = 0.0;
}

void fragment() {
	vec3 dir = normalize(world_position - CAMERA_POSITION_WORLD);
	vec3 color = sample_gradient(UV.y * 6.0);
	float elev = clamp(asin(dir.y) / (PI / 2.0), 0.0, 1.0);
	vec2 uv = vec2(atan(dir.z, dir.x) / (2.0 * PI) + 0.5 + cloud_params.y,
		0.5 - asin(dir.y) / PI + cloud_params.z);
	float cloud_a = texture(cloud_texture, uv).r;
	float cloud_b = texture(cloud_texture, uv * vec2(2.0, 1.35) + vec2(0.17, 0.29)).r;
	float cloud_shape = mix(cloud_a, cloud_b, 0.35);
	float density = clamp(cloud_params.x, 0.0, 1.0);
	float threshold = mix(0.92, 0.32, density);
	float horizon_mask = smoothstep(0.02, 0.18, elev) * (1.0 - smoothstep(0.82, 0.98, elev));
	float cloud_mask = smoothstep(threshold - 0.1, threshold + 0.1, cloud_shape) * horizon_mask;
	float sun_alignment = max(dot(dir, normalize(sun_direction)), 0.0);
	float halo = pow(sun_alignment, 18.0);
	float highlight = pow(sun_alignment, 36.0);
	vec3 cloud_ambient = mix(cloud_layer2_ambient_color, cloud_layer1_ambient_color,
		clamp(cloud_shape * 1.2, 0.0, 1.0));
	vec3 cloud_lighting = sun_halo_color * halo + sun_color * highlight;
	vec3 cloud_color = cloud_ambient + cloud_emissive_color + cloud_lighting;
	ALBEDO = mix(color, cloud_color, cloud_mask);
	// Opaque output uses Godot's opaque pass; priority applies within that pass.
	// No ALPHA assignment: do not promote the original opaque dome to transparency.
}
