shader_type spatial;
render_mode unshaded, cull_disabled, fog_disabled;

// Retail fel (LiquidMaterial 13): WebWowViewerCpp shaders/slang/common/liquid/
// liquidFelMat.slang calcFelLiquidMat. The v-names in comments are that function's locals.
// Opaque (the reference returns alpha 1). Maths runs in WoW world space (z up) and in
// authored colour (bytes / 255); the result is converted to linear.

// LiquidTypeXTexture slots 0-5 (uTexture0..5) and the next frame of the animated normal
// slot 5 (uTexture6, LiquidFel.cpp resolveAnimatedTextures).
uniform sampler2D texture_0 : repeat_enable, filter_linear_mipmap;
uniform sampler2D texture_1 : repeat_enable, filter_linear_mipmap;
uniform sampler2D texture_2 : repeat_enable, filter_linear_mipmap;
uniform sampler2D texture_3 : repeat_enable, filter_linear_mipmap;
uniform sampler2D texture_4 : repeat_enable, filter_linear_mipmap;
uniform sampler2D texture_5 : repeat_enable, filter_linear_mipmap;
uniform sampler2D texture_next : repeat_enable, filter_linear_mipmap;
uniform sampler2D depth_texture : hint_depth_texture, filter_nearest, repeat_disable;

// Shared material clock in milliseconds (the reference scene time).
global uniform float liquid_animation_time_ms;
// LiquidType.Float[0..17] (FelData f0..f17).
uniform vec4 floats_0 = vec4(0.0);
uniform vec4 floats_4 = vec4(0.0);
uniform vec4 floats_8 = vec4(0.0);
uniform vec4 floats_12 = vec4(0.0);
uniform vec2 floats_16 = vec2(0.0);
// Retail70688 G8 per-draw constant block, bound from the LiquidType row:
//   fel_params_0 = cb1[12] = {Float_13, 1/Float_11, Float_14, Float_15}
//   fel_params_1 = cb1[13] = {frame-pair blend, Float_16, Float_17, 0}
// 1/Float_11 is 1 when Float_11 is not positive; the reciprocal is computed here.
uniform vec4 fel_params_0 = vec4(1.0, 0.0, 1.0, 1.0);
uniform vec4 fel_params_1 = vec4(0.0, 0.0, 0.0, 0.0);
// LiquidType.Coefficient[0..3] (FelData depthTable).
uniform vec4 depth_coefficients = vec4(0.0);
// LiquidType.Int[0..3]; LiquidFel.cpp derives FelData intParams from Int[1..3].
uniform ivec4 liquid_ints = ivec4(0);
// Frame counts of slots 3-5; .z is the normal map's (FelData intParams.w).
uniform vec3 frame_counts_3 = vec3(1.0);
// Retail70688 G4: two base layers; third UV block/emissive/procedural at>=1,
// shore at>=2, specular at>=3. Shore blend/UV assignment awaits the shader evidence.
uniform int liquid_detail = 2;
// LiquidObject FlowSpeed and FlowDirection (degrees). Missing row means both zero.
uniform vec2 flow = vec2(0.0);

#include "res://shaders/scene_light.gdshaderinc"
global uniform vec3 specular_color;
#include "res://shaders/retail_fog.gdshaderinc"

// MapSceneRenderer.cpp:204 binds uExteriorSpecularColor.a, the specular power, as 1.0.
const float SPECULAR_POWER = 1.0;
const float DEGREES = 0.017453292;

varying vec3 wow_position;
varying float vertex_depth;

vec3 linear_to_gamma(vec3 color) {
	return mix(12.92 * color, 1.055 * pow(color, vec3(1.0 / 2.4)) - 0.055, step(vec3(0.0031308), color));
}

vec3 gamma_to_linear(vec3 color) {
	return mix(color / 12.92, pow((color + 0.055) / 1.055, vec3(2.4)), step(vec3(0.04045), color));
}

vec3 to_wow(vec3 godot) {
	return vec3(godot.x, -godot.z, godot.y);
}

// commonLiquidStructs.slang GetTexScrollComponent: fraction of the scroll cycle at `time` ms.
float scroll_component(float time, float speed) {
	if (speed > 0.00001 && speed < 1000.0) {
		int period = int(1000.0 / speed);
		return float(int(time) % period) / float(period);
	}
	if (speed < -0.00001 && speed > -1000.0) {
		int period = int(-1000.0 / speed);
		return 1.0 - float(int(time) % period) / float(period);
	}
	return 0.0;
}

// GetTexScrollMtx of a flow `speed` along `radians`.
vec2 scroll_along(float time, float radians, float speed) {
	return vec2(scroll_component(time, cos(radians) * speed), scroll_component(time, sin(radians) * speed));
}

// Normal-map texel -> (x, y, sqrt(1 - x^2 - y^2)) on the raw 0..1 values (v117, v118).
vec3 unpack_normal(vec2 texel) {
	return vec3(texel, sqrt(clamp(1.0 - dot(texel, texel), 0.0, 1.0)));
}

// commonLiquidFunctions.slang sceneViewDepth: linear view-space distance of the depth
// buffer at `uv`, in the same unit as the fragment's own -VERTEX.z.
float scene_view_depth(sampler2D depth, vec2 uv, mat4 inverse_projection) {
	float raw = texture(depth, uv).r;
	vec4 view = inverse_projection * vec4(uv * 2.0 - 1.0, raw, 1.0);
	return -view.z / view.w;
}

// makeFog2 on the liquid's view-space fragment (liquid materials blend as mode 2).
vec3 apply_retail_fog(vec3 color, vec3 view_position, mat4 inv_view) {
	return retail_fog(color, (inv_view * vec4(view_position, 1.0)).xyz, inv_view[3].xyz, 2);
}

void vertex() {
	wow_position = to_wow((MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz);
	// adt_water_geometry: MH2O vertex depth / 255 (vPositionDepth.w).
	vertex_depth = COLOR.a;
}

void fragment() {
	float time = liquid_animation_time_ms;
	float f0 = floats_0.x;
	float f1 = floats_0.y;
	float f2 = floats_0.z;
	float f3 = floats_0.w;
	float f4 = floats_4.x;
	float f5 = floats_4.y;
	float f6 = floats_4.z;
	float f7 = floats_4.w;
	float f8 = floats_8.x;
	float f9 = floats_8.y;
	float f10 = floats_8.z;
	// cb1[12]/cb1[13] (G8): shore scale, tint blend, layer mix, distortion, pair blend,
	// normal flattening and glow strength all come from the row, not from literals.
	float lit_blend = fel_params_0.x;
	float shore_scale = fel_params_0.y;
	float layer_mix_scale = fel_params_0.z;
	float distortion_scale = fel_params_0.w;
	float pair_blend = fel_params_1.x;
	float normal_flatten = fel_params_1.y;
	float glow_strength = fel_params_1.z;
	// LiquidFel.cpp intParams: hasDepth, offset, blend, normal-map frame count.
	float has_depth = liquid_ints.y != 0 ? 1.0 : 0.0;  // v94
	float int_offset = float(liquid_ints.z) * 0.01;  // v36
	float int_blend = float(liquid_ints.w) * 0.01;  // v37

	// Exterior ADT liquid maps world position * 0.06 (LiquidInstance.cpp generateTexCoordsFromPos).
	vec2 uv = wow_position.xy * 0.06;  // v7
	// G4: (scale, phase degrees, velocity) triples. Phase orients translation,
	// never the source UV; a missing/zero-speed LiquidObject produces no scroll.
	float flow_angle = flow.y * DEGREES;
	vec2 uv_0 = f0 * uv + scroll_along(time, (f1 + flow.y) * DEGREES, f2 * flow.x);
	vec2 uv_1 = f3 * uv + scroll_along(time, (f4 + flow.y) * DEGREES, f5 * flow.x);
	vec2 uv_2 = f6 * uv + scroll_along(time, (f7 + flow.y) * DEGREES, f8 * flow.x);
	float glow_speed = f10 * flow.x;
	vec2 uv_3 = f9 * (int_blend + 1.0) * uv + scroll_along(time, flow_angle, glow_speed);  // v103
	float reverse_angle = flow_angle + PI;  // v76 (Godot built-in PI)
	vec2 uv_4 = f9 * (1.0 - int_blend) * uv + scroll_along(time, reverse_angle, glow_speed)
		+ vec2(sin(reverse_angle), cos(reverse_angle)) * int_offset;  // v104

	// E5 corrects E1: Fel's group lookup period is zero, so frame zero is fixed and the
	// cb1[13].x pair blend the row supplies is zero as well.
	float frame_blend = pair_blend;

	float d = vertex_depth;
	float depth_mix = clamp(depth_coefficients.x + depth_coefficients.y * d + depth_coefficients.z * d * d
		+ depth_coefficients.w * d * d * d, 0.0, 1.0);  // v107

	vec3 normal = vec3(0.0, 0.0, 1.0);
	vec2 distortion = vec2(0.0);
	if (liquid_detail >= 1) {
		normal = mix(unpack_normal(texture(texture_5, uv_2).xy), unpack_normal(texture(texture_next, uv_2).xy), frame_blend);
		normal = 2.0 * normal - 1.0;
		normal = mix(normal, vec3(0.0, 0.0, 1.0), 1.0 - depth_mix * normal_flatten);
		distortion = distortion_scale * normal.xy;
	}

	vec3 layer_0 = texture(texture_0, uv_0 + distortion).rgb;  // v123
	vec3 layer_1 = texture(texture_1, uv_1 + distortion).rgb;  // v124
	float layer_mix = 1.0 - depth_mix * layer_mix_scale;  // v126
	if (has_depth != 0.0) {
		layer_mix = clamp(layer_mix, 0.0, 1.0);
	}
	vec3 base = mix(layer_0, layer_1, layer_mix);  // v127

	vec3 to_sun = normalize(-to_wow(light_sun_direction()));  // v129
	// uSunAttenuation.z is 0 without LightParams sun attenuation, so v134 = 1.
	float sun_attenuation = 1.0;
	float n_dot_l = clamp(dot(to_sun, normal), 0.0, 1.0);  // v139
	vec3 light = light_direct() * (n_dot_l * sun_attenuation) + light_ambient();  // v143
	vec3 color = mix(light * base, base, lit_blend);  // v146

	vec3 camera = to_wow(CAMERA_POSITION_WORLD);  // v128
	vec3 to_camera = -normalize(wow_position - camera);  // v151
	vec3 halfway = normalize(to_sun + to_camera);  // v153
	float highlight = pow(clamp(dot(halfway, normal), 0.0, 1.0), SPECULAR_POWER) * sun_attenuation;  // v155
	if (liquid_detail >= 3) {
		vec4 specular_mask = texture(texture_2, uv + distortion);
		color += specular_color * specular_mask.rgb * highlight * specular_mask.a;
	}
	if (liquid_detail >= 1) {
		vec4 glow_0 = texture(texture_4, uv_3 + distortion);
		vec4 glow_1 = texture(texture_4, uv_4 + distortion);
		color += (glow_0.rgb + glow_1.rgb) * glow_0.a * glow_1.a * (depth_mix * glow_strength);
	}

	// G8 shore: D is saturate((scene depth - liquid surface depth) / Float_11); both
	// depths are linear view-space yards, so Float_11 is the fade width in yards (2.6 for
	// type 1192). D = 0 is the shoreline and D = 1 is the open surface. The BLS samples
	// t6 through the draw item's UV rectangle; its CPU binding is open, so the linearised
	// screen depth buffer supplies the same quantity.
	float surface_depth = -VERTEX.z;
	float scene_depth = scene_view_depth(depth_texture, SCREEN_UV, INV_PROJECTION_MATRIX);
	float shore_fade = clamp(shore_scale * (scene_depth - surface_depth), 0.0, 1.0);
	if (liquid_detail >= 2) {
		// t5 at (TC1.z, 1 - D): TC1.z is the second base layer's U, as G8/BLS specify.
		vec4 shore = texture(texture_3, vec2(uv_1.x, 1.0 - shore_fade));
		color = shore.rgb + (color - shore.rgb) * (shore_fade * shore.a);
	}
	// Opaque material: the shore mix happens in-shader (G8 §4). The t16 screen-space
	// composite is out of scope. GLOW_0/1 also ignore the authored depth term that only
	// feeds makeFog2's alpha in the reference, which is inert for an opaque output.

	color = apply_retail_fog(color, VERTEX, INV_VIEW_MATRIX);
	ALBEDO = gamma_to_linear(max(color, vec3(0.0)));
}
