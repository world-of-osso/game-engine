shader_type spatial;
render_mode unshaded, cull_disabled, blend_mix, depth_draw_never, fog_disabled;

// Forever LiquidMaterial 130 = Liquid::LiquidTypePBRWater (EF1). The class owns a
// separate `PBRWater Prepass`; this shader is the shading pass only. Its exact uber
// permutation and bind-unit -> t-register map are not published, so the six authored
// masks are bound by their VERIFIED row roles and the pixel maths follows Forever's
// Water pixel program (PBR branch) where that program is readable. Everything taken
// from the uber without a published permutation is marked ADAPTED.

// LiquidTypeXTexture row order (EF1): foam high, mid, low, rim, depth, river.
uniform sampler2D foam_high : repeat_enable, filter_linear_mipmap_anisotropic;
uniform sampler2D foam_mid : repeat_enable, filter_linear_mipmap_anisotropic;
uniform sampler2D foam_low : repeat_enable, filter_linear_mipmap_anisotropic;
uniform sampler2D foam_rim : repeat_enable, filter_linear_mipmap_anisotropic;
uniform sampler2D foam_depth : repeat_enable, filter_linear_mipmap_anisotropic;
uniform sampler2D foam_river : repeat_enable, filter_linear_mipmap_anisotropic;
uniform sampler2D screen_texture : hint_screen_texture, filter_linear, repeat_disable;

// RF3: Forever's PBRWater shading pass binds no scene depth, and Float_11 is not a fade
// width (nothing reciprocates it). Thickness comes from the row's own `_depth` art below.

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

vec3 apply_retail_fog(vec3 color, vec3 view_position, mat4 inv_view) {
	return retail_fog(color, (inv_view * vec4(view_position, 1.0)).xyz, inv_view[3].xyz, 2);
}

void vertex() {
	wow_position = to_wow((MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz);
	// I2 B.4: the authored depth byte rides the second texture pair's second component.
	authored_coverage = UV2.y > 0.0 ? 1.0 : 0.0;
}

void fragment() {
	float time = pbr_time_ms;
	vec2 uv = mesh_uv ? UV : wow_position.xy * 0.06;
	// Forever PBRWater pixel program (2977157 permutation 255) samples four 2D stages with two
	// vertex texture-coordinate pairs: t0 with the first pair, t1 with its second half, t2 with
	// the second pair and t3 with its second half. Each pair keeps the material's own scroll
	// transform (G4 triples from the batch).
	vec4 layer_0 = vec4(pbr_batch_0.w, pbr_batch_3.w, pbr_batch_2.w, 0.0);
	vec4 layer_1 = vec4(pbr_batch_1.x, pbr_batch_1.y, pbr_batch_1.z, 0.0);
	vec4 layer_2 = vec4(pbr_batch_1.w, pbr_batch_4.x, pbr_batch_4.y, 0.0);
	vec2 uv0 = layer_uv(layer_0, uv, time);
	vec2 uv1 = layer_uv(layer_1, uv, time);
	vec2 uv2 = layer_uv(layer_2, uv, time);

	// INFERRED role assignment: the shading path binds four surface textures to four
	// consecutive stages in the row's role order, so t0..t3 are the high, mid, low and rim
	// elements. The depth and river elements belong to the depth variant and this permutation
	// never reads them.
	vec4 high = texture(foam_high, uv0);
	vec4 mid = texture(foam_mid, uv1);
	vec4 low = texture(foam_low, uv2);
	vec4 rim = texture(foam_rim, uv0);

	// Program lines 130-138: coverage = 2*(t0*t1) + t2.wxyz, its x component saturated is the
	// alpha coverage and the luminance of yzw is the foam amount. All six authored masks are
	// near-white coverage art with the signal in alpha (measured 0.91-0.96 rgb, 0.18-0.46 a).
	vec4 stack = low.wxyz + 2.0 * (high * mid).wxyz;
	float coverage = clamp(stack.x, 0.0, 1.0);
	float foam_luma = dot(stack.yzw, vec3(0.300000, 0.590000, 0.110000));
	float mask = coverage * foam_luma;

	// Program lines 134-136: the water colour is t3.rgb * the material tint * t3.a. The row
	// colours upload as RGB triples prepared by the client (colour 0 already inverted into
	// batch word 0), so no channel swap is applied here.
	// ADAPTED: the client uploads the row's first float as a blend amount against a
	// per-channel target table whose contents are unpublished. Colour 1 is the first
	// non-inverted triple; the table stands in as the LightData river close colour, which is
	// the same water colour the LiquidType5 fallback uses.
	vec3 tint = mix(pbr_batch_2.rgb, river_close.rgb, clamp(pbr_batch_0.w, 0.0, 1.0));
	vec3 water_color = rim.rgb * tint * rim.a;

	// Program line 680: the light factor multiplies the water colour, and every term that
	// builds it saturates, so it is an attenuation in 0..1.
	vec3 to_sun = normalize(-to_wow(light_sun_direction()));
	vec3 attenuation = clamp(light_direct() * clamp(to_sun.z, 0.0, 1.0) + light_ambient(), 0.0, 1.0);

	vec3 camera = to_wow(CAMERA_POSITION_WORLD);
	vec3 to_fragment = wow_position - camera;
	vec3 view_direction = to_fragment / length(to_fragment);

	// The screen copy and its underwater fog are the same inputs the fallback uses; the
	// authored masks replace the MH2O depth byte as the blend weight (deviation: retail writes
	// an alpha-to-coverage mask and discards below 0.501961, which a forward renderer has no
	// equivalent for).
	vec3 below = linear_to_gamma(texture(screen_texture, SCREEN_UV).rgb);
	float fog_distance = max(0.0, -underwater_fog.x);
	float fog_visibility = min(1.0 / exp(fog_distance * underwater_fog.z),
		clamp(1.0 / 0.7, 0.0, 1.0));
	vec3 scene_color = underwater_fog_color + (below - underwater_fog_color) * fog_visibility;

	vec3 color = mix(scene_color, water_color * attenuation, clamp(mask, 0.0, 1.0));
	// A small view-dependent term keeps the surface from reading as a flat wash: the program's
	// rim element feeds its normal, and the fresnel-weighted sky tint is the fallback's own.
	vec3 normal_world = normalize(to_wow(NORMAL));
	vec3 normal = normalize(vec3(0.35 * (rim.rg * 2.0 - 1.0) + 0.65 * normal_world.xy, 1.0));
	float facing = 1.0 - clamp(dot(-view_direction, normal), 0.0, 1.0);
	float fresnel = facing * facing;
	color += specular_color * fresnel * 0.15;

	float alpha = clamp(mask * authored_coverage, 0.0, 1.0);

	color = apply_retail_fog(color, VERTEX, INV_VIEW_MATRIX);
	ALBEDO = gamma_to_linear(max(color, vec3(0.0)));
	ALPHA = alpha;
}
