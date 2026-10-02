shader_type spatial;
// Terrain detail doodads (solarityclient detail.vert/detail.frag, the client's
// DetailDoodad.bls): two-sided, depth-writing, alpha-tested at 128.
render_mode unshaded, cull_disabled, depth_draw_opaque, fog_disabled, shadows_disabled;

uniform sampler2D detail_texture : source_color, filter_linear_mipmap, repeat_enable;
// RetailSceneLight, in authored (sRGB-byte / 255) space.
uniform vec3 ambient = vec3(1.0);
uniform vec3 direct;
// World direction sunlight travels, not the direction towards the sun.
uniform vec3 sun_direction = vec3(0.0, -1.0, 0.0);
// groundEffectDist: opacity ramps from 1 at 85% of it to 0 at it.
uniform float detail_distance = 140.0;
#include "res://shaders/retail_fog.gdshaderinc"

varying vec3 detail_world_position;
varying vec4 detail_color;
varying float detail_visibility;

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

void vertex() {
	detail_world_position = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	float depth = -(MODELVIEW_MATRIX * vec4(VERTEX, 1.0)).z;
	detail_visibility = clamp((detail_distance - depth) / (detail_distance * 0.15), 0.0, 1.0);
	// The terrain face normal, unrotated with the model: the whole card lights alike.
	vec3 normal = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
	float diffuse = clamp(dot(normal, -normalize(sun_direction)), 0.0, 1.0);
	// COLOR is the terrain tint (doubled MCCV, 255 white) and the MCSH visibility.
	detail_color = vec4(min(ambient + direct * diffuse, vec3(1.0)) * COLOR.rgb, COLOR.a);
}

void fragment() {
	vec4 sampled = texture(detail_texture, UV);
	if (sampled.a * detail_visibility < 128.0 / 255.0) {
		discard;
	}
	// Authored shadows darken the doodad by 30%.
	vec3 authored = linear_to_gamma(sampled.rgb) * detail_color.rgb * (detail_color.a * 0.3 + 0.7);
	authored = retail_fog(authored, detail_world_position, CAMERA_POSITION_WORLD, 0);
	ALBEDO = gamma_to_linear(authored);
}
