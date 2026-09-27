shader_type spatial;
// The binder creates source variants for the authored blend and cull state.
render_mode ambient_light_disabled, fog_disabled, specular_disabled, cull_back, blend_mix;

uniform int effect_mode = 0; // 0: StandardMaterial-style single, 1: two-texture combiner.
uniform sampler2D base_texture : source_color, repeat_enable;
uniform sampler2D second_texture : source_color, repeat_enable;
uniform vec4 base_color : source_color = vec4(1.0);
uniform int shader_id = 16;
uniform int uv_mode_1 = 0;
uniform int uv_mode_2 = 0;
uniform vec2 uv_offset_1;
uniform vec2 uv_offset_2;
uniform float transparency = 1.0;
uniform float alpha_test = 0.0;
uniform int render_flags = 0;
uniform int gx_blend = 0;

// RetailSceneLight, in authored sRGB-byte / 255 space.
uniform vec3 ambient = vec3(1.0);
uniform vec3 horizon_ambient = vec3(1.0);
uniform vec3 ground_ambient = vec3(1.0);
uniform vec3 direct;
uniform vec3 sun_direction = vec3(0.0, -1.0, 0.0);
uniform vec3 fog_color;
uniform vec2 fog_range;
uniform float fog_opacity;
uniform int fog_mode = 0;

varying vec3 world_position;
varying vec3 world_normal;
varying vec3 camera_position;
varying vec3 authored_color;

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

vec4 combine_textures(vec4 texture1, vec4 texture2, int id) {
	if (id == 0x4014) {
		return clamp(texture1 * texture2 * 2.0, vec4(0.0), vec4(1.0));
	} else if (id == 0x10) {
		return vec4(texture1.rgb * texture2.rgb, texture1.a);
	} else if (id == 0x11) {
		return texture1 * texture2;
	} else if (id == 0x4016) {
		return vec4(clamp(texture1.rgb * texture2.rgb * 2.0, vec3(0.0), vec3(1.0)), texture1.a);
	} else if (id == 0x8015 || id == 0x8002) {
		return vec4(texture1.rgb + texture2.rgb * texture2.a, 1.0);
	} else if (id == 0x8001) {
		return vec4(texture1.rgb * mix(texture2.rgb * 2.0, vec3(1.0), texture1.a), 1.0);
	} else if (id == 0x8003) {
		return vec4(texture1.rgb + texture2.rgb * texture2.a * texture1.a, 1.0);
	}
	// Original WGSL's explicit default branch is texture1.
	return texture1;
}

vec3 retail_ambient(vec3 normal, float n_dot_l) {
	float n_dot_up = dot(normal, vec3(0.0, 1.0, 0.0));
	vec3 hemisphere;
	if (n_dot_up >= 0.0) {
		hemisphere = mix(horizon_ambient, ambient, n_dot_up);
	} else {
		hemisphere = mix(horizon_ambient, ground_ambient, -n_dot_up);
	}
	return mix(hemisphere * 0.7, hemisphere * 1.1, 0.5 + 0.5 * n_dot_l);
}

vec3 retail_fog_color() {
	if (gx_blend == 3 || gx_blend == 10 || gx_blend == 13) {
		return vec3(0.0);
	} else if (gx_blend == 4) {
		return vec3(1.0);
	} else if (gx_blend == 5) {
		return vec3(0.5);
	}
	return linear_to_gamma(fog_color);
}

vec3 apply_retail_fog(vec3 color) {
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

void vertex() {
	world_position = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}

void fragment() {
	vec2 first_uv = (uv_mode_1 == 1 ? UV2 : UV) + uv_offset_1;
	vec4 first = texture(base_texture, first_uv);
	vec4 color;
	if (effect_mode == 1) {
		vec2 second_uv = (uv_mode_2 == 1 ? UV2 : UV) + uv_offset_2;
		vec4 second = texture(second_texture, second_uv);
		color = combine_textures(vec4(linear_to_gamma(first.rgb), first.a), vec4(linear_to_gamma(second.rgb), second.a), shader_id);
		color.a = clamp(color.a * transparency, 0.0, 1.0);
	} else {
		vec3 linear_diffuse = first.rgb * base_color.rgb * COLOR.rgb;
		color = vec4(linear_to_gamma(linear_diffuse), clamp(first.a * base_color.a * COLOR.a * transparency, 0.0, 1.0));
	}
	if (color.a < alpha_test) {
		discard;
	}
	ALBEDO = vec3(1.0);
	ROUGHNESS = 1.0;
	ALPHA = color.a;
	authored_color = color.rgb;
	world_normal = normalize((INV_VIEW_MATRIX * vec4(NORMAL, 0.0)).xyz);
	camera_position = CAMERA_POSITION_WORLD;
}

void light() {
	if (LIGHT_IS_DIRECTIONAL) {
		vec3 rgb = authored_color;
		if ((render_flags & 1) == 0) {
			vec3 normal = normalize(world_normal);
			float n_dot_l = clamp(dot(normal, -normalize(sun_direction)), 0.0, 1.0);
			rgb *= retail_ambient(normal, n_dot_l) + direct * n_dot_l * ATTENUATION;
		}
		if ((render_flags & 2) == 0) {
			rgb = apply_retail_fog(rgb);
		}
		DIFFUSE_LIGHT += gamma_to_linear(rgb);
	}
}
