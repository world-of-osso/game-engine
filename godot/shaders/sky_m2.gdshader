shader_type spatial;
render_mode unshaded, fog_disabled, cull_back, blend_mix, depth_draw_never, shadows_disabled;

uniform sampler2D base_texture : source_color, repeat_enable;
uniform sampler2D second_texture : source_color, repeat_enable;
uniform sampler2D third_texture : source_color, repeat_enable;
uniform sampler2D fourth_texture : source_color, repeat_enable;
uniform vec4 base_color : source_color = vec4(1.0);
uniform float transparency = 1.0;
// The model alpha (M2Object::setAlpha): stars' night brightness, skybox blend weights.
uniform float model_alpha = 1.0;
uniform float alpha_test = 0.0;
uniform int combine_mode = 0;
uniform bool has_second_texture = false;
uniform bool has_third_texture = false;
uniform bool has_fourth_texture = false;
uniform int uv_mode_1 = 0;
uniform int uv_mode_2 = 0;
uniform int uv_mode_3 = 0;
uniform int uv_mode_4 = 0;
uniform vec2 uv_offset_1;
uniform vec2 uv_offset_2;

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

vec4 combine_textures(vec4 first, vec4 second, vec4 third, vec4 fourth) {
	if (combine_mode == 0xE || combine_mode == 0x11) {
		return first * second;
	} else if (combine_mode == 0x4014) {
		return clamp(first * second * 2.0, vec4(0.0), vec4(1.0));
	} else if (combine_mode == 0x10) {
		return vec4(first.rgb * second.rgb, first.a);
	} else if (combine_mode == 0x4016) {
		return vec4(clamp(first.rgb * second.rgb * 2.0, vec3(0.0), vec3(1.0)), first.a);
	} else if (combine_mode == 0x8015 || combine_mode == 0x8002) {
		return vec4(first.rgb + second.rgb * second.a, 1.0);
	} else if (combine_mode == 0x8001) {
		return vec4(first.rgb * mix(second.rgb * 2.0, vec3(1.0), first.a), 1.0);
	} else if (combine_mode == 0x8003) {
		return vec4(first.rgb + second.rgb * second.a * first.a, 1.0);
	} else if (combine_mode == 0x8012) {
		return first * mix(second, third, third.a);
	} else if (combine_mode == 0x8016) {
		return first * mix(second, third, fourth.a);
	}
	return first;
}

void vertex() {
	// Reverse-Z far plane. Authored sky stays behind terrain without writing scene depth.
	POSITION = PROJECTION_MATRIX * MODELVIEW_MATRIX * vec4(VERTEX, 1.0);
	POSITION.z = POSITION.w * 0.000001;
}

void fragment() {
	vec2 first_uv = (uv_mode_1 == 1 ? UV2 : UV) + uv_offset_1;
	vec4 first = texture(base_texture, first_uv);
	vec4 color = vec4(linear_to_gamma(first.rgb), first.a);
	if (has_second_texture) {
		vec2 second_uv = (uv_mode_2 == 1 ? UV2 : UV) + uv_offset_2;
		vec4 second = texture(second_texture, second_uv);
		second.rgb = linear_to_gamma(second.rgb);
		vec4 third = second;
		vec4 fourth = third;
		if (has_third_texture) {
			third = texture(third_texture, uv_mode_3 == 1 ? UV2 : UV);
			third.rgb = linear_to_gamma(third.rgb);
		}
		if (has_fourth_texture) {
			fourth = texture(fourth_texture, uv_mode_4 == 1 ? UV2 : UV);
			fourth.rgb = linear_to_gamma(fourth.rgb);
		}
		color = combine_textures(color, second, third, fourth);
	}
	color *= base_color;
	color.a = clamp(color.a * transparency * model_alpha, 0.0, 1.0);
	if (color.a < alpha_test) {
		discard;
	}
	ALBEDO = gamma_to_linear(color.rgb);
	ALPHA = color.a;
}
