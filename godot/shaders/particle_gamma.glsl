#[compute]
#version 450
// The scene colour between linear light and authored (gamma) encoding around the
// particle composite (rendering/particle_composite_effect.gd). The extended sRGB
// curve continues past 1, so HDR values outside particles round-trip.
layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;
layout(set = 0, binding = 0, rgba16f) uniform restrict image2D scene_color;
layout(push_constant, std430) uniform Params {
	ivec2 size;
	int encode;
	int padding;
};

vec3 encode_srgb(vec3 x) {
	vec3 high = 1.055 * pow(max(x, vec3(0.0)), vec3(1.0 / 2.4)) - 0.055;
	return mix(high, x * 12.92, lessThanEqual(x, vec3(0.0031308)));
}

vec3 decode_srgb(vec3 x) {
	vec3 high = pow((max(x, vec3(0.0)) + 0.055) / 1.055, vec3(2.4));
	return mix(high, x / 12.92, lessThanEqual(x, vec3(0.04045)));
}

void main() {
	ivec2 pixel = ivec2(gl_GlobalInvocationID.xy);
	if (any(greaterThanEqual(pixel, size))) {
		return;
	}
	vec4 color = imageLoad(scene_color, pixel);
	color.rgb = encode == 1 ? encode_srgb(color.rgb) : decode_srgb(color.rgb);
	imageStore(scene_color, pixel, color);
}
