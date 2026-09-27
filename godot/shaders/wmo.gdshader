shader_type spatial;
// Material binding selects authored cull/blend variants; no Godot PBR shading or fog.
render_mode ambient_light_disabled, fog_disabled, specular_disabled, cull_back, blend_mix;

uniform sampler2D base_texture : source_color, repeat_enable;
uniform sampler2D second_texture : source_color, repeat_enable;
uniform vec4 base_color : source_color = vec4(1.0);
// StandardMaterial emissive arrives in linear space.
uniform vec3 emissive;
uniform vec3 interior_ambient;
uniform bool exterior_lit = false;
uniform bool unlit = false;
uniform bool unfogged = false;
uniform bool has_mocv = false;
uniform bool has_second_mocv = false;
uniform bool has_uv2 = false;
uniform int two_layer_shader = 0;
uniform int blend_mode = 0;

// RetailSceneLight in authored (sRGB byte / 255) space.
uniform vec3 ambient = vec3(1.0);
uniform vec3 horizon_ambient = vec3(1.0);
uniform vec3 ground_ambient = vec3(1.0);
uniform vec3 direct;
uniform vec3 sun_direction = vec3(0.0, -1.0, 0.0);
uniform vec3 fog_color;
uniform vec2 fog_range;
uniform float fog_opacity;
// 0 off, 1 linear, 2 exponential, 3 exponential squared.
uniform int fog_mode = 0;

varying vec3 world_position;
varying vec3 world_normal;
varying vec3 camera_position;
varying float second_mocv_alpha;
varying vec3 ambient_authored;
varying vec3 diffuse_authored;
varying vec3 emission_authored;
varying float exterior_weight;

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

vec3 retail_ambient(vec3 up_ambient, vec3 horizon, vec3 ground, vec3 normal, float n_dot_l) {
	float n_dot_up = dot(normal, vec3(0.0, 1.0, 0.0));
	vec3 hemisphere;
	if (n_dot_up >= 0.0) {
		hemisphere = mix(horizon, up_ambient, n_dot_up);
	} else {
		hemisphere = mix(horizon, ground, -n_dot_up);
	}
	return mix(hemisphere * 0.7, hemisphere * 1.1, 0.5 + 0.5 * n_dot_l);
}

vec3 retail_fog_color() {
	if (blend_mode == 3 || blend_mode == 10 || blend_mode == 13) {
		return vec3(0.0);
	} else if (blend_mode == 4) {
		return vec3(1.0);
	} else if (blend_mode == 5) {
		return vec3(0.5);
	}
	return linear_to_gamma(fog_color);
}

vec3 apply_retail_fog(vec3 color) {
	if (unfogged) {
		return color;
	}
	float distance_to_camera = length(world_position - camera_position);
	float amount = 0.0;
	if (fog_mode == 1) {
		amount = 1.0 - clamp((fog_range.y - distance_to_camera) / (fog_range.y - fog_range.x), 0.0, 1.0);
	} else if (fog_mode == 2) {
		amount = 1.0 - 1.0 / exp(distance_to_camera * fog_range.x);
	} else if (fog_mode == 3) {
		float scaled = distance_to_camera * fog_range.x;
		amount = 1.0 - 1.0 / exp(scaled * scaled);
	}
	return mix(color, retail_fog_color(), amount * fog_opacity);
}

vec4 wmo_texel(vec4 first, vec4 second, float mocv2_alpha) {
	vec3 first_gamma = linear_to_gamma(first.rgb);
	vec3 second_gamma = linear_to_gamma(second.rgb);
	if (two_layer_shader == 13) {
		return vec4(mix(second_gamma, first_gamma, mocv2_alpha), 1.0);
	}
	if (two_layer_shader == 6) {
		vec3 under = mix(first_gamma, second_gamma, second.a);
		return vec4(mix(under, first_gamma, mocv2_alpha), first.a);
	}
	return vec4(first_gamma, first.a);
}

void vertex() {
	world_position = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	second_mocv_alpha = has_second_mocv ? CUSTOM0.x : 1.0;
}

void fragment() {
	vec4 first = texture(base_texture, UV) * base_color;
	vec2 second_uv = has_uv2 ? UV2 : vec2(1.0);
	vec4 second = texture(second_texture, second_uv);
	vec4 texel = wmo_texel(first, second, second_mocv_alpha);
	if (blend_mode == 1 && texel.a < (128.0 / 255.0)) {
		discard;
	}
	ALPHA = (blend_mode == 2 || blend_mode == 3) ? texel.a : 1.0;
	ALBEDO = vec3(1.0);
	ROUGHNESS = 1.0;
	world_normal = normalize((INV_VIEW_MATRIX * vec4(NORMAL, 0.0)).xyz);
	camera_position = CAMERA_POSITION_WORLD;
	diffuse_authored = texel.rgb;
	emission_authored = linear_to_gamma(emissive);
	vec3 mocv = has_mocv ? COLOR.rgb * 2.0 : vec3(0.0);
	exterior_weight = exterior_lit ? 1.0 : (has_mocv ? COLOR.a : 0.0);
	float exterior_dot = clamp(dot(world_normal, -normalize(sun_direction)), 0.0, 1.0);
	// Retail's fixed interior sun only weights the ambients; it adds no direct light.
	vec3 interior_sun = normalize(vec3(-0.30822, -0.9, 0.30822));
	float interior_dot = clamp(dot(world_normal, -interior_sun), 0.0, 1.0);
	vec3 interior = retail_ambient(interior_ambient + mocv, interior_ambient + mocv, interior_ambient + mocv, world_normal, interior_dot);
	vec3 exterior = retail_ambient(ambient + mocv, horizon_ambient + mocv, ground_ambient + mocv, world_normal, exterior_dot);
	ambient_authored = diffuse_authored * mix(interior, exterior, exterior_weight) + emission_authored;
	if (unlit) {
		ambient_authored = diffuse_authored + emission_authored;
	}
	// Retain interior/ambient when no directional light exists. light() adds only the
	// gamma-encoded difference of shadowed direct, keeping the authored-space sum.
	EMISSION = gamma_to_linear(apply_retail_fog(ambient_authored));
}

void light() {
	if (LIGHT_IS_DIRECTIONAL && !unlit) {
		vec3 normal = normalize(world_normal);
		float n_dot_l = clamp(dot(normal, -normalize(sun_direction)), 0.0, 1.0);
		vec3 with_direct = ambient_authored + diffuse_authored * exterior_weight * direct * n_dot_l * ATTENUATION;
		vec3 lit = gamma_to_linear(apply_retail_fog(with_direct));
		vec3 base = gamma_to_linear(apply_retail_fog(ambient_authored));
		DIFFUSE_LIGHT += lit - base;
	}
}
