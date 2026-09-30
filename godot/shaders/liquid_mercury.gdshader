shader_type spatial;
render_mode unshaded, cull_disabled, blend_mix, depth_draw_never, fog_disabled;

// Retail mercury (LiquidMaterial 5): WebWowViewerCpp shaders/slang/common/liquid/
// liquidMercuryMat.slang calcMercuryLiquidMat. The v-names in comments are that function's
// locals. A chrome-like surface: the scene reflected along the bumped normal, desaturated and
// fresnel-lit. Maths runs in WoW world space (z up) and in authored colour (sRGB bytes / 255);
// the result is converted to linear.

// LiquidTypeXTexture slot 2 (normal/bump), the reference's uTexture2.
uniform sampler2D texture_2 : repeat_enable, filter_linear_mipmap;
uniform sampler2D screen_texture : hint_screen_texture, filter_linear, repeat_disable;
uniform sampler2D depth_texture : hint_depth_texture, filter_nearest, repeat_disable;

// Shared material clock in milliseconds (the reference scene time).
uniform float animation_time_ms = 0.0;
// LiquidType.Float[0..15] (MercuryData f0..f15; LiquidMercury.cpp packs them in order).
uniform vec4 floats_0 = vec4(0.0);
uniform vec4 floats_4 = vec4(0.0);
uniform vec4 floats_12 = vec4(0.0);
// Godot world direction the sunlight travels.
uniform vec3 sun_direction = vec3(0.0, -1.0, 0.0);
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

vec3 to_godot(vec3 wow) {
	return vec3(wow.x, wow.z, -wow.y);
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

// The reference's rotation-scale matrix mat4(c*s, s*s | -s*s, c*s) applied to `uv`.
vec2 rotate_scale(vec2 uv, float degrees, float scale) {
	float c = cos(degrees * DEGREES);
	float s = sin(degrees * DEGREES);
	return mat2(vec2(c * scale, s * scale), vec2(-s * scale, c * scale)) * uv;
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
	float f3 = floats_0.w;
	float f4 = floats_4.x;
	float f5 = floats_4.y;
	float f6 = floats_4.z;   // v57, bump strength
	float f14 = floats_12.z; // v34.y, normal tilt

	// Texture coordinates: exterior ADT liquid maps world position * 0.06 (LiquidInstance.cpp
	// generateTexCoordsFromPos), rotated/scaled by f5/f3/f4 and scrolled at a fixed 0.01 (v13-v39).
	vec2 uv = wow_position.xy * 0.06;
	vec2 scroll = vec2(scroll_component(time, 0.01), 0.0);  // v23
	vec2 uv0 = rotate_scale(uv / 32.0, f5, f3);
	vec2 uv1 = rotate_scale(-(uv + 0.5) / 32.0, f5, f4);
	vec4 v39 = vec4((16.0 * uv0 + scroll).yx, (32.0 * uv1 + scroll).yx);
	vec4 v37 = vec4((uv0 + scroll).yx, (uv1 + scroll).yx);

	vec3 camera = to_wow(CAMERA_POSITION_WORLD);  // v42
	vec3 to_fragment = wow_position - camera;  // v43
	float distance_to_camera = length(to_fragment);  // v44
	vec3 view_direction = to_fragment / distance_to_camera;  // v45

	// Liquid thickness along the view axis from the opaque depth buffer (v48-v50).
	float liquid_depth = -VERTEX.z;
	float raw_depth = texture(depth_texture, SCREEN_UV).r;
	float thickness = raw_depth <= 0.0001 ? 100000.0
		: scene_view_depth(depth_texture, SCREEN_UV, INV_PROJECTION_MATRIX) - liquid_depth;

	// Two scrolled bump pairs blended by distance, scaled by f6, tilting the normal by f14 (v58-v46).
	vec2 near_bump = (texture(texture_2, v37.xy).xy + texture(texture_2, v37.zw).xy - 1.0) * 2.0;  // v58
	vec2 far_bump = (texture(texture_2, v39.xy).xy + texture(texture_2, v39.zw).xy - 1.0) * 2.0;  // v59
	float bump_blend = clamp(0.2 + distance_to_camera / 1000.0, 0.0, 1.0);
	vec3 bump = vec3(0.7 * far_bump + (near_bump - 0.7 * far_bump) * bump_blend, 1.0);
	bump.xy *= f6;
	bump = normalize(bump);  // v52
	vec3 normal = normalize(vec3(f14 * bump.xy, 1.0));  // v46
	float facing = 1.0 - clamp(dot(-view_direction, normal), 0.0, 1.0);  // v66
	float fresnel = facing * facing;  // v63

	// Shore: alpha rises over the first yards of thickness; the normal bends toward the view
	// ray where the liquid is shallow (v68, v69).
	float shore_width = clamp(fresnel * 0.5 + 0.5, 0.0, 1.0);
	float alpha = smoothstep(0.0, 1.2 * shore_width, thickness);  // v68
	float shallow = 1.0 - smoothstep(0.0, 2.0 * shore_width, thickness * (1.0 - view_direction.z));  // v69
	normal = normalize(normal + shallow * view_direction);

	// Scene reflected 128 yd along the reflection ray, faded at the screen edges and for steep
	// rays, over the fog colour (v76-v70). reflectionParams.w is 1, so the normal is unscaled.
	vec3 reflected = reflect(view_direction, normal);  // v76
	// v77 is v43 + v76 * 128 (camera-relative) projected by uPMatrix * uLookAtMat, whose view
	// matrix includes the camera translation; the absolute point keeps the intended screen position.
	vec3 reflected_point = wow_position + reflected * 128.0;  // v77
	vec4 clip = PROJECTION_MATRIX * VIEW_MATRIX * vec4(to_godot(reflected_point), 1.0);
	vec2 ndc = clip.xy / clip.w;  // v78
	vec3 reflection = linear_to_gamma(texture(screen_texture, ndc * 0.5 + 0.5).rgb);  // v80
	float steepness = (1.0 / 0.75) * clamp(0.75 - reflected.z * reflected.z, 0.0, 1.0);  // v81
	float edge = clamp(1.0 - abs(ndc.x * ndc.x * ndc.x * ndc.x), 0.0, 1.0)
		* clamp(1.0 - ndc.y * ndc.y * ndc.y * ndc.y, 0.0, 1.0) * steepness;  // v82
	vec3 fog_gamma = linear_to_gamma(fog_color);  // v83
	vec3 mirror = fog_gamma + (reflection - fog_gamma) * clamp(edge, 0.0, 1.0);  // v70
	float luminance = dot(mirror, vec3(0.3, 0.6, 0.1));  // v85

	fresnel = fresnel * fresnel;
	vec3 color = 0.5 * clamp(fresnel + 0.9, 0.0, 1.0) * mix(mirror, vec3(luminance), 0.3);  // v86
	color += 0.5 * fresnel * mix(mirror, vec3(luminance), 0.7);

	vec3 to_sun = normalize(-to_wow(sun_direction));  // v87
	// uSunAttenuation.z is 0 without LightParams sun attenuation, so v92 = 1.
	float sun_attenuation = 1.0;
	vec3 halfway = normalize(to_sun - view_direction);  // v102
	float highlight = pow(clamp(dot(halfway, normal), 0.0, 1.0), SPECULAR_POWER) * sun_attenuation;  // v104
	color += 0.2 + 0.3 * (highlight + highlight * mirror) * fresnel + 0.3 * luminance * fresnel;

	alpha *= vertex_depth == 0.0 ? 0.0 : 1.0;  // v41
	color = apply_retail_fog(color, VERTEX, INV_VIEW_MATRIX);
	ALBEDO = gamma_to_linear(max(color, vec3(0.0)));
	ALPHA = alpha;
}
