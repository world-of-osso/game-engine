shader_type spatial;
render_mode ambient_light_disabled, fog_disabled, specular_disabled;
#include "res://shaders/m2_point_light.gdshaderinc"

// config: layer count, blend mode (0 layered, 1 weighted, 2 height-weighted), UV repeat.
uniform vec4 config = vec4(1.0, 0.0, 1.0, 0.0);
// layer_params: height scale, height offset, MCMT id (unused), overbright multiplier.
uniform vec4 layer_params_0 = vec4(1.0, 0.0, 0.0, 1.0);
uniform vec4 layer_params_1 = vec4(1.0, 0.0, 0.0, 1.0);
uniform vec4 layer_params_2 = vec4(1.0, 0.0, 0.0, 1.0);
uniform vec4 layer_params_3 = vec4(1.0, 0.0, 0.0, 1.0);
// animation_params.xy: UV velocity; .z: reflection multiplier.
uniform vec4 animation_params_0;
uniform vec4 animation_params_1;
uniform vec4 animation_params_2;
uniform vec4 animation_params_3;
uniform float animation_time;

uniform sampler2D ground_0 : source_color, repeat_enable;
uniform sampler2D ground_1 : source_color, repeat_enable;
uniform sampler2D ground_2 : source_color, repeat_enable;
uniform sampler2D ground_3 : source_color, repeat_enable;
uniform sampler2D height_0 : repeat_enable;
uniform sampler2D height_1 : repeat_enable;
uniform sampler2D height_2 : repeat_enable;
uniform sampler2D height_3 : repeat_enable;
// MCAL is packed linearly; sampling must never apply sRGB decoding.
uniform sampler2D alpha_packed : filter_linear, repeat_disable;
uniform samplerCube environment_map : source_color;

// RetailSceneLight, in authored (sRGB-byte / 255) space.
uniform vec3 ambient = vec3(1.0);
uniform vec3 horizon_ambient = vec3(1.0);
uniform vec3 ground_ambient = vec3(1.0);
uniform vec3 direct;
// World direction sunlight travels, not the direction towards the sun.
uniform vec3 sun_direction = vec3(0.0, -1.0, 0.0);
// Godot's fog is disabled: fog colour in linear space.
uniform vec3 fog_color;
// Retail legacy exponential fog: (start, end) in yards and density per yard.
uniform vec2 fog_range;
uniform float fog_density;
uniform float fog_opacity;
// 0 off, 1 retail legacy fog.
uniform int fog_mode = 0;

varying vec3 terrain_world_position;
varying vec3 terrain_diffuse;
varying float terrain_spec_blend;
varying vec3 terrain_world_normal;
varying vec3 terrain_view_direction;
varying vec3 terrain_camera_position;

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

vec4 weighted_weights(vec3 alpha) {
	return vec4(1.0 - clamp(alpha.r + alpha.g + alpha.b, 0.0, 1.0), alpha);
}

vec4 layered_weights(vec3 alpha, int layer_count) {
	vec4 weights = vec4(1.0, 0.0, 0.0, 0.0);
	if (layer_count > 1) {
		weights.x = 1.0 - alpha.r;
		weights.y = alpha.r;
	}
	if (layer_count > 2) {
		weights.xy *= 1.0 - alpha.g;
		weights.z = alpha.g;
	}
	if (layer_count > 3) {
		weights.xyz *= 1.0 - alpha.b;
		weights.w = alpha.b;
	}
	return weights;
}

vec4 height_weighted(vec4 weights, vec4 heights) {
	vec4 pct = weights * heights;
	float pct_max = max(max(pct.x, pct.y), max(pct.z, pct.w));
	vec4 kept = pct * (vec4(1.0) - clamp(vec4(pct_max) - pct, vec4(0.0), vec4(1.0)));
	float total = kept.x + kept.y + kept.z + kept.w;
	if (total > 1e-6) {
		return kept / total;
	}
	return weights;
}

vec3 retail_ambient(vec3 world_normal, float n_dot_l) {
	float n_dot_up = dot(world_normal, vec3(0.0, 1.0, 0.0));
	vec3 hemisphere;
	if (n_dot_up >= 0.0) {
		hemisphere = mix(horizon_ambient, ambient, n_dot_up);
	} else {
		hemisphere = mix(horizon_ambient, ground_ambient, -n_dot_up);
	}
	return mix(hemisphere * 0.7, hemisphere * 1.1, 0.5 + 0.5 * n_dot_l);
}

vec3 apply_retail_fog(vec3 color, vec3 world_position, vec3 camera_position) {
	float distance_to_camera = length(world_position - camera_position);
	float amount = 0.0;
	if (fog_mode == 1) {
		// commonFogFunctions.slang calculateLegacyFog, without height fog.
		float exp_fog = exp(-max(0.0, distance_to_camera - fog_range.x) * fog_density);
		float end_fade = clamp(1.42857146 * (1.0 - distance_to_camera / fog_range.y), 0.0, 1.0);
		amount = 1.0 - min(exp_fog, end_fade);
	}
	return mix(color, linear_to_gamma(fog_color), amount * fog_opacity);
}

void vertex() {
	terrain_world_position = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}

void fragment() {
	vec2 uv0 = UV + animation_params_0.xy * animation_time;
	vec2 uv1 = UV + animation_params_1.xy * animation_time;
	vec2 uv2 = UV + animation_params_2.xy * animation_time;
	vec2 uv3 = UV + animation_params_3.xy * animation_time;
	vec4 c0 = texture(ground_0, uv0 * config.z);
	vec4 c1 = texture(ground_1, uv1 * config.z);
	vec4 c2 = texture(ground_2, uv2 * config.z);
	vec4 c3 = texture(ground_3, uv3 * config.z);
	c0.rgb = linear_to_gamma(c0.rgb) * layer_params_0.w;
	c1.rgb = linear_to_gamma(c1.rgb) * layer_params_1.w;
	c2.rgb = linear_to_gamma(c2.rgb) * layer_params_2.w;
	c3.rgb = linear_to_gamma(c3.rgb) * layer_params_3.w;

	vec3 alpha = texture(alpha_packed, UV).rgb;
	vec4 weights = weighted_weights(alpha);
	if (int(config.y) == 0) {
		weights = layered_weights(alpha, int(config.x));
	} else if (int(config.y) == 2) {
		vec4 heights = vec4(
			texture(height_0, uv0 * config.z).a * layer_params_0.x + layer_params_0.y,
			texture(height_1, uv1 * config.z).a * layer_params_1.x + layer_params_1.y,
			texture(height_2, uv2 * config.z).a * layer_params_2.x + layer_params_2.y,
			texture(height_3, uv3 * config.z).a * layer_params_3.x + layer_params_3.y
		);
		weights = height_weighted(weights, heights);
	}
	vec4 blended = c0 * weights.x + c1 * weights.y + c2 * weights.z + c3 * weights.w;
	// Godot COLOR stores MCCV as byte/255; original in.color is byte/127.
	vec3 diffuse = blended.rgb * (COLOR.rgb * (255.0 / 127.0));
	vec3 world_normal = normalize((INV_VIEW_MATRIX * vec4(NORMAL, 0.0)).xyz);
	vec3 world_view = normalize(CAMERA_POSITION_WORLD - terrain_world_position);
	float reflection_weight = dot(weights, vec4(animation_params_0.z, animation_params_1.z, animation_params_2.z, animation_params_3.z));
	if (reflection_weight != 0.0) {
		vec3 reflection_dir = reflect(-world_view, world_normal);
		vec3 reflection = linear_to_gamma(texture(environment_map, reflection_dir).rgb);
		float fresnel = pow(1.0 - max(dot(world_normal, world_view), 0.0), 4.0);
		diffuse = mix(diffuse, reflection, clamp(reflection_weight * fresnel, 0.0, 1.0));
	}
	// The sole scene-owned directional light invokes light() even when direct=0.
	ALBEDO = vec3(1.0);
	ROUGHNESS = 1.0;
	// Pass authored operands through fragment -> light without applying gamma yet.
	EMISSION = vec3(0.0);
	terrain_diffuse = diffuse;
	terrain_spec_blend = blended.a;
	terrain_world_normal = world_normal;
	terrain_view_direction = world_view;
	terrain_camera_position = CAMERA_POSITION_WORLD;
}

void light() {
	if (LIGHT_IS_DIRECTIONAL) {
		vec3 normal = normalize(terrain_world_normal);
		float n_dot_l = clamp(dot(normal, -normalize(sun_direction)), 0.0, 1.0);
		vec3 authored = terrain_diffuse * (retail_ambient(normal, n_dot_l) + direct * n_dot_l * ATTENUATION);
		vec3 half_vector = -normalize(sun_direction - terrain_view_direction);
		float highlight = pow(max(0.0, dot(half_vector, normal)), 20.0);
		authored += terrain_spec_blend * direct * highlight * ATTENUATION;
		authored = apply_retail_fog(authored, terrain_world_position, terrain_camera_position);
		DIFFUSE_LIGHT += gamma_to_linear(authored);
	} else {
		// M2 point lights (WebWowViewerCpp pointLight.frag.slang:43-58); LIGHT_COLOR
		// carries Godot's PI.
		float n_dot_l = max(dot(NORMAL, LIGHT), 0.0);
		DIFFUSE_LIGHT += gamma_to_linear(terrain_diffuse) * LIGHT_COLOR / PI * m2_point_light_attenuation(ATTENUATION, SPECULAR_AMOUNT) * n_dot_l;
	}
}
