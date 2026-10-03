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
uniform float animation_time_ms = 0.0;
// LiquidType.Float[0..17] (FelData f0..f17).
uniform vec4 floats_0 = vec4(0.0);
uniform vec4 floats_4 = vec4(0.0);
uniform vec4 floats_8 = vec4(0.0);
uniform vec4 floats_12 = vec4(0.0);
uniform vec2 floats_16 = vec2(0.0);
// LiquidType.Coefficient[0..3] (FelData depthTable).
uniform vec4 depth_coefficients = vec4(0.0);
// LiquidType.Int[0..3]; LiquidFel.cpp derives FelData intParams from Int[1..3].
uniform ivec4 liquid_ints = ivec4(0);
// Frame counts of slots 3-5; .z is the normal map's (FelData intParams.w).
uniform vec3 frame_counts_3 = vec3(1.0);
// LiquidObject FlowSpeed and FlowDirection (degrees).
uniform vec2 flow = vec2(0.0);

#include "res://shaders/scene_light.gdshaderinc"
uniform vec3 specular_color = vec3(0.0);
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
	if (speed > 0.00000001 && speed < 1000.0) {
		int period = int(1000.0 / speed);
		return float(int(time) % period) / float(period);
	}
	if (speed < -0.00000001 && speed > -1000.0) {
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

// commonLiquidFunctions.slang sceneViewDepth: view distance of the depth buffer at `uv`.
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
	float time = animation_time_ms;
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
	float f11 = floats_8.w;
	float f12 = floats_12.x;
	float f13 = floats_12.y;
	float f14 = floats_12.z;
	float f15 = floats_12.w;
	float f16 = floats_16.x;
	float f17 = floats_16.y;
	// LiquidFel.cpp intParams: hasDepth, offset, blend, normal-map frame count.
	float has_depth = liquid_ints.y != 0 ? 1.0 : 0.0;  // v94
	float int_offset = float(liquid_ints.z) * 0.01;  // v36
	float int_blend = float(liquid_ints.w) * 0.01;  // v37
	float normal_frames = frame_counts_3.z;  // v98

	// Exterior ADT liquid maps world position * 0.06 (LiquidInstance.cpp generateTexCoordsFromPos).
	vec2 uv = wow_position.xy * 0.06;  // v7
	float flow_speed = flow.x;  // v17
	float flow_angle = flow.y * DEGREES;  // v71
	vec2 uv_0 = f0 * uv + scroll_along(time, (flow.y + f1) * DEGREES, f2 * flow_speed);  // v100
	vec2 uv_1 = f3 * uv + scroll_along(time, (flow.y + f4) * DEGREES, f5 * flow_speed);  // v101
	vec2 uv_2 = f6 * uv + scroll_along(time, (flow.y + f7) * DEGREES, f8 * flow_speed);  // v102
	float glow_speed = f10 * flow_speed;  // v72
	vec2 uv_3 = f9 * (int_blend + 1.0) * uv + scroll_along(time, flow_angle, glow_speed);  // v103
	float reverse_angle = flow_angle + PI;  // v76 (Godot built-in PI)
	vec2 uv_4 = f9 * (1.0 - int_blend) * uv + scroll_along(time, reverse_angle, glow_speed)
		+ vec2(sin(reverse_angle), cos(reverse_angle)) * int_offset;  // v104

	// Normal-map crossfade fraction between frame k (slot 5) and k+1 (v97-v99).
	float interval = f12 * 1000.0;
	float frame_blend = 0.0;
	if (interval > 0.0 && normal_frames > 1.0) {
		frame_blend = fract(normal_frames * mod(time, interval) / interval);
	}

	float d = vertex_depth;
	float depth_mix = clamp(depth_coefficients.x + depth_coefficients.y * d + depth_coefficients.z * d * d
		+ depth_coefficients.w * d * d * d, 0.0, 1.0);  // v107

	vec3 normal = mix(unpack_normal(texture(texture_5, uv_2).xy), unpack_normal(texture(texture_next, uv_2).xy), frame_blend);
	normal = 2.0 * normal - 1.0;
	normal = mix(normal, vec3(0.0, 0.0, 1.0), 1.0 - depth_mix * f16);  // v119
	vec2 distortion = f15 * normal.xy;  // v122

	vec3 layer_0 = texture(texture_0, uv_0 + distortion).rgb;  // v123
	vec3 layer_1 = texture(texture_1, uv_1 + distortion).rgb;  // v124
	float layer_mix = 1.0 - depth_mix * f14;  // v126
	if (has_depth != 0.0) {
		layer_mix = clamp(layer_mix, 0.0, 1.0);
	}
	vec3 base = mix(layer_0, layer_1, layer_mix);  // v127

	vec3 to_sun = normalize(-to_wow(light_sun_direction()));  // v129
	// uSunAttenuation.z is 0 without LightParams sun attenuation, so v134 = 1.
	float sun_attenuation = 1.0;
	float n_dot_l = clamp(dot(to_sun, normal), 0.0, 1.0);  // v139
	vec3 light = light_direct() * (n_dot_l * sun_attenuation) + light_ambient();  // v143
	vec3 color = mix(light * base, base, f13);  // v146

	vec3 camera = to_wow(CAMERA_POSITION_WORLD);  // v128
	vec3 to_camera = -normalize(wow_position - camera);  // v151
	vec3 halfway = normalize(to_sun + to_camera);  // v153
	float highlight = pow(clamp(dot(halfway, normal), 0.0, 1.0), SPECULAR_POWER) * sun_attenuation;  // v155
	vec4 specular_mask = texture(texture_2, uv + distortion);  // v156
	color += specular_color * specular_mask.rgb * highlight * specular_mask.a;  // v157

	vec4 glow_0 = texture(texture_4, uv_3 + distortion);  // v158
	vec4 glow_1 = texture(texture_4, uv_4 + distortion);  // v159
	color += (glow_0.rgb + glow_1.rgb) * glow_0.a * glow_1.a * (depth_mix * f17);  // v161

	// Shore gradient from the opaque depth behind the surface (v163-v167).
	float edge_scale = f11 > 0.0 ? 1.0 / f11 : 1.0;  // v95
	float fragment_depth = -VERTEX.z;  // abs(vPositionDepth.z)
	float scene_depth = abs(scene_view_depth(depth_texture, SCREEN_UV, INV_PROJECTION_MATRIX));  // v163
	float edge = clamp(edge_scale * (scene_depth - fragment_depth), 0.0, 1.0);  // v165
	vec4 shore = texture(texture_3, vec2(uv_1.x, 1.0 - edge));  // v167
	color = mix(shore.rgb, color, edge * shore.a);
	// v106/v168 (authored depth * shore alpha) only feed makeFog2's alpha; the output alpha is 1.

	color = apply_retail_fog(color, VERTEX, INV_VIEW_MATRIX);
	ALBEDO = gamma_to_linear(max(color, vec3(0.0)));
}
