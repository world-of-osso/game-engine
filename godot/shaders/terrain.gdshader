shader_type spatial;
render_mode fog_disabled, specular_disabled;
#include "res://shaders/m2_point_light.gdshaderinc"

// One ADT tile: every MCNK chunk in one draw. UV2 carries each vertex's chunk (index_x,
// index_y); UV is the chunk's own 0..1 texture coordinate.
//
// chunk_data, texel (index_y * 16 + index_x, row):
//   row 0      config: layer count, blend mode (0 layered, 1 weighted, 2 height-weighted),
//              UV repeat
//   rows 1-4   layer_params per slot: height scale, height offset, MCMT id (unused),
//              overbright multiplier
//   rows 5-8   animation_params per slot: .xy UV velocity, .z reflection multiplier
//   row 9      ground texture of each slot, row 10 its height texture (see layer_texel)
uniform sampler2D chunk_data : filter_nearest, repeat_disable;
uniform float animation_time;

// Layer textures by size class (rust/src/terrain/tile_material.rs LayerArrays).
uniform sampler2DArray layers_0 : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
uniform sampler2DArray layers_1 : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
uniform sampler2DArray layers_2 : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
uniform sampler2DArray layers_3 : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
// Each chunk's 64x64 MCAL at (index_x, index_y) * 64, packed linearly: sampling must never
// apply sRGB decoding.
uniform sampler2D alpha_atlas : filter_linear, repeat_disable;
// R > 0.5: the chunk is rejected from the camera and only casts its shadow.
uniform sampler2D chunk_hidden : filter_nearest, repeat_disable;
uniform samplerCube environment_map : source_color;

#include "res://shaders/scene_light.gdshaderinc"
// Godot's fog is disabled: fog colour in linear space.
#include "res://shaders/retail_fog.gdshaderinc"

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
		hemisphere = mix(light_horizon_ambient(), light_ambient(), n_dot_up);
	} else {
		hemisphere = mix(light_horizon_ambient(), light_ground_ambient(), -n_dot_up);
	}
	return mix(hemisphere * 0.7, hemisphere * 1.1, 0.5 + 0.5 * n_dot_l);
}

vec3 apply_retail_fog(vec3 color, vec3 world_position, vec3 camera_position) {
	return retail_fog(color, world_position, camera_position, 0);
}

const float CHUNK_REF_WHITE = -1.0;
const float CHUNK_REF_UNTEXTURED = -2.0;
const float LAYER_CLASS_STRIDE = 4096.0;

varying flat ivec2 terrain_chunk;

// A texture reference of chunk_data rows 9-10: class * LAYER_CLASS_STRIDE + layer;
// CHUNK_REF_WHITE samples Godot's default white of an unbound sampler, and
// CHUNK_REF_UNTEXTURED the original renderer's one gray texel. The size class branches per
// pixel, so the anisotropic footprint comes from gradients taken outside them.
vec4 layer_texel(float ref, vec2 uv, vec2 uv_dx, vec2 uv_dy) {
	if (ref < -1.5) {
		return vec4(vec3(0.21586050011389926), 1.0);
	}
	if (ref < -0.5) {
		return vec4(1.0);
	}
	int size_class = int(ref / LAYER_CLASS_STRIDE);
	vec3 coord = vec3(uv, ref - float(size_class) * LAYER_CLASS_STRIDE);
	if (size_class == 0) {
		return textureGrad(layers_0, coord, uv_dx, uv_dy);
	} else if (size_class == 1) {
		return textureGrad(layers_1, coord, uv_dx, uv_dy);
	} else if (size_class == 2) {
		return textureGrad(layers_2, coord, uv_dx, uv_dy);
	}
	return textureGrad(layers_3, coord, uv_dx, uv_dy);
}

vec4 chunk_row(int row) {
	return texelFetch(chunk_data, ivec2(terrain_chunk.y * 16 + terrain_chunk.x, row), 0);
}

void vertex() {
	terrain_chunk = ivec2(round(UV2));
	// A camera-rejected chunk collapses to a point outside the shadow pass: no fragments.
	if (!IN_SHADOW_PASS && texelFetch(chunk_hidden, terrain_chunk, 0).r > 0.5) {
		VERTEX = vec3(0.0);
	}
	terrain_world_position = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}

void fragment() {
	vec4 config = chunk_row(0);
	vec4 layer_params_0 = chunk_row(1);
	vec4 layer_params_1 = chunk_row(2);
	vec4 layer_params_2 = chunk_row(3);
	vec4 layer_params_3 = chunk_row(4);
	vec4 animation_params_0 = chunk_row(5);
	vec4 animation_params_1 = chunk_row(6);
	vec4 animation_params_2 = chunk_row(7);
	vec4 animation_params_3 = chunk_row(8);
	vec4 ground = chunk_row(9);
	vec4 height = chunk_row(10);
	vec2 uv0 = UV + animation_params_0.xy * animation_time;
	vec2 uv1 = UV + animation_params_1.xy * animation_time;
	vec2 uv2 = UV + animation_params_2.xy * animation_time;
	vec2 uv3 = UV + animation_params_3.xy * animation_time;
	// Every layer scrolls by a constant: all share the derivatives of the repeated UV.
	vec2 uv_dx = dFdx(UV * config.z);
	vec2 uv_dy = dFdy(UV * config.z);
	vec4 c0 = layer_texel(ground.x, uv0 * config.z, uv_dx, uv_dy);
	vec4 c1 = layer_texel(ground.y, uv1 * config.z, uv_dx, uv_dy);
	vec4 c2 = layer_texel(ground.z, uv2 * config.z, uv_dx, uv_dy);
	vec4 c3 = layer_texel(ground.w, uv3 * config.z, uv_dx, uv_dy);
	c0.rgb = linear_to_gamma(c0.rgb) * layer_params_0.w;
	c1.rgb = linear_to_gamma(c1.rgb) * layer_params_1.w;
	c2.rgb = linear_to_gamma(c2.rgb) * layer_params_2.w;
	c3.rgb = linear_to_gamma(c3.rgb) * layer_params_3.w;

	// Clamping to the chunk's edge texel centres matches its own clamp-to-edge texture.
	vec2 alpha_texel = vec2(terrain_chunk) * 64.0 + clamp(UV * 64.0, vec2(0.5), vec2(63.5));
	vec3 alpha = texture(alpha_atlas, alpha_texel / 1024.0).rgb;
	vec4 weights = weighted_weights(alpha);
	if (int(config.y) == 0) {
		weights = layered_weights(alpha, int(config.x));
	} else if (int(config.y) == 2) {
		vec4 heights = vec4(
			layer_texel(height.x, uv0 * config.z, uv_dx, uv_dy).a * layer_params_0.x + layer_params_0.y,
			layer_texel(height.y, uv1 * config.z, uv_dx, uv_dy).a * layer_params_1.x + layer_params_1.y,
			layer_texel(height.z, uv2 * config.z, uv_dx, uv_dy).a * layer_params_2.x + layer_params_2.y,
			layer_texel(height.w, uv3 * config.z, uv_dx, uv_dy).a * layer_params_3.x + layer_params_3.y
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
	// Keep authored lighting, but do not compile out Godot's SSAO sampling.
	IRRADIANCE = vec4(0.0, 0.0, 0.0, 1.0);
	RADIANCE = vec4(0.0, 0.0, 0.0, 1.0);
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
		float n_dot_l = clamp(dot(normal, -normalize(light_sun_direction())), 0.0, 1.0);
		vec3 authored = terrain_diffuse * (retail_ambient(normal, n_dot_l) + light_direct() * n_dot_l * ATTENUATION);
		vec3 half_vector = -normalize(light_sun_direction() - terrain_view_direction);
		float highlight = pow(max(0.0, dot(half_vector, normal)), 20.0);
		authored += terrain_spec_blend * light_direct() * highlight * ATTENUATION;
		authored = apply_retail_fog(authored, terrain_world_position, terrain_camera_position);
		DIFFUSE_LIGHT += gamma_to_linear(authored);
	} else {
		// M2 point lights (WebWowViewerCpp pointLight.frag.slang:43-58); LIGHT_COLOR
		// carries Godot's PI.
		float n_dot_l = max(dot(NORMAL, LIGHT), 0.0);
		DIFFUSE_LIGHT += gamma_to_linear(terrain_diffuse) * LIGHT_COLOR / PI * m2_point_light_attenuation(ATTENUATION, SPECULAR_AMOUNT) * n_dot_l;
	}
}
