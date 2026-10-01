shader_type spatial;
// Original ground.rs: white albedo, FDID 187126, 100x100 plane, UV x20.
// Original retail_m2.wgsl + retail_light_data::shade: authored-space lighting,
// no specular, no point lights or shadows in this standalone reference scene.
// Original debug DistanceFog: radial camera distance, Linear 15..45,
// authored color (0.18,0.2,0.23). Not Godot exponential/depth fog.
// This preserves the requested initial debug fog, not global sky systems'
// subsequent LightData fog overwrite. See environment.rs for that parity limit.
render_mode unshaded, fog_disabled, cull_back, shadows_disabled;

uniform sampler2D base_texture : source_color, repeat_enable, filter_linear_mipmap;
uniform bool linear_fog_enabled;
uniform vec3 ambient;
uniform vec3 horizon_ambient;
uniform vec3 ground_ambient;
uniform vec3 direct;
uniform vec3 sun_direction;
varying vec3 world_position;
varying vec3 world_normal;

vec3 linear_to_gamma(vec3 color) {
	vec3 low = color * 12.92;
	vec3 high = 1.055 * pow(max(color, vec3(0.0)), vec3(1.0 / 2.4)) - 0.055;
	return mix(high, low, lessThanEqual(color, vec3(0.0031308)));
}

vec3 gamma_to_linear(vec3 color) {
	vec3 low = color / 12.92;
	vec3 high = pow((max(color, vec3(0.0)) + 0.055) / 1.055, vec3(2.4));
	return mix(high, low, lessThanEqual(color, vec3(0.04045)));
}

void vertex() {
	world_position = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	world_normal = normalize(MODEL_NORMAL_MATRIX * NORMAL);
}

void fragment() {
	vec3 diffuse = linear_to_gamma(texture(base_texture, UV * 20.0).rgb);
	vec3 normal = normalize(world_normal);
	float n_dot_l = clamp(dot(normal, -normalize(sun_direction)), 0.0, 1.0);
	float n_dot_up = dot(normal, vec3(0.0, 1.0, 0.0));
	vec3 hemisphere = n_dot_up >= 0.0 ? mix(horizon_ambient, ambient, n_dot_up)
		: mix(horizon_ambient, ground_ambient, -n_dot_up);
	vec3 ambient_light = mix(hemisphere * 0.7, hemisphere * 1.1, 0.5 + 0.5 * n_dot_l);
	vec3 color = diffuse * (ambient_light + direct * n_dot_l);
	if (linear_fog_enabled) {
		float distance_from_camera = length(world_position - CAMERA_POSITION_WORLD);
		float fog_weight = clamp((distance_from_camera - 15.0) / (45.0 - 15.0), 0.0, 1.0);
		color = mix(color, vec3(0.18, 0.2, 0.23), fog_weight);
	}
	ALBEDO = gamma_to_linear(color);
}
