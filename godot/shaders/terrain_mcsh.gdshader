shader_type spatial;
render_mode unshaded, fog_disabled, specular_disabled, depth_draw_never, blend_mix;
#include "res://shaders/scene_light.gdshaderinc"

uniform sampler2D mcsh_mask : hint_default_black, filter_nearest, repeat_disable;
// Caller supplies the historical world flag word and m_colors[2].r operand.
// Modern index2 mapping and runtime flag ownership remain documented gaps.
uniform int world_shadow_enable_bits = 0;
uniform float shadow_alpha = 0.0;

vec3 authored_to_linear(vec3 color) {
	return mix(pow((color + 0.055) / 1.055, vec3(2.4)), color / 12.92,
		lessThanEqual(color, vec3(0.04045)));
}

void fragment() {
	if ((world_shadow_enable_bits & 16) == 0) {
		discard;
	}
	float coverage = texture(mcsh_mask, UV).r;
	if (coverage == 0.0 || shadow_alpha == 0.0) {
		discard;
	}
	// User-supplied clean-room evidence: exact unsigned8-bit input arithmetic.
	uvec3 ambient_bytes = uvec3(round(clamp(light_ambient(), vec3(0.0), vec3(1.0)) * 255.0));
	uvec3 shadow_bytes = (uvec3(85u) * (ambient_bytes + uvec3(3u))) >> uvec3(8u);
	ALBEDO = authored_to_linear(vec3(shadow_bytes) / 255.0);
	ALPHA = shadow_alpha * coverage;
}
