shader_type spatial;
render_mode unshaded, cull_disabled, blend_mix, depth_draw_never, fog_disabled;

// Retail swamp (LiquidMaterial 14): WebWowViewerCpp shaders/slang/common/liquid/
// liquidSwampMat.slang calcSwampLiquidMat, packed by LiquidSwamp.cpp into SwampData. The
// v-names in comments are that function's locals. Maths runs in WoW world space (z up) and in
// authored colour (sRGB bytes / 255, as the terrain shader); the result is converted to linear.

// LiquidTypeXTexture slots 0-2 (depth-blended layers), 3 (shore overlay), 4 (animated normal
// map, frame k) and texture_next (reference uTexture6: slot 4 frame k + 1).
uniform sampler2D texture_0 : repeat_enable, filter_linear_mipmap;
uniform sampler2D texture_1 : repeat_enable, filter_linear_mipmap;
uniform sampler2D texture_2 : repeat_enable, filter_linear_mipmap;
uniform sampler2D texture_3 : repeat_enable, filter_linear_mipmap;
uniform sampler2D texture_4 : repeat_enable, filter_linear_mipmap;
uniform sampler2D texture_next : repeat_enable, filter_linear_mipmap;
uniform sampler2D screen_texture : hint_screen_texture, filter_linear, repeat_disable;
uniform sampler2D depth_texture : hint_depth_texture, filter_nearest, repeat_disable;

// Shared material clock in milliseconds (the reference scene time).
uniform float animation_time_ms = 0.0;
// LiquidType.Float[0..17] (SwampData f0..f17).
uniform vec4 floats_0 = vec4(0.0);
uniform vec4 floats_4 = vec4(0.0);
uniform vec4 floats_8 = vec4(0.0);
uniform vec4 floats_12 = vec4(0.0);
uniform vec2 floats_16 = vec2(0.0);
// LiquidType.Coefficient[0..3] (SwampData depthTable): vertex depth -> layer mix, a cubic.
uniform vec4 depth_coefficients = vec4(0.0);
// LiquidType.Color[0] as m_colors[0] bytes / 255; LiquidSwamp.cpp swizzles it [2],[1],[0].
uniform vec3 liquid_color_0 = vec3(0.0);
// Frames in LiquidTypeXTexture slots 3-5; .y is slot 4 (SwampData colorRGB_pad.w).
uniform vec3 frame_counts_3 = vec3(1.0);
// LiquidObject FlowSpeed and FlowDirection (degrees).
uniform vec2 flow = vec2(0.0);

// Scene light, authored RGB.
uniform vec3 ambient = vec3(1.0);
uniform vec3 direct = vec3(0.0);
// Godot world direction the sunlight travels.
uniform vec3 sun_direction = vec3(0.0, -1.0, 0.0);
uniform vec3 specular_color = vec3(0.0);
// Scene fog in linear RGB, as the terrain shader.
uniform vec3 fog_color;
uniform vec2 fog_range;
uniform float fog_density;
uniform float fog_opacity;
uniform int fog_mode = 0;

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

// The reference's written-out smoothstep: clamp((x - a) / (b - a)) Hermite, also for a >= b.
float ramp(float a, float b, float x) {
	float t = clamp((x - a) / (b - a), 0.0, 1.0);
	return t * t * (3.0 - 2.0 * t);
}

// commonLiquidFunctions.slang sceneViewDepth: view distance of the depth buffer at `uv`.
float scene_view_depth(sampler2D depth, vec2 uv, mat4 inverse_projection) {
	float raw = texture(depth, uv).r;
	vec4 view = inverse_projection * vec4(uv * 2.0 - 1.0, raw, 1.0);
	return -view.z / view.w;
}

// Two scrolled normal-map pairs, near/far blended and flattened by f4 (v83-v85, v93-v95).
vec3 swamp_bump(sampler2D normals, vec4 near_uv, vec4 far_uv, float flatten) {
	vec2 near_bump = (texture(normals, near_uv.xy).xy + texture(normals, near_uv.zw).xy - 1.0) * 2.0;
	vec2 far_bump = (texture(normals, far_uv.xy).xy + texture(normals, far_uv.zw).xy - 1.0) * 2.0;
	vec3 bump = vec3(1.17 * far_bump + (near_bump - 1.17 * far_bump) * 0.15, 1.0);
	bump.xy *= flatten;
	return normalize(bump);
}

vec3 apply_retail_fog(vec3 color, float distance_to_camera) {
	float amount = 0.0;
	if (fog_mode == 1) {
		// commonFogFunctions.slang calculateLegacyFog, without height fog.
		float exp_fog = exp(-max(0.0, distance_to_camera - fog_range.x) * fog_density);
		float end_fade = clamp(1.42857146 * (1.0 - distance_to_camera / fog_range.y), 0.0, 1.0);
		amount = 1.0 - min(exp_fog, end_fade);
	}
	return mix(color, linear_to_gamma(fog_color), amount * fog_opacity);
}

void vertex() {
	wow_position = to_wow((MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz);
	// adt_water_geometry: MH2O vertex depth / 255 (vPositionDepth.w).
	vertex_depth = COLOR.a;
}

void fragment() {
	float time = animation_time_ms;
	float f4 = floats_4.x;
	float f5 = floats_4.y;
	float f6 = floats_4.z;  // v26
	float f7 = floats_4.w;  // v27
	float f8 = floats_8.x;  // v28
	float f9 = floats_8.y;  // v29
	float f10 = floats_8.z;  // v30
	float f12 = floats_12.x;
	float f13 = floats_12.y;
	float f14 = floats_12.z;
	float f15 = floats_12.w;
	float f16 = floats_16.x;  // v31
	float f17 = floats_16.y;  // v53
	vec3 murk_color = vec3(liquid_color_0.b, liquid_color_0.g, liquid_color_0.r);  // v40.rgb

	// Normal-map crossfade: fraction of the way from frame k (texture_4) to k + 1 (v55-v57).
	float interval = f5 * 1000.0;
	float frames = frame_counts_3.y;
	float frame_blend = 0.0;
	if (interval > 0.0 && frames > 1.0) {
		frame_blend = fract(frames * mod(time, interval) / interval);
	}

	// Texture coordinates: exterior ADT liquid maps world position * 0.06 (LiquidInstance.cpp
	// generateTexCoordsFromPos); the normal map is scaled by f6 and scrolled by the flow.
	float flow_angle = flow.y * DEGREES;
	float flow_speed = flow.x + 0.015;
	vec2 scroll = vec2(
		scroll_component(time, cos(flow_angle) * flow_speed),
		scroll_component(time, sin(flow_angle) * flow_speed));  // v35
	vec2 uv = mesh_uv ? UV : wow_position.xy * 0.06;  // v7
	vec4 v58 = vec4(uv / 32.0, -(uv + 0.5) / 32.0);
	vec4 v59 = vec4((f6 * 32.0 * f16 * v58.xy + scroll).yx, (f6 * 32.0 * f16 * v58.zw + scroll).yx);
	v58 = vec4((f6 * v58.xy + scroll).yx, (f6 * v58.zw + scroll).yx);
	vec2 overlay_uv = f10 * (uv / 32.0);  // v60
	vec2 layer_uv_0 = f7 * uv;  // v61
	vec2 layer_uv_1 = f8 * uv;  // v62
	vec2 layer_uv_2 = f9 * uv;  // v63

	float authored_depth = vertex_depth == 0.0 ? 0.0 : 1.0;  // v65
	vec3 camera = to_wow(CAMERA_POSITION_WORLD);
	vec3 to_fragment = wow_position - camera;
	float distance_to_camera = length(to_fragment);  // v68
	vec3 view_direction = to_fragment / distance_to_camera;  // v69

	// Swamp thickness along the view axis from the opaque depth buffer (v71-v75).
	float water_depth = -VERTEX.z;  // v74
	float raw_depth = texture(depth_texture, SCREEN_UV).r;
	float thickness = raw_depth <= 0.0001 ? 100000.0
		: scene_view_depth(depth_texture, SCREEN_UV, INV_PROJECTION_MATRIX) - water_depth;

	vec3 bump_k = swamp_bump(texture_4, v58, v59, f4);  // v77
	vec3 bump_next = swamp_bump(texture_next, v58, v59, f4);  // v87
	vec3 bump = bump_k + (bump_next - bump_k) * frame_blend;  // v97
	vec3 normal = normalize(vec3(bump.xy, 1.0));  // v70
	float facing = 1.0 - clamp(dot(-view_direction, normal), 0.0, 1.0);
	float fresnel = facing * facing * f14;  // v103

	// uSunAttenuation.z is 0 without LightParams sun attenuation, so v108 = 1.
	float sun_attenuation = 1.0;
	float d = vertex_depth;
	float depth_mix = clamp(depth_coefficients.x + depth_coefficients.y * d + depth_coefficients.z * d * d + depth_coefficients.w * d * d * d, 0.0, 1.0)
		+ abs(0.1 * bump.y);  // v113

	// Refracted opaque scene, kept only where the offset still lands behind the swamp (v120-v125).
	float shore = ramp(0.0, 0.04, thickness);  // v120
	vec2 refracted_uv = SCREEN_UV + normal.xy * (0.05 * clamp(shore * thickness, 0.0, 1.0)
		+ 0.0625 * clamp(0.01 * shore * thickness * (1.2 - fresnel), 0.0, 1.0));
	float refracted_depth = scene_view_depth(depth_texture, refracted_uv, INV_PROJECTION_MATRIX);
	bool behind = refracted_depth > water_depth;
	refracted_uv = behind ? refracted_uv : SCREEN_UV;
	vec3 below = linear_to_gamma(texture(screen_texture, refracted_uv).rgb);
	float below_thickness = behind ? refracted_depth - water_depth : thickness;
	// Murk absorbs the refracted scene towards the ambient-lit murk colour (v126).
	float murk = exp(-f17 * max(below_thickness, 0.0));
	below = below * murk + murk_color * ambient * (1.0 - murk);

	// Three depth-blended surface layers, distorted by the bump (v127-v136).
	vec2 distortion = bump.xy * 0.1;
	vec4 layer_0 = texture(texture_0, layer_uv_0 + distortion);
	vec4 layer_1 = texture(texture_1, layer_uv_1 + distortion);
	vec4 layer_2 = texture(texture_2, layer_uv_2 + distortion);
	vec4 shallow = layer_0 + (layer_1 - layer_0) * ramp(floats_0.x, floats_0.y, depth_mix);
	vec4 deep = layer_1 + (layer_2 - layer_1) * ramp(floats_0.z, floats_0.w, depth_mix);
	vec4 surface = shallow + (deep - shallow) * ramp(floats_0.y, floats_0.z, depth_mix);

	// Shore overlay (slot 3) over the first f15 yards of thickness (v137-v138).
	float overlay = 1.0 - ramp(0.0, f15, thickness * (1.0 - view_direction.z));
	vec4 overlay_texel = texture(texture_3, overlay_uv);
	overlay *= overlay_texel.a;
	surface.rgb = surface.rgb * (1.0 - overlay) + overlay_texel.rgb * overlay;
	surface.a = clamp(surface.a + overlay, 0.0, 1.0);
	vec3 color = surface.a * surface.rgb + (1.0 - surface.a) * below;  // v139

	// v141 dots the sun with (1, 1, 1), not the normal, as the reference does.
	vec3 to_sun = normalize(-to_wow(sun_direction));  // v140
	float flat_light = clamp(dot(to_sun, vec3(1.0)), 0.0, 1.0) * sun_attenuation;
	color *= f12 * clamp(flat_light * direct + ambient, 0.0, 1.0);  // v142

	vec3 halfway = normalize(to_sun - view_direction);  // v148
	float highlight = pow(clamp(dot(halfway, normal), 0.0, 1.0), SPECULAR_POWER) * sun_attenuation
		+ clamp(normal.z - 0.8, 0.0, 1.0);  // v150
	vec3 specular = specular_color * highlight;  // v151
	float n_dot_l = clamp(dot(to_sun, normal), 0.0, 1.0) * sun_attenuation;  // v155
	color = f12 * clamp(n_dot_l * direct + ambient, 0.0, 1.0) * color + f13 * specular;  // v157

	// Alpha: opaque past the 0.04 yd shore, else fading in towards half the fog end (v158-v159).
	float fog_fade = ramp(fog_range.x, fog_range.y * 0.5, distance_to_camera);
	float alpha = (shore + (1.0 - shore) * sqrt(fog_fade)) * authored_depth;

	color = apply_retail_fog(color, distance_to_camera);
	ALBEDO = gamma_to_linear(max(color, vec3(0.0)));
	ALPHA = alpha;
}
