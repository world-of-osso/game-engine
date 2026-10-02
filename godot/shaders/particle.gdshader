shader_type spatial;
// The binder swaps this line for the emitter's blend and depth state.
render_mode unshaded, fog_disabled, cull_disabled, depth_draw_never, blend_add;

// WebWowViewerCpp m2ParticleShader.frag.slang: textures combine in authored (gamma)
// space, so they are sampled without the sRGB hint.
uniform sampler2D texture_0 : filter_linear_mipmap, repeat_enable;
uniform sampler2D texture_1 : filter_linear_mipmap, repeat_enable;
uniform sampler2D texture_2 : filter_linear_mipmap, repeat_enable;
// Atlas cell size in UV units (1 / columns, 1 / rows).
uniform vec2 cell_size = vec2(1.0);
// Multitexture layer 1 and 2 UV scales (paramXTransform).
uniform vec2 layer_scale = vec2(1.0);
// 0 Particle_Mod, 1 2ColorTex_3AlphaTex, 2 3ColorTex_3AlphaTex.
uniform int pixel_shader = 0;
// EGxBlend of the particle blend type (ParticleBlendingModeToEGxBlendEnum).
uniform int gx_blend = 4;
uniform float alpha_test = 0.0039215689;

#include "res://shaders/retail_fog.gdshaderinc"

// Instance data: basis x/y are the quad half axes, basis z holds layer 1's UV offset
// and the placement fade; custom holds the atlas cell offset and layer 2's UV offset.
varying vec2 uv0;
varying vec2 uv1;
varying vec2 uv2;
varying float fade;
varying vec3 world_position;

vec3 gamma_to_linear(vec3 gamma_color) {
	vec3 low = gamma_color / 12.92;
	vec3 high = pow((max(gamma_color, vec3(0.0)) + 0.055) / 1.055, vec3(2.4));
	return mix(high, low, lessThanEqual(gamma_color, vec3(0.04045)));
}

void vertex() {
	// BuildQuadT3 corners: (-1, 1) samples (0, 0), (1, -1) samples (1, 1).
	vec2 corner = vec2(VERTEX.x * 0.5 + 0.5, 0.5 - VERTEX.y * 0.5);
	uv0 = INSTANCE_CUSTOM.xy + corner * cell_size;
	uv1 = corner * layer_scale.x + MODEL_MATRIX[2].xy;
	uv2 = corner * layer_scale.y + INSTANCE_CUSTOM.zw;
	// The quad lies in its xy plane (VERTEX.z = 0), so basis z never moves it.
	fade = MODEL_MATRIX[2].z;
	// Pools draw top-level, so MODEL_MATRIX is the instance's world basis.
	world_position = (MODEL_MATRIX * vec4(VERTEX.xy, 0.0, 1.0)).xyz;
}

void fragment() {
	vec4 tex = texture(texture_0, uv0);
	if (tex.a < alpha_test) {
		discard;
	}
	vec4 color = vec4(COLOR.rgb * tex.rgb, tex.a * COLOR.a);
	if (pixel_shader == 1) {
		vec4 modulated = tex * texture(texture_1, uv1);
		color = vec4(COLOR.rgb * modulated.rgb, modulated.a * texture(texture_2, uv2).a * COLOR.a);
	} else if (pixel_shader == 2) {
		vec4 modulated = tex * texture(texture_1, uv1) * texture(texture_2, uv2);
		color = vec4(COLOR.rgb * modulated.rgb, modulated.a * COLOR.a);
	}
	if (color.a < alpha_test) {
		discard;
	}
	// m2ParticleShader.frag.slang: makeFog2 with the particle's blend mode, which fogs
	// additive and modulating particles toward their identity colour.
	vec3 rgb = retail_fog(color.rgb, world_position, CAMERA_POSITION_WORLD, gx_blend);
	float alpha = color.a;
	// A fading placement fades its particles; modes that ignore source alpha scale
	// colour toward their identity instead.
	if (gx_blend == 10) {
		// NoAlphaAdd: ONE, ONE (drawn as SRC_ALPHA, ONE with alpha 1).
		rgb *= fade;
		alpha = 1.0;
	} else if (gx_blend == 4) {
		// Mod: DST_COLOR, ZERO.
		rgb = mix(vec3(1.0), rgb, fade);
		alpha = 1.0;
	} else if (gx_blend == 5) {
		// Mod2x: DST_COLOR, SRC_COLOR; doubled in authored space so 0.5 stays neutral.
		rgb = mix(vec3(0.5), rgb, fade) * 2.0;
		alpha = 1.0;
	} else if (gx_blend == 13) {
		// BlendAdd: ONE, ONE_MINUS_SRC_ALPHA over premultiplied output.
		rgb *= fade;
		alpha *= fade;
	} else if (gx_blend >= 2) {
		alpha *= fade;
	}
	ALBEDO = gamma_to_linear(rgb);
	ALPHA = alpha;
}
