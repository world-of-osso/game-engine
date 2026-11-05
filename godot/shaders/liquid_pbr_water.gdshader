shader_type spatial;
render_mode unshaded, cull_disabled, blend_mix, depth_draw_never, fog_disabled;

// Forever LiquidMaterial 130 = Liquid::LiquidTypePBRWater (EF1). The class owns a
// separate `PBRWater Prepass`; this shader is the shading pass only. Its exact uber
// permutation and bind-unit -> t-register map are not published, so the six authored
// masks are bound by their VERIFIED row roles and the pixel maths follows Forever's
// Water pixel program (PBR branch) where that program is readable. Everything taken
// from the uber without a published permutation is marked ADAPTED.

// LiquidTypeXTexture row order (EF1): foam high, mid, low, rim, depth, river.
uniform sampler2D foam_high : repeat_enable, filter_linear_mipmap;
uniform sampler2D foam_mid : repeat_enable, filter_linear_mipmap;
uniform sampler2D foam_low : repeat_enable, filter_linear_mipmap;
uniform sampler2D foam_rim : repeat_enable, filter_linear_mipmap;
uniform sampler2D foam_depth : repeat_enable, filter_linear_mipmap;
uniform sampler2D foam_river : repeat_enable, filter_linear_mipmap;
uniform sampler2D screen_texture : hint_screen_texture, filter_linear, repeat_disable;
uniform sampler2D depth_texture : hint_depth_texture, filter_nearest, repeat_disable;

// G10 / liquid_shore::ShoreDepthMap.
uniform sampler2D shore_depth_map : repeat_disable, filter_linear;
uniform vec4 shore_map_transform = vec4(0.0);
uniform bool shore_map_enabled = false;
uniform float shore_scale = 1.0;

// Forever EF2: one CPU-assembled 38-float batch, copied verbatim. vec0 is
// (1-Color0.rgb, Float_0); vec1 Float_3..6; vec2 (Color1.rgb, Float_2);
// vec3 (Color2.rgb, Float_1); vec4..9 Float_7..25 with +0x60=Float_15, +0x64=Float_19.
// ADAPTED: the batch-word -> cb-register hop is unpublished.
uniform vec4 pbr_batch_0 = vec4(0.0);
uniform vec4 pbr_batch_1 = vec4(0.0);
uniform vec4 pbr_batch_2 = vec4(0.0);
uniform vec4 pbr_batch_3 = vec4(0.0);
uniform vec4 pbr_batch_4 = vec4(0.0);
uniform vec4 pbr_batch_5 = vec4(0.0);
uniform vec4 pbr_batch_6 = vec4(0.0);
uniform vec4 pbr_batch_7 = vec4(0.0);
uniform vec4 pbr_batch_8 = vec4(0.0);
uniform vec4 pbr_batch_9 = vec4(0.0);

// The WaterMaterials clock drives the PBR layers; See LiquidSurface::set_time.
uniform float pbr_time_ms = 0.0;
uniform vec2 flow = vec2(0.0);
uniform int color_source = 1;
uniform bool mesh_uv = false;
uniform bool interior = false;

#include "res://shaders/scene_light.gdshaderinc"
global uniform vec3 specular_color;
global uniform vec4 river_close;
global uniform vec4 river_far;
global uniform vec4 ocean_close;
global uniform vec4 ocean_far;
// LiquidData stores Color as BGR bytes (Liquid Ley Line already swaps); these are RGB.
vec3 row_rgb(vec3 stored) { return stored.bgr; }
global uniform vec3 underwater_fog;
global uniform vec3 underwater_fog_color;
#include "res://shaders/retail_fog.gdshaderinc"

const float DEGREES = 0.017453292;
const float SPECULAR_POWER = 1.0;

varying vec3 wow_position;
varying vec2 shore_uv;
varying float authored_coverage;

vec3 to_wow(vec3 godot) {
	return vec3(godot.x, -godot.z, godot.y);
}

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

// G4 per-layer transform: Float[3k] * uv + wrapped velocity.
vec2 layer_uv(vec4 layer, vec2 uv, float time) {
	float phase = (layer.y + flow.y) * DEGREES;
	float speed = layer.z * flow.x;
	vec2 velocity = vec2(cos(phase) * speed, sin(phase) * speed);
	return uv * layer.x + vec2(
		scroll_component(time, velocity.x), scroll_component(time, velocity.y));
}

float scene_view_depth(sampler2D depth, vec2 uv, mat4 inverse_projection) {
	float raw = texture(depth, uv).r;
	vec4 view = inverse_projection * vec4(uv * 2.0 - 1.0, raw, 1.0);
	return -view.z / view.w;
}

vec3 apply_retail_fog(vec3 color, vec3 view_position, mat4 inv_view) {
	return retail_fog(color, (inv_view * vec4(view_position, 1.0)).xyz, inv_view[3].xyz, 2);
}

void vertex() {
	wow_position = to_wow((MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz);
	authored_coverage = COLOR.a > 0.0 ? 1.0 : 0.0;
	shore_uv = VERTEX.xz * shore_map_transform.xy + shore_map_transform.zw;
}

void fragment() {
	float time = pbr_time_ms;
	vec2 uv = mesh_uv ? UV : wow_position.xy * 0.06;
	// G4 triples split across EF2 batch words: (Float_0,Float_1,Float_2) etc.
	vec4 layer_0 = vec4(pbr_batch_0.w, pbr_batch_3.w, pbr_batch_2.w, 0.0);
	vec4 layer_1 = vec4(pbr_batch_1.x, pbr_batch_1.y, pbr_batch_1.z, 0.0);
	vec4 layer_2 = vec4(pbr_batch_1.w, pbr_batch_4.x, pbr_batch_4.y, 0.0);
	vec2 uv0 = layer_uv(layer_0, uv, time);
	vec2 uv1 = layer_uv(layer_1, uv, time);
	vec2 uv2 = layer_uv(layer_2, uv, time);
	// Float_13..17 = the batch words at +0x58..+0x68 (batch[22..26]).
	vec4 foam_cubic = vec4(pbr_batch_5.z, pbr_batch_5.w, pbr_batch_6.x, pbr_batch_6.y);
	float foam_strength = pbr_batch_6.z;

	vec4 high = texture(foam_high, uv0);
	vec4 mid = texture(foam_mid, uv1);
	vec4 low = texture(foam_low, uv2);
	vec4 rim = texture(foam_rim, uv0);
	vec4 depth_mask = texture(foam_depth, uv1);
	vec4 river = texture(foam_river, uv2);

	// G10 shore: D = saturate(water depth / Float[11]).
	float d = 1.0;
	if (shore_map_enabled) {
		d = clamp(texture(shore_depth_map, shore_uv).r * shore_scale, 0.0, 1.0);
	}

	// Forever Water PBR branch: cubic(mask) * weight drives the close/far mix.
	float m = mid.r;
	float m2 = m * m;
	float foam = clamp(foam_cubic.x + foam_cubic.y * m
		+ foam_cubic.z * m2 + foam_cubic.w * m2 * m, 0.0, 1.0);
	foam = clamp(foam * foam_strength, 0.0, 1.0);
	// ADAPTED: the authored masks are near-white coverage; their alpha is the foam
	// amount. High/mid/low stack, rim and river add near the shoreline.
	float mask = clamp(high.a + mid.a + low.a, 0.0, 1.0);
	foam = clamp(max(foam, mask * (1.0 - d)), 0.0, 1.0);

	// ADAPTED: Color[1]/Color[2] act as the close/far pair and Color[0] tints foam.
	// ADAPTED: LightData's River alpha pair is the water opacity for the PBR class;
	// LightData7588's Ocean deep alpha is zero, which would make open water invisible.
	vec4 close_color = vec4(row_rgb(pbr_batch_2.rgb), river_close.a);
	vec4 far_color = vec4(row_rgb(pbr_batch_3.rgb), river_far.a);
	if (color_source == 2 && interior) {
		close_color.rgb = vec3(1.0);
		far_color.rgb = vec3(1.0);
	}
	vec4 water_color = mix(close_color, far_color, foam);
	// ADAPTED: the river mask tints the authored water colour; it is white for calm water.
	water_color.rgb = mix(water_color.rgb, water_color.rgb * river.rgb, river.a * (1.0 - foam));

	vec3 camera = to_wow(CAMERA_POSITION_WORLD);
	vec3 to_fragment = wow_position - camera;
	float distance_to_camera = length(to_fragment);
	vec3 view_direction = to_fragment / distance_to_camera;

	float water_depth = -VERTEX.z;
	float raw_depth = texture(depth_texture, SCREEN_UV).r;
	float thickness = raw_depth <= 0.0001 ? 100000.0
		: scene_view_depth(depth_texture, SCREEN_UV, INV_PROJECTION_MATRIX) - water_depth;
	float relative_thickness = thickness / max(water_depth, 0.0001);

	vec3 normal_world = normalize(to_wow(NORMAL));
	vec3 normal = normalize(vec3(0.35 * (rim.rg * 2.0 - 1.0) + 0.65 * normal_world.xy, 1.0));
	float facing = 1.0 - clamp(dot(-view_direction, normal), 0.0, 1.0);
	float fresnel = facing * facing;

	vec3 to_sun = normalize(-to_wow(light_sun_direction()));
	vec3 surface_light = light_direct() * clamp(to_sun.z, 0.0, 1.0) + light_ambient();
	vec3 lit_water = water_color.rgb * surface_light;

	vec2 refracted_uv = SCREEN_UV + 0.1 * normal.xy
		* clamp(100.0 * relative_thickness * (1.2 - fresnel), 0.0, 1.0);
	float refracted_depth = scene_view_depth(depth_texture, refracted_uv, INV_PROJECTION_MATRIX);
	bool behind = refracted_depth > water_depth;
	refracted_uv = behind ? refracted_uv : SCREEN_UV;
	vec3 below = linear_to_gamma(texture(screen_texture, refracted_uv).rgb);
	float below_thickness = behind ? refracted_depth - water_depth : thickness;

	float fog_distance = max(0.0, below_thickness - underwater_fog.x);
	float fog_visibility = min(1.0 / exp(fog_distance * underwater_fog.z),
		clamp((1.0 / 0.7) * (1.0 - below_thickness / underwater_fog.y), 0.0, 1.0));
	vec3 color = underwater_fog_color + (below - underwater_fog_color) * fog_visibility;

	// Forever Water: t1's alpha scales the accumulated water alpha; the authored masks
	// supply the remaining coverage. ADAPTED: coverage weights come from row alphas.
	float opacity = clamp((1.0 - fresnel * 0.1) * water_color.a, 0.0, 1.0);
	color = mix(color, lit_water, opacity);

	vec3 halfway = normalize(to_sun - view_direction);
	float highlight = pow(clamp(dot(halfway, normal), 0.0, 1.0), SPECULAR_POWER);
	vec3 specular = specular_color * highlight * fresnel * depth_mask.a

		+ water_color.rgb * max(dot(to_sun, normal), 0.0) * 0.25 * depth_mask.a;
	color += specular;

	// ADAPTED: shore foam and the authored floor coverage replace the MH2O depth byte.
	float shore_foam = clamp(rim.a + low.a, 0.0, 1.0) * (1.0 - d);
	color = mix(color, mix(water_color.rgb, 1.0 - row_rgb(pbr_batch_0.rgb), high.a * 0.5) + high.r * 0.2,
		shore_foam * 0.6);

	float alpha = clamp((0.45 + 0.55 * d) * water_color.a + shore_foam * 0.35, 0.0, 1.0);
	alpha = clamp(alpha * authored_coverage, 0.0, 1.0);

	color = apply_retail_fog(color, VERTEX, INV_VIEW_MATRIX);
	ALBEDO = gamma_to_linear(max(color, vec3(0.0)));
	ALPHA = alpha;
}
