shader_type spatial;
render_mode unshaded, cull_disabled, blend_mix, depth_draw_never, fog_disabled;

// Retail water (LiquidMaterial 1/3): WebWowViewerCpp shaders/slang/common/liquid/
// liquidWaterMat.slang calcWaterLiquidMat, a port of the retail pixel shader. The v-names
// in comments are that function's locals. Maths runs in WoW world space (z up) and in
// authored colour (sRGB bytes / 255, as the terrain shader); the result is converted to linear.

// LiquidTypeXTexture slot 2 (normal/bump) and slot 3 (foam) of the LiquidType.
uniform sampler2D texture_2 : repeat_enable, filter_linear_mipmap;
uniform sampler2D texture_3 : repeat_enable, filter_linear_mipmap;
uniform sampler2D screen_texture : hint_screen_texture, filter_linear, repeat_disable;
uniform sampler2D depth_texture : hint_depth_texture, filter_nearest, repeat_disable;

// Shared material clock in milliseconds (the reference scene time).
uniform float animation_time_ms = 0.0;
// LiquidType.Float[0..17] (WaterData f0..f17).
uniform vec4 floats_0 = vec4(0.0);
uniform vec4 floats_4 = vec4(0.0);
uniform vec4 floats_8 = vec4(0.0);
uniform vec4 floats_12 = vec4(0.0);
uniform vec2 floats_16 = vec2(0.0);
// LiquidType.Coefficient[0..3]: vertex depth -> close/far mix, a cubic.
uniform vec4 depth_coefficients = vec4(0.0);
// LiquidWater.cpp CalculateWavePeriod of the two wave scales.
uniform vec2 wave_periods = vec2(0.0);
// LiquidObject FlowSpeed and FlowDirection (degrees).
uniform vec2 flow = vec2(0.0);
// 0 ocean, 1 river, 2 WMO colours (LiquidTypeXTexture procedural texture type).
uniform int color_source = 1;
uniform bool interior = false;

#include "res://shaders/scene_light.gdshaderinc"
uniform vec3 specular_color = vec3(0.0);
// Close/far liquid colour with shallow/deep alpha.
uniform vec4 river_close = vec4(0.0);
uniform vec4 river_far = vec4(0.0);
uniform vec4 ocean_close = vec4(0.0);
uniform vec4 ocean_far = vec4(0.0);
// Underwater fog (start, end, density per yard) and colour.
uniform vec3 underwater_fog = vec3(0.0, 100000000.0, 0.0);
uniform vec3 underwater_fog_color = vec3(0.0);
#include "res://shaders/retail_fog.gdshaderinc"

// MapSceneRenderer.cpp:204 binds uExteriorSpecularColor.a, the specular power, as 1.0.
const float SPECULAR_POWER = 1.0;
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

// LiquidWater.cpp calculateWaveOffset.
float wave_offset(float time, float speed, float period) {
	float phase = 100.0 * time * speed;
	float cycle = period * 100000.0;
	return (phase - round(phase / cycle) * cycle) * 0.000010000001;
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
	float f2 = floats_0.z;
	float f3 = floats_0.w;
	float f4 = floats_4.x;
	float f5 = floats_4.y;
	float f6 = floats_4.z;
	float f7 = floats_4.w;
	float f8 = floats_8.x;
	float f9 = floats_8.y;
	float f10 = floats_8.z;
	float f14 = floats_12.z;

	// Texture coordinates: exterior ADT water maps world position * 0.06 (LiquidInstance.cpp
	// generateTexCoordsFromPos), rotated/scaled by f5/f3/f4 and scrolled by the flow (v16-v21).
	float flow_angle = flow.y * DEGREES;
	float flow_speed = flow.x + 0.015;
	vec2 scroll = vec2(
		scroll_component(time, cos(flow_angle) * flow_speed),
		scroll_component(time, sin(flow_angle) * flow_speed));
	float rotation = f5 * DEGREES;
	float c = cos(rotation);
	float s = sin(rotation);
	mat2 scale0 = mat2(vec2(c * f3, s * f3), vec2(-s * f3, c * f3));
	mat2 scale1 = mat2(vec2(c * f4, s * f4), vec2(-s * f4, c * f4));
	float wave_speed = floats_16.x * 0.0018750001;
	vec2 waves = vec2(0.0);
	if (wave_speed != 0.0) {
		waves = vec2(wave_offset(time, wave_speed, wave_periods.x), wave_offset(time, wave_speed, wave_periods.y));
	}
	vec4 v25 = vec4(mod(time, 10000.0) * 0.00031415926, f3 * waves.x, f4 * waves.y, f8);
	vec2 uv = mesh_uv ? UV : wow_position.xy * 0.06;
	vec4 v20 = vec4(scale0 * (uv / 32.0), scale1 * (-(uv + 0.5) / 32.0));
	vec4 phase = vec4(4.0, 2.0, -1.0, -1.0) * v25.x + wow_position.xyxy;
	float wobble_s = sin(phase.x);
	float wobble_c = cos(phase.y);
	vec4 v22 = v20.xyxy * 228.0 + vec4(0.1 * wobble_s, 0.1 * wobble_c, 0.05 * wobble_s + 0.5, 0.05 * wobble_c + 0.5);
	v20.xy += vec2(v25.y, 0.0);
	v20.zw += vec2(-v25.z, 0.0);
	vec4 v21 = vec4((32.0 * v25.w * v20.xy + scroll).yx, (32.0 * v25.w * v20.zw + scroll).yx);
	v20 = vec4((v20.xy + scroll).yx, (v20.zw + scroll).yx);

	vec3 camera = to_wow(CAMERA_POSITION_WORLD);
	vec3 to_fragment = wow_position - camera;
	float distance_to_camera = length(to_fragment);
	vec3 view_direction = to_fragment / distance_to_camera;  // v87

	// Water thickness along the view axis from the opaque depth buffer (v90-v94).
	float water_depth = -VERTEX.z;
	float raw_depth = texture(depth_texture, SCREEN_UV).r;
	float thickness = raw_depth <= 0.0001 ? 100000.0
		: scene_view_depth(depth_texture, SCREEN_UV, INV_PROJECTION_MATRIX) - water_depth;
	float relative_thickness = thickness / max(water_depth, 0.0001);

	// Two scrolled bump pairs blended by distance, then flattened by f6 and f14 (v95, v105).
	vec2 near_bump = (texture(texture_2, v20.xy).xy + texture(texture_2, v20.zw).xy - 1.0) * 2.0;
	vec2 far_bump = (texture(texture_2, v21.xy).xy + texture(texture_2, v21.zw).xy - 1.0) * 2.0;
	float bump_blend = clamp(0.2 + distance_to_camera / 1000.0, 0.0, 1.0);
	vec3 bump = vec3(0.7 * far_bump + (near_bump - 0.7 * far_bump) * bump_blend, 1.0);
	bump.xy *= f6;
	bump = normalize(bump);
	vec3 normal = normalize(vec3(f14 * bump.xy, 1.0));
	float facing = 1.0 - clamp(dot(-view_direction, normal), 0.0, 1.0);
	float fresnel = facing * facing;  // v106

	vec3 to_sun = normalize(-to_wow(light_sun_direction()));  // v111
	// uSunAttenuation.z is 0 without LightParams sun attenuation, so v116 = 1.
	float sun_attenuation = 1.0;
	vec3 surface_light = light_direct() * clamp(to_sun.z, 0.0, 1.0) * sun_attenuation + light_ambient();  // v126

	vec4 close_color = river_close;
	vec4 far_color = river_far;
	if (color_source == 0) {
		close_color = ocean_close;
		far_color = ocean_far;
	} else if (color_source == 2 && interior) {
		close_color.rgb = vec3(1.0);
		far_color.rgb = vec3(1.0);
	}
	float authored_depth = vertex_depth == 0.0 ? 0.0 : 1.0;  // v131
	float d = vertex_depth;
	float depth_mix = clamp(depth_coefficients.x + depth_coefficients.y * d + depth_coefficients.z * d * d + depth_coefficients.w * d * d * d, 0.0, 1.0);
	float color_mix = depth_mix * f2 + abs(0.1 * bump.y);  // v138
	vec4 water_color = close_color + (far_color - close_color) * color_mix;  // v139
	vec3 lit_water = water_color.rgb * surface_light;  // v140

	// Refracted opaque scene, kept only where the offset still lands behind the water (v141-v145).
	vec2 refracted_uv = SCREEN_UV + 0.1 * normal.xy * clamp(100.0 * relative_thickness * (1.2 - fresnel), 0.0, 1.0);
	float refracted_depth = scene_view_depth(depth_texture, refracted_uv, INV_PROJECTION_MATRIX);
	bool behind = refracted_depth > water_depth;
	refracted_uv = behind ? refracted_uv : SCREEN_UV;
	vec3 below = linear_to_gamma(texture(screen_texture, refracted_uv).rgb);
	float below_thickness = behind ? refracted_depth - water_depth : thickness;

	// Underwater fog over the refracted scene (v148-v157).
	float fog_distance = max(0.0, below_thickness - underwater_fog.x);
	float fog_visibility = min(1.0 / exp(fog_distance * underwater_fog.z),
		clamp((1.0 / 0.7) * (1.0 - below_thickness / underwater_fog.y), 0.0, 1.0));
	vec3 color = underwater_fog_color + (below - underwater_fog_color) * fog_visibility;
	// Opacity is a blend weight, not an amplification factor. The input-contract GPU
	// regression reproduces the old negative-alpha scene gain of 6.27x.
	float opacity = clamp((1.0 - fresnel * 0.1) * water_color.a, 0.0, 1.0);
	color = mix(color, lit_water, opacity);

	vec3 halfway = normalize(to_sun - view_direction);
	float highlight = pow(clamp(dot(halfway, normal), 0.0, 1.0), SPECULAR_POWER) * sun_attenuation
		+ clamp(normal.z - 0.8, 0.0, 1.0);
	vec3 specular = specular_color * highlight * fresnel;  // v168

	// Shore: alpha rises and foam fades over the first yards of water thickness (v169-v173).
	float shore = 1.2 * clamp(fresnel * 0.5 + 0.2, 0.0, 1.0);
	float alpha = smoothstep(0.0, shore, thickness);
	float foam_fade = 1.0 - smoothstep(0.0, 2.5 * clamp(fresnel * 0.5 + 0.2, 0.0, 1.0), thickness * (1.0 - view_direction.z));
	vec3 foam = texture(texture_3, v22.xy - normal.xy).rgb + texture(texture_3, v22.zw + 0.2 * normal.yx).rgb;
	color += foam * (f7 * 0.35 * foam_fade * (1.1 - clamp(distance_to_camera / 150.0, 0.0, 1.0)) * normal.z);
	alpha = (alpha + (1.0 - alpha) * clamp(-thickness, 0.0, 1.0)) * authored_depth;

	float n_dot_l = clamp(dot(to_sun, normal), 0.0, 1.0);
	color += f9 * n_dot_l * sun_attenuation * light_direct() * lit_water + (interior ? 0.0 : f10) * specular;
	color = apply_retail_fog(color, VERTEX, INV_VIEW_MATRIX);
	ALBEDO = gamma_to_linear(max(color, vec3(0.0)));
	ALPHA = alpha;
}
