shader_type spatial;
render_mode unshaded, cull_disabled, fog_disabled;

// Retail magma and slime (LiquidMaterial 2/4): WebWowViewerCpp shaders/slang/common/liquid/
// liquidMagmaMat.slang calcMagmaLiquidMat. The v-names in comments are that function's
// locals. Opaque; texel maths in authored colour (bytes / 255), result converted to linear.

// LiquidTypeXTexture slots 1-3 and the 16-slice noise volume (FDID 768431) as a 128x2048 atlas.
uniform sampler2D texture_1 : repeat_enable, filter_linear_mipmap;
uniform sampler2D texture_2 : repeat_enable, filter_linear_mipmap;
uniform sampler2D texture_3 : repeat_enable, filter_linear_mipmap;
uniform sampler2D noise_volume : repeat_enable, filter_linear;

uniform float animation_time_ms = 0.0;
// LiquidType.Float[0..11] (MagmaData f0..f11).
uniform vec4 floats_0 = vec4(0.0);
uniform vec4 floats_4 = vec4(0.0);
uniform vec4 floats_8 = vec4(0.0);
// LiquidType.Int[0..3]; Int[2] is the base colour (LiquidMagma.cpp colorRGB_pad).
uniform ivec4 liquid_ints = ivec4(0);
// LiquidObject FlowSpeed and FlowDirection (degrees).
uniform vec2 flow = vec2(0.0);

#include "res://shaders/retail_fog.gdshaderinc"

const float DEGREES = 0.017453292;

// WMO liquids carry their texture coordinates (wmoGroupGeom.cpp:566-575).
uniform bool mesh_uv = false;
varying vec3 wow_position;

vec3 linear_to_gamma(vec3 color) {
	return mix(12.92 * color, 1.055 * pow(color, vec3(1.0 / 2.4)) - 0.055, step(vec3(0.0031308), color));
}

vec3 gamma_to_linear(vec3 color) {
	return mix(color / 12.92, pow((color + 0.055) / 1.055, vec3(2.4)), step(vec3(0.04045), color));
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

vec2 scroll(float time, vec2 speed) {
	return vec2(scroll_component(time, speed.x), scroll_component(time, speed.y));
}

// The reference's rotation-scale matrix mat4(c*s, s*s | -s*s, c*s) applied to `uv`.
vec2 rotate_scale(vec2 uv, float degrees, float scale) {
	float c = cos(degrees * DEGREES);
	float s = sin(degrees * DEGREES);
	return mat2(vec2(c * scale, s * scale), vec2(-s * scale, c * scale)) * uv;
}

// Two slices of the noise atlas at `slice_phase` (0..16) blended by its fraction (v73, v84).
vec3 noise_at(vec2 uv, float slice_phase) {
	float slice = floor(slice_phase);
	float next = mod(slice + 1.0, 16.0);
	vec3 first = texture(noise_volume, vec2(uv.x, (uv.y + slice) * 0.0625)).rgb;
	vec3 second = texture(noise_volume, vec2(uv.x, (uv.y + next) * 0.0625)).rgb;
	return first + (second - first) * (slice_phase - slice);
}

// LiquidMagma.cpp: Int[2] is a packed RGB with a 0x01000000 bias.
vec3 base_color() {
	int packed = liquid_ints.z;
	if (packed >= 0x01000000) {
		packed -= 0x01000000;
	}
	return vec3(float((packed >> 16) & 0xFF), float((packed >> 8) & 0xFF), float(packed & 0xFF)) / 255.0;
}

// makeFog2 on the liquid's view-space fragment (liquid materials blend as mode 2).
vec3 apply_retail_fog(vec3 color, vec3 view_position, mat4 inv_view) {
	return retail_fog(color, (inv_view * vec4(view_position, 1.0)).xyz, inv_view[3].xyz, 2);
}

void vertex() {
	vec3 world = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	wow_position = vec3(world.x, -world.z, world.y);
}

void fragment() {
	float time = animation_time_ms;
	// Exterior ADT liquid maps world position * 0.06 (LiquidInstance.cpp generateTexCoordsFromPos).
	vec2 uv = mesh_uv ? UV : wow_position.xy * 0.06;
	float flow_angle = flow.y * DEGREES;
	vec2 flow_speed = vec2((floats_0.x + flow.x) * cos(flow_angle), (floats_0.y + flow.x) * sin(flow_angle));  // v35
	vec2 flow_scroll = scroll(time, flow_speed);  // v36
	vec2 half_scroll = scroll(time, flow_speed * 0.5);  // v42

	vec2 base_uv = uv + flow_scroll;  // v60
	float wobble = ((cos(base_uv.x) + sin(base_uv.y)) + (cos(base_uv.x * 2.0) + sin(base_uv.y * 2.0))
		+ (cos(base_uv.x * 3.0) + sin(base_uv.y * 3.0))) * 0.5 * 0.3 * 0.5 + 0.5;  // v64
	vec2 noise_uv_0 = rotate_scale(uv, floats_0.w, floats_0.z * 0.61699998) + half_scroll;  // v65
	vec2 noise_uv_1 = rotate_scale(uv, floats_0.w, floats_0.z * 0.47400001) + half_scroll;  // v66
	vec2 layer_uv_1 = rotate_scale(uv, floats_4.w, floats_4.x) + flow_scroll;  // v67
	vec2 layer_uv_2 = rotate_scale(uv, floats_8.x, floats_4.y) + flow_scroll;  // v68
	vec2 layer_uv_3 = rotate_scale(uv, floats_8.y, floats_4.z) + flow_scroll;  // v69

	float slice_phase = mod(time, 600000.0) * 0.0000016666667 * 16.0;  // v77
	vec3 blend = mix(noise_at(noise_uv_0, slice_phase), noise_at(noise_uv_1, slice_phase), wobble);  // v95

	vec4 layer_1 = texture(texture_1, layer_uv_1);
	vec4 layer_2 = texture(texture_2, layer_uv_2);
	vec4 layer_3 = texture(texture_3, layer_uv_3);
	vec3 color = base_color();  // v99
	color = color + (layer_1.rgb - color) * blend.x;
	color = color + (layer_2.rgb - color) * (layer_2.a * blend.y);
	color = color + (layer_3.rgb - color) * (layer_3.a * blend.z);

	color = apply_retail_fog(color, VERTEX, INV_VIEW_MATRIX);
	ALBEDO = gamma_to_linear(color);
}
