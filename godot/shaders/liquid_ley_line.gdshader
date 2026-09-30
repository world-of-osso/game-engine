shader_type spatial;
render_mode unshaded, cull_disabled, blend_mix, depth_draw_never, fog_disabled;

// Retail ley line (LiquidMaterial 12): WebWowViewerCpp shaders/slang/common/liquid/
// liquidLeyLineMat.slang calcLeyLineLiquidMat. The v-names in comments are that function's
// locals. Maths runs in WoW world space (z up) and in authored colour (bytes / 255); the
// result is converted to linear.

// forwardLiquidShader_text.slang feeds v10..v13 from uTexture0, uTexture1, uTexture3, uTexture4:
// two tinted layers, the normal/bump slot and the foam slot.
uniform sampler2D texture_0 : repeat_enable, filter_linear_mipmap;
uniform sampler2D texture_1 : repeat_enable, filter_linear_mipmap;
uniform sampler2D texture_3 : repeat_enable, filter_linear_mipmap;
uniform sampler2D texture_4 : repeat_enable, filter_linear_mipmap;
uniform sampler2D screen_texture : hint_screen_texture, filter_linear, repeat_disable;
uniform sampler2D depth_texture : hint_depth_texture, filter_nearest, repeat_disable;

// Shared material clock in milliseconds (the reference scene time).
uniform float animation_time_ms = 0.0;
// LiquidType.Float[0..17] (LeyLineData f0..f17).
uniform vec4 floats_0 = vec4(0.0);
uniform vec4 floats_4 = vec4(0.0);
uniform vec4 floats_8 = vec4(0.0);
uniform vec4 floats_12 = vec4(0.0);
uniform vec2 floats_16 = vec2(0.0);
// LiquidType.Coefficient[0..3]: vertex depth -> alpha, a cubic (depthTable).
uniform vec4 depth_coefficients = vec4(0.0);
// LiquidType.Color[1]; LiquidLeyLine.cpp packs color1 as m_colors[1][2], [1], [0].
uniform vec3 liquid_color_1 = vec3(0.0);
// LiquidObject FlowSpeed and FlowDirection (degrees).
uniform vec2 flow = vec2(0.0);

// Underwater fog (start, end, density per yard) and colour.
uniform vec3 underwater_fog = vec3(0.0, 100000000.0, 0.0);
uniform vec3 underwater_fog_color = vec3(0.0);
#include "res://shaders/retail_fog.gdshaderinc"

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

// The reference's scale-and-scroll matrices (v29, v40, v67): scroll GetTexScrollMtx of a
// velocity at flow direction + `degrees` and flow speed + `speed`.
vec2 flow_scroll(float time, float degrees, float speed) {
	float angle = (flow.y + degrees) * DEGREES;
	float velocity = speed + flow.x;
	return vec2(scroll_component(time, cos(angle) * velocity), scroll_component(time, sin(angle) * velocity));
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

	// Exterior ADT liquid maps world position * 0.06 (LiquidInstance.cpp generateTexCoordsFromPos) (v7).
	vec2 uv = wow_position.xy * 0.06;
	vec2 bump_scroll = flow_scroll(time, f11, f12);  // v77
	vec4 v79 = vec4(uv / 32.0, -(uv + 0.5) / 32.0);
	vec4 v80 = v79 * 228.0;
	vec4 v81 = vec4((f10 * 32.0 * f13 * v79.xy + bump_scroll).yx, (f10 * 32.0 * f13 * v79.zw + bump_scroll).yx);
	v79 = vec4((f10 * v79.xy + bump_scroll).yx, (f10 * v79.zw + bump_scroll).yx);
	vec2 layer_uv_0 = f3 * uv + flow_scroll(time, f4, f5);  // v82
	vec2 layer_uv_1 = f6 * uv + flow_scroll(time, f7, f8);  // v83
	float authored_depth = vertex_depth == 0.0 ? 0.0 : 1.0;  // v85

	vec3 camera = to_wow(CAMERA_POSITION_WORLD);
	vec3 to_fragment = wow_position - camera;
	float distance_to_camera = length(to_fragment);  // v88
	vec3 view_direction = to_fragment / distance_to_camera;  // v89

	// Liquid thickness along the view axis from the opaque depth buffer (v91-v96).
	float liquid_depth = -VERTEX.z;
	float raw_depth = texture(depth_texture, SCREEN_UV).r;
	float thickness = raw_depth <= 0.0001 ? 100000.0
		: scene_view_depth(depth_texture, SCREEN_UV, INV_PROJECTION_MATRIX) - liquid_depth;
	float relative_thickness = thickness / max(liquid_depth, 0.0001);

	// Two bump pairs blended by distance, flattened by f15 (v103-v97).
	vec2 near_bump = (texture(texture_3, v79.xy).xy + texture(texture_3, v79.zw).xy - 1.0) * 2.0;
	vec2 far_bump = (texture(texture_3, v81.xy).xy + texture(texture_3, v81.zw).xy - 1.0) * 2.0;
	float bump_blend = clamp(0.2 + distance_to_camera / 1000.0, 0.0, 1.0);
	vec3 bump = vec3(0.7 * far_bump + (near_bump - 0.7 * far_bump) * bump_blend, 1.0);
	bump.xy *= f15;
	bump = normalize(bump);  // v97
	vec3 normal = normalize(vec3(bump.xy, 1.0));  // v90
	float facing = 1.0 - clamp(dot(-view_direction, normal), 0.0, 1.0);
	float fresnel = facing * facing;  // v108

	// Two scrolled layers multiplied together and tinted by Color[1] (v113-v115).
	vec2 distortion = abs(0.03 * bump.xy);
	vec4 layer_0 = texture(texture_0, layer_uv_0 + distortion);
	vec4 layer_1 = texture(texture_1, layer_uv_1 + distortion);
	vec3 glow = layer_0.rgb * layer_0.a * layer_1.rgb * layer_1.a * 4.0 + liquid_color_1.bgr;

	// Refracted opaque scene, kept only where the offset still lands behind the liquid (v116-v120).
	vec2 refracted_uv = SCREEN_UV + 0.1 * normal.xy * clamp(100.0 * relative_thickness * (1.2 - fresnel), 0.0, 1.0);
	float refracted_depth = scene_view_depth(depth_texture, refracted_uv, INV_PROJECTION_MATRIX);
	bool behind = refracted_depth > liquid_depth;
	refracted_uv = behind ? refracted_uv : SCREEN_UV;
	vec3 below = linear_to_gamma(texture(screen_texture, refracted_uv).rgb);
	float below_thickness = behind ? refracted_depth - liquid_depth : thickness;

	// Underwater fog over the refracted scene, then half the glow on top (v125-v132).
	float fog_distance = max(0.0, below_thickness - underwater_fog.x);
	float fog_visibility = min(1.0 / exp(fog_distance * underwater_fog.z),
		clamp((1.0 / 0.7) * (1.0 - below_thickness / underwater_fog.y), 0.0, 1.0));
	vec3 color = underwater_fog_color + (below - underwater_fog_color) * fog_visibility;
	color = color + (glow - color) * 0.5;

	// Shore: alpha rises and foam fades over the first yards of thickness (v134-v138).
	float shore = clamp(fresnel * 0.5 + 0.2, 0.0, 1.0);
	float alpha = smoothstep(0.0, 1.2 * shore, thickness);
	float foam_fade = 1.0 - smoothstep(0.0, 2.5 * shore, thickness * (1.0 - view_direction.z));
	vec3 foam = texture(texture_4, v80.xy - normal.xy).rgb + texture(texture_4, v80.zw + 0.2 * normal.yx).rgb;
	color += foam * (f14 * foam_fade * (1.1 - clamp(distance_to_camera / 150.0, 0.0, 1.0)) * normal.z);

	// Vertex-depth cubic, pulled toward 1 by (1 - f9) (v139-v146).
	float d = vertex_depth;
	float depth_alpha = clamp(depth_coefficients.x + depth_coefficients.y * d + depth_coefficients.z * d * d + depth_coefficients.w * d * d * d, 0.0, 1.0);
	depth_alpha = (1.0 - (1.0 - depth_alpha) * f9) * authored_depth;
	alpha *= depth_alpha;

	color = apply_retail_fog(color, VERTEX, INV_VIEW_MATRIX);
	ALBEDO = gamma_to_linear(max(color, vec3(0.0)));
	ALPHA = alpha;
}
