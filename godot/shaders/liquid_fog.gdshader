shader_type spatial;
render_mode unshaded, cull_disabled, blend_mix, depth_draw_never, fog_disabled;

// Retail fog liquid (LiquidMaterial 10): WebWowViewerCpp shaders/slang/common/liquid/
// liquidFogMat.slang calcLiquidFogMat. The v-names in comments are that function's locals.
// Maths runs in WoW world space (z up) and in authored colour; the result is converted to linear.

// LiquidTypeXTexture slot 0 (the fog density texture, reference uTexture0).
uniform sampler2D texture_0 : repeat_enable, filter_linear_mipmap;
uniform sampler2D depth_texture : hint_depth_texture, filter_nearest, repeat_disable;

// Shared material clock in milliseconds (the reference scene time).
uniform float animation_time_ms = 0.0;
// LiquidType.Float[0..11] (FogData f0_f1_f2_f3, f4_f5_f6_f7, f8_f9_f10_f11).
uniform vec4 floats_0 = vec4(0.0);
uniform vec4 floats_4 = vec4(0.0);
uniform vec4 floats_8 = vec4(0.0);
// LiquidType.Color[0]; LiquidFog.cpp packs it as fogColor with alpha 1.
uniform vec3 liquid_color_0 = vec3(0.0);

#include "res://shaders/retail_fog.gdshaderinc"

const float DEGREES = 0.017453292;

// WMO liquids carry their texture coordinates (wmoGroupGeom.cpp:566-575).
uniform bool mesh_uv = false;
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
	float f0 = floats_0.x;
	float f1 = floats_0.y;
	float f3 = floats_0.w;  // v16
	float f4 = floats_4.x;  // v17
	float f5 = floats_4.y;  // v18
	float f9 = floats_8.y;  // v19
	float f10 = floats_8.z;  // v20

	// Exterior ADT liquid maps world position * 0.06 (LiquidInstance.cpp
	// generateTexCoordsFromPos); two rotated/scaled layers scrolled along u by f9/f10 (v27).
	vec2 uv = mesh_uv ? UV : wow_position.xy * 0.06;
	vec2 uv_0 = rotate_scale(uv, f5, f3) + vec2(scroll_component(time, f9), 0.0);
	vec2 uv_1 = rotate_scale(uv + 0.5, f5, f4) + vec2(scroll_component(time, f10), 0.0);
	vec4 layer_0 = texture(texture_0, uv_0);  // v28
	vec4 layer_1 = texture(texture_0, uv_1);  // v29

	float authored_depth = vertex_depth == 0.0 ? 0.0 : 1.0;  // v31
	vec3 camera = to_wow(CAMERA_POSITION_WORLD);  // v32
	vec3 to_fragment = wow_position - camera;  // v33
	float distance_to_camera = length(to_fragment);

	// Fog thickness along the view axis from the opaque depth buffer (v34-v36).
	float fog_view_depth = -VERTEX.z;  // v35
	float raw_depth = texture(depth_texture, SCREEN_UV).r;
	float thickness = raw_depth <= 0.0001 ? 100000.0
		: abs(scene_view_depth(depth_texture, SCREEN_UV, INV_PROJECTION_MATRIX)) - fog_view_depth;
	vec3 through = to_fragment / distance_to_camera * thickness;  // v37

	// Height-attenuated exponential density modulated by the texture alphas (v40-v49).
	float density = clamp(layer_0.a * layer_1.a + layer_1.a, 0.0, 1.0);  // v40
	float path = length(through) * exp(-f0 * camera.z);  // v46
	if (through.z > 0.01) {
		path = clamp(path, 0.0, density * 2.0);
	}
	float transmittance = clamp(exp(-(f1 * density) * path), 0.0, 1.0);  // v38
	float alpha = (1.0 - transmittance) * authored_depth;  // v49

	vec3 color = liquid_color_0;  // v50, fogColor.rgb * fogColor.a with a = 1
	color = apply_retail_fog(color, VERTEX, INV_VIEW_MATRIX);
	ALBEDO = gamma_to_linear(color);
	ALPHA = alpha;
}
