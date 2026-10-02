shader_type spatial;
render_mode unshaded, fog_disabled, cull_back, blend_mix, depth_draw_never, shadows_disabled;

// Original sky uploads mip0 only; keep shared native mips but never sample them here.
uniform sampler2D base_texture : source_color, filter_linear, repeat_enable;
uniform sampler2D second_texture : source_color, filter_linear, repeat_enable;
uniform sampler2D third_texture : source_color, filter_linear, repeat_enable;
uniform sampler2D fourth_texture : source_color, filter_linear, repeat_enable;
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
	// source_color samples are already linear, matching the original WGSL stages.
	vec4 color = first;
	if (has_second_texture) {
		vec2 second_uv = (uv_mode_2 == 1 ? UV2 : UV) + uv_offset_2;
		vec4 second = texture(second_texture, second_uv);
		vec4 third = second;
		vec4 fourth = third;
		if (has_third_texture) {
			third = texture(third_texture, uv_mode_3 == 1 ? UV2 : UV);
		}
		if (has_fourth_texture) {
			fourth = texture(fourth_texture, uv_mode_4 == 1 ? UV2 : UV);
		}
		color = combine_textures(color, second, third, fourth);
	}
	color *= base_color;
	color.a = clamp(color.a * transparency * model_alpha, 0.0, 1.0);
	if (color.a < alpha_test) {
		discard;
	}
	ALBEDO = color.rgb;
	ALPHA = color.a;
}
