shader_type spatial;
render_mode unshaded, cull_disabled, blend_mix, depth_draw_never, fog_disabled;

// Retail azerite (LiquidMaterial 18): WebWowViewerCpp shaders/slang/common/liquid/
// liquidAzeriteMat.slang calcAzeritheLiquidMat. The v-names in comments are that function's
// locals. Maths runs in WoW world space (z up) and in authored colour (bytes / 255); the
// result is converted to linear.

// forwardLiquidShader_text.slang binds uTexture0/1/2/4/5 (v9-v13), uTexture6 = next frame of
// slot 5 (v14), uTexture7 = env map FDID 1797551 (v15), uTexture8 = foam FDID 1844666 (v16).
uniform sampler2D texture_0 : repeat_enable, filter_linear_mipmap;
uniform sampler2D texture_1 : repeat_enable, filter_linear_mipmap;
uniform sampler2D texture_2 : repeat_enable, filter_linear_mipmap;
uniform sampler2D texture_4 : repeat_enable, filter_linear_mipmap;
uniform sampler2D texture_5 : repeat_enable, filter_linear_mipmap;
uniform sampler2D texture_next : repeat_enable, filter_linear_mipmap;
uniform sampler2D environment_texture : repeat_enable, filter_linear_mipmap;
uniform sampler2D shore_foam : repeat_enable, filter_linear_mipmap;
uniform sampler2D depth_texture : hint_depth_texture, filter_nearest, repeat_disable;

// Shared material clock in milliseconds (the reference scene time).
uniform float animation_time_ms = 0.0;
// LiquidType.Float[0..15] (AzeritheData f0..f15).
uniform vec4 floats_0 = vec4(0.0);
uniform vec4 floats_4 = vec4(0.0);
uniform vec4 floats_8 = vec4(0.0);
uniform vec4 floats_12 = vec4(0.0);
// LiquidType.Color[0] (LiquidAzerithe.cpp color0.rgb, unswizzled).
uniform vec3 liquid_color_0 = vec3(0.0);
// Frame counts of slots 3-5; .z is color0.w, the slot-5 normal map frame count.
uniform vec3 frame_counts_3 = vec3(1.0);
// LiquidObject FlowSpeed and FlowDirection (degrees).
uniform vec2 flow = vec2(0.0);

// Scene light (lighting/mod.rs bind_water), authored RGB.
uniform vec3 ambient = vec3(1.0);
uniform vec3 direct = vec3(0.0);
// Godot world direction the sunlight travels.
uniform vec3 sun_direction = vec3(0.0, -1.0, 0.0);
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

// Unit normal from a two-channel normal map texel (v98-v101).
vec3 unpack_normal(vec2 texel) {
	vec2 n = texel * 2.0 - 1.0;
	return vec3(n, sqrt(1.0 - dot(n, n)));
}

void vertex() {
	wow_position = to_wow((MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz);
	// adt_water_geometry: MH2O vertex depth / 255 (vPositionDepth.w).
	vertex_depth = COLOR.a;
}

void fragment() {
	float time = animation_time_ms;  // v17
	float f8 = floats_8.x;
	float f9 = floats_8.y;
	float f11 = floats_8.w;
	float f12 = floats_12.x;
	float f14 = floats_12.z;
	float f15 = floats_12.w;

	// All four layers share one scroll: direction flow + f4, speed f5 + flow speed (v39-v42).
	float flow_angle = (flow.y + floats_4.x) * DEGREES;
	float flow_speed = floats_4.y + flow.x;
	vec2 scroll = vec2(
		scroll_component(time, cos(flow_angle) * flow_speed),
		scroll_component(time, sin(flow_angle) * flow_speed));
	// Exterior ADT liquid maps world position * 0.06 (LiquidInstance.cpp generateTexCoordsFromPos).
	vec2 uv = mesh_uv ? UV : wow_position.xy * 0.06;  // v7
	vec2 uv_f1 = uv * floats_0.y + scroll;  // v89
	vec2 uv_f3 = uv * floats_0.w + scroll;  // v90
	vec2 uv_f0 = uv * floats_0.x + scroll;  // v91
	vec2 uv_f2 = uv * floats_0.z + scroll;  // v92

	float shore_scale = f11 > 0.0 ? 1.0 / f11 : 1.0;  // v84.y
	float cycle = 0.0;  // v85
	if (f9 > 0.0) {
		cycle = mod(time, f9 * 1000.0) / (f9 * 1000.0);
	}
	// Crossfade of the slot-5 normal map frame k (texture_5) to k+1 (texture_next) (v86-v88).
	float frame_interval = f12 * 1000.0;
	float frame_count = frame_counts_3.z;
	float frame_blend = 0.0;
	if (frame_interval > 0.0 && frame_count > 1.0) {
		frame_blend = fract(frame_count * mod(time, frame_interval) / frame_interval);
	}

	float authored_depth = vertex_depth == 0.0 ? 0.0 : 1.0;  // v94
	float depth = clamp(vertex_depth, 0.0, 1.0);  // v95
	vec3 normal = mix(unpack_normal(texture(texture_5, uv_f0).xy), unpack_normal(texture(texture_next, uv_f0).xy), frame_blend);  // v102
	normal = vec3(0.0, 0.0, 1.0) + (normal - vec3(0.0, 0.0, 1.0)) * f15;
	if (length(normal) < 0.006) {
		normal.z += 0.001;
	}
	normal = normalize(normal);
	vec2 distortion = f8 * normal.xy;  // v103

	vec3 pulse_map = texture(texture_2, uv_f0 + distortion).rgb;  // v104
	vec4 layer_0 = texture(texture_0, uv_f0 + distortion);  // v107
	vec4 layer_4 = texture(texture_4, uv_f1 + distortion);  // v108
	vec4 layer_1 = texture(texture_1, uv_f2 + distortion);  // v109

	// Pulse window in the (start, end) = pulse_map.gb range of the cycle (v105-v117).
	float pulse_start = pulse_map.g;
	float pulse_end = pulse_map.b;
	if (pulse_end < pulse_start) {
		if (cycle < pulse_start) {
			cycle += 1.0;
		}
		pulse_end += 1.0;
	}
	float pulse_phase = clamp(cycle - pulse_start, 0.0, 1.0) / (pulse_end - pulse_start);  // v115
	float pulse = 0.0;  // v116
	if (!(pulse_phase > 1.0 || pulse_map.r > 1.0)) {
		pulse = clamp(pulse_map.r - abs(2.0 * pulse_phase - 1.0), 0.0, 1.0);
	}
	float weight_0 = clamp(1.0 - pulse / 0.5, 0.0, 1.0);  // v119
	float weight_4 = clamp(pulse / 0.5, 0.0, 1.0) - clamp((pulse - 0.5) / 0.25, 0.0, 1.0);  // v120
	float weight_1 = clamp((pulse - 0.5) / 0.25, 0.0, 1.0) - clamp((pulse - 0.75) / 0.25, 0.0, 1.0);  // v121
	float weight_glow = clamp((pulse - 0.75) / 0.25, 0.0, 1.0);  // v122
	float shallow = 1.0 - clamp((1.0 - depth) * f14, 0.0, 1.0);  // v123

	vec3 camera = to_wow(CAMERA_POSITION_WORLD);  // v125
	vec3 to_fragment = wow_position - camera;  // v126, unnormalised as in the reference
	vec3 to_sun = normalize(-to_wow(sun_direction));  // v127
	// uSunAttenuation.z is 0 without LightParams sun attenuation, so v132 = 1.
	float sun_attenuation = 1.0;
	vec3 light = direct * (clamp(dot(to_sun, normal), 0.0, 1.0) * sun_attenuation) + ambient;  // v141
	vec3 soft_light = light * 0.25 + vec3(0.75);  // v142

	vec3 color = layer_1.rgb * weight_1 + soft_light * layer_4.rgb * weight_4 + soft_light * layer_0.rgb * weight_0;  // v124
	// Sphere-mapped environment reflection (v143-v153).
	vec3 reflected = to_fragment - 2.0 * dot(normal, to_fragment) * normal;  // v146
	float sphere = 2.0 * sqrt(reflected.x * reflected.x + reflected.y * reflected.y + (reflected.z + 1.0) * (reflected.z + 1.0));
	vec2 environment_uv = reflected.xy / sphere + 0.5;
	color += clamp(texture(environment_texture, environment_uv).rgb, 0.0, 1.0) * weight_0 * layer_0.a;
	color += 2.0 * layer_1.rgb * weight_glow;
	color = color + (soft_light * liquid_color_0 - color) * shallow;

	// Shore foam over the first 1/f11 yards of liquid thickness (v149-v162).
	float thickness = abs(scene_view_depth(depth_texture, SCREEN_UV, INV_PROJECTION_MATRIX)) - abs(VERTEX.z);
	float shore = clamp(shore_scale * thickness, 0.0, 1.0);  // v151
	vec3 foam = texture(shore_foam, vec2(fract(uv_f3.x * 0.1), 1.0 - shore)).rgb;  // v153
	color = foam + (color - foam) * shore;
	float alpha = authored_depth * sqrt(shore);  // v155

	color = apply_retail_fog(color, VERTEX, INV_VIEW_MATRIX);
	ALBEDO = gamma_to_linear(max(color, vec3(0.0)));
	ALPHA = alpha;
}
