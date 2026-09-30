shader_type spatial;
// Material binding selects authored cull/blend variants; no Godot PBR shading or fog.
render_mode ambient_light_disabled, fog_disabled, specular_disabled, cull_back, blend_mix;
#include "res://shaders/m2_point_light.gdshaderinc"

// WebWowViewerCpp WMOMaterialTemplate.textures[0..9] (`wmoObject.cpp` getMaterialInstance).
uniform sampler2D base_texture : source_color, repeat_enable;
uniform sampler2D second_texture : source_color, repeat_enable;
uniform sampler2D third_texture : source_color, repeat_enable;
uniform sampler2D texture_4 : source_color, repeat_enable;
uniform sampler2D texture_5 : source_color, repeat_enable;
uniform sampler2D texture_6 : source_color, repeat_enable;
uniform sampler2D texture_7 : source_color, repeat_enable;
uniform sampler2D texture_8 : source_color, repeat_enable;
uniform sampler2D texture_9 : source_color, repeat_enable;
uniform vec4 base_color : source_color = vec4(1.0);
// StandardMaterial emissive arrives in linear space.
uniform vec3 emissive;
uniform vec3 interior_ambient;
uniform bool exterior_lit = false;
uniform bool unlit = false;
uniform bool unfogged = false;
uniform bool has_mocv = false;
uniform bool has_second_mocv = false;
uniform bool has_uv2 = false;
uniform bool has_uv3 = false;
uniform bool has_uv4 = false;
uniform bool has_moc2 = false;
// iWmoApi.h wmoMaterialShader entry for the MOMT shader id.
uniform int vertex_shader = 0;
uniform int pixel_shader = 0;
uniform int blend_mode = 0;

// RetailSceneLight in authored (sRGB byte / 255) space.
uniform vec3 ambient = vec3(1.0);
uniform vec3 horizon_ambient = vec3(1.0);
uniform vec3 ground_ambient = vec3(1.0);
uniform vec3 direct;
uniform vec3 sun_direction = vec3(0.0, -1.0, 0.0);
uniform vec3 fog_color;
// Retail legacy exponential fog: (start, end) in yards and density per yard.
uniform vec2 fog_range;
uniform float fog_density;
uniform float fog_opacity;
// 0 off, 1 retail legacy fog.
uniform int fog_mode = 0;

varying vec3 world_position;
varying vec3 world_normal;
varying vec3 camera_position;
varying float second_mocv_alpha;
// commonWMOMaterial.slang calcWMOVertMat vTexCoord2..4 and wmoShader.vert vColorSecond (MOC2).
varying vec2 second_uv;
varying vec2 third_uv;
varying vec2 fourth_uv;
varying vec4 moc2;
varying vec3 ambient_authored;
varying vec3 diffuse_authored;
varying vec3 emission_authored;
varying float exterior_weight;

vec3 linear_to_gamma(vec3 linear_color) {
	vec3 low = linear_color * 12.92;
	vec3 high = 1.055 * pow(max(linear_color, vec3(0.0)), vec3(1.0 / 2.4)) - 0.055;
	return mix(high, low, lessThanEqual(linear_color, vec3(0.0031308)));
}

vec3 gamma_to_linear(vec3 gamma_color) {
	vec3 low = gamma_color / 12.92;
	vec3 high = pow((max(gamma_color, vec3(0.0)) + 0.055) / 1.055, vec3(2.4));
	return mix(high, low, lessThanEqual(gamma_color, vec3(0.04045)));
}

vec3 retail_ambient(vec3 up_ambient, vec3 horizon, vec3 ground, vec3 normal, float n_dot_l) {
	float n_dot_up = dot(normal, vec3(0.0, 1.0, 0.0));
	vec3 hemisphere;
	if (n_dot_up >= 0.0) {
		hemisphere = mix(horizon, up_ambient, n_dot_up);
	} else {
		hemisphere = mix(horizon, ground, -n_dot_up);
	}
	return mix(hemisphere * 0.7, hemisphere * 1.1, 0.5 + 0.5 * n_dot_l);
}

vec3 retail_fog_color() {
	if (blend_mode == 3 || blend_mode == 10 || blend_mode == 13) {
		return vec3(0.0);
	} else if (blend_mode == 4) {
		return vec3(1.0);
	} else if (blend_mode == 5) {
		return vec3(0.5);
	}
	return linear_to_gamma(fog_color);
}

vec3 apply_retail_fog(vec3 color) {
	if (unfogged) {
		return color;
	}
	float distance_to_camera = length(world_position - camera_position);
	float amount = 0.0;
	if (fog_mode == 1) {
		// commonFogFunctions.slang calculateLegacyFog, without height fog.
		float exp_fog = exp(-max(0.0, distance_to_camera - fog_range.x) * fog_density);
		float end_fade = clamp(1.42857146 * (1.0 - distance_to_camera / fog_range.y), 0.0, 1.0);
		amount = 1.0 - min(exp_fog, end_fade);
	}
	return mix(color, retail_fog_color(), amount * fog_opacity);
}

vec4 gamma(vec4 color) {
	return vec4(linear_to_gamma(color.rgb), color.a);
}

// commonFunctions.slang posToTexCoord (sphere env coordinate from view space).
vec2 pos_to_tex_coord(vec3 view_position, vec3 view_normal) {
	vec3 reflection = reflect(normalize(view_position), normalize(view_normal));
	vec3 offset = vec3(reflection.x, reflection.y, reflection.z + 1.0);
	return -(normalize(offset).xy * 0.5) + vec2(0.5);
}

// commonFunctions.slang contangent_frame (fragment variant).
mat3 contangent_frame(vec3 normal, vec3 position, vec2 uv) {
	vec3 dp1 = dFdx(position);
	vec3 dp2 = dFdy(position);
	vec2 duv1 = dFdx(uv);
	vec2 duv2 = dFdy(uv);
	vec3 dp2perp = cross(dp2, normal);
	vec3 dp1perp = cross(normal, dp1);
	vec3 tangent = dp2perp * duv1.x + dp1perp * duv2.x;
	vec3 bitangent = dp2perp * duv1.y + dp1perp * duv2.y;
	float invmax = inversesqrt(max(dot(tangent, tangent), dot(bitangent, bitangent)));
	return mat3(tangent * invmax, bitangent * invmax, normal);
}

struct WmoFragment {
	vec3 diffuse;
	vec3 emissive;
	float opacity;
};

// Fragment inputs: calcWMOVertMat vTexCoord..vTexCoord4, view-space position and
// normal, vColor2.a (second MOCV) and vColorSecond (MOC2 as logical RGBA).
struct WmoInputs {
	vec2 uv1;
	vec2 uv2;
	vec2 uv3;
	vec2 uv4;
	vec3 view_position;
	vec3 view_normal;
	float mocv2_alpha;
	vec4 moc2;
};

// MapObjParallax: commonWMOMaterial.slang caclWMOFragMat case 19, ported literally
// (including its `tex3.b` and texture-3 lookup at MOC2.bg).
WmoFragment parallax_fragment(vec4 tex, vec4 tex3, WmoInputs i) {
	WmoFragment result = WmoFragment(vec3(0.0), vec3(0.0), 1.0);
	vec4 tex_6 = gamma(texture(texture_6, i.uv2));
	mat3 tbn = contangent_frame(i.view_normal, -i.view_position, i.uv2);
	vec3 view_ts = tbn * normalize(-i.view_position);
	vec2 view_ts_norm = view_ts.xy / view_ts.z;
	float bump_offset_middle = 0.5 * tex_6.r;
	vec4 tex_4 = gamma(texture(texture_4, i.uv2 - (view_ts_norm * bump_offset_middle * 0.5)));
	vec4 tex_5 = gamma(texture(texture_5, i.uv3 - (view_ts_norm * bump_offset_middle * 0.5)));
	vec4 tex_3 = gamma(texture(third_texture, i.uv2));
	vec3 diffuse_result = tex_5.rgb + tex_4.rgb * tex_4.a;
	vec3 diffuse_result_2 = (tex_3.rgb - diffuse_result) * tex_6.g + diffuse_result;
	vec3 mix3 = tex_3.rgb * tex_6.b + (tex_5.rgb * tex_5.a * (1.0 - tex3.b));
	// Reference vColorSecond.bg is MOC2 logical (red, green); CUSTOM1 holds RGBA.
	vec4 tex_2 = gamma(texture(third_texture, i.moc2.rg));
	vec3 tex_2_mult = tex_2.rgb * tex_2.a;
	vec3 fake_specular;
	if (i.mocv2_alpha > 0.0) {
		result.diffuse = mix(diffuse_result_2, tex.rgb, i.mocv2_alpha);
		fake_specular = ((tex.rgb * tex.a) - tex_2_mult) * i.mocv2_alpha + tex_2_mult;
	} else {
		fake_specular = tex_2_mult;
		result.diffuse = diffuse_result_2;
	}
	result.emissive = (mix3 - (mix3 * i.mocv2_alpha)) + fake_specular * tex_2.rgb;
	return result;
}

// MapObjDFShader: caclWMOFragMat case 20, a four-layer height blend weighted by MOC2.
// Ported literally, including `weightsMax` skipping the third weight and the
// reference's black ambient-occlusion colour (its AO is a TODO).
WmoFragment df_fragment(WmoInputs i) {
	vec4 tex_1 = gamma(texture(base_texture, pos_to_tex_coord(i.view_position, i.view_normal)));
	vec4 tex_2 = gamma(texture(second_texture, i.uv1));
	vec4 tex_3 = gamma(texture(third_texture, i.uv2));
	vec4 tex_4 = gamma(texture(texture_4, i.uv3));
	vec4 tex_5 = gamma(texture(texture_5, i.uv4));
	vec4 tex_6 = gamma(texture(texture_6, i.uv1));
	vec4 tex_7 = gamma(texture(texture_7, i.uv2));
	vec4 tex_8 = gamma(texture(texture_8, i.uv3));
	vec4 tex_9 = gamma(texture(texture_9, i.uv4));
	// Reference vColorSecond.bgr is MOC2 logical RGB.
	float second_color_sum = dot(i.moc2.rgb, vec3(1.0));
	vec4 weights = vec4(i.moc2.rgb, 1.0 - clamp(second_color_sum, 0.0, 1.0));
	vec4 heights = max(vec4(tex_6.a, tex_7.a, tex_8.a, tex_9.a), 0.004);
	vec4 alpha_vec = weights * heights;
	float weights_max = max(alpha_vec.r, max(alpha_vec.g, max(alpha_vec.r, alpha_vec.a)));
	vec4 alpha_vec_2 = (1.0 - clamp(vec4(weights_max) - alpha_vec, 0.0, 1.0)) * alpha_vec;
	vec4 normalized = alpha_vec_2 * (1.0 / dot(alpha_vec_2, vec4(1.0)));
	vec4 mixed = tex_2 * normalized.r + tex_3 * normalized.g + tex_4 * normalized.b + tex_5 * normalized.a;
	vec3 ambient_occlusion = vec3(0.0);
	return WmoFragment(mix(mixed.rgb, ambient_occlusion, i.moc2.a), (mixed.a * tex_1.rgb) * mixed.rgb, 1.0);
}

// commonWMOMaterial.slang caclWMOFragMat in authored (gamma) space. Specular is
// omitted: the reference calcSpec returns vec3(0).
WmoFragment wmo_fragment(vec4 tex, vec4 tex2, vec4 tex3, WmoInputs i) {
	WmoFragment f = WmoFragment(vec3(0.0), vec3(0.0), 1.0);
	float a2 = i.mocv2_alpha;
	if (pixel_shader == -1) {
		f.diffuse = tex.rgb * tex2.rgb;
		f.opacity = tex.a;
	} else if (pixel_shader == 0 || pixel_shader == 1 || pixel_shader == 2 || pixel_shader == 18) {
		// MapObjDiffuse, MapObjSpecular, MapObjMetal, MapObjLod.
		f.diffuse = tex.rgb;
		f.opacity = tex.a;
	} else if (pixel_shader == 3) { // MapObjEnv
		f.diffuse = tex.rgb;
		f.emissive = tex2.rgb * tex.a;
	} else if (pixel_shader == 4) { // MapObjOpaque
		f.diffuse = tex.rgb;
	} else if (pixel_shader == 5) { // MapObjEnvMetal
		f.diffuse = tex.rgb;
		f.emissive = (tex.rgb * tex.a) * tex2.rgb;
	} else if (pixel_shader == 6) { // MapObjTwoLayerDiffuse
		vec3 layer2 = mix(tex.rgb, tex2.rgb, tex2.a);
		f.diffuse = mix(layer2, tex.rgb, a2);
		f.opacity = tex.a;
	} else if (pixel_shader == 7) { // MapObjTwoLayerEnvMetal
		vec4 color_mix = mix(tex, tex2, 1.0 - a2);
		f.diffuse = color_mix.rgb;
		f.emissive = (color_mix.rgb * color_mix.a) * tex3.rgb;
		f.opacity = tex.a;
	} else if (pixel_shader == 8) { // MapObjTwoLayerTerrain
		f.diffuse = mix(tex2.rgb, tex.rgb, a2);
		f.opacity = tex.a;
	} else if (pixel_shader == 9) { // MapObjDiffuseEmissive
		f.diffuse = tex.rgb;
		f.emissive = tex2.rgb * tex2.a * a2;
		f.opacity = tex.a;
	} else if (pixel_shader == 10) { // MapObjMaskedEnvMetal
		float mix_factor = clamp(tex3.a * a2, 0.0, 1.0);
		f.diffuse = mix(mix((tex.rgb * tex2.rgb) * 2.0, tex3.rgb, mix_factor), tex.rgb, tex.a);
		f.opacity = tex.a;
	} else if (pixel_shader == 11) { // MapObjEnvMetalEmissive
		f.diffuse = tex.rgb;
		f.emissive = ((tex.rgb * tex.a) * tex2.rgb) + ((tex3.rgb * tex3.a) * a2);
		f.opacity = tex.a;
	} else if (pixel_shader == 12) { // MapObjTwoLayerDiffuseOpaque
		f.diffuse = mix(tex2.rgb, tex.rgb, a2);
	} else if (pixel_shader == 13) { // MapObjTwoLayerDiffuseEmissive
		f.diffuse = mix(tex2.rgb * (1.0 - tex2.a), tex.rgb, a2);
		f.emissive = (tex2.rgb * tex2.a) * (1.0 - a2);
		f.opacity = tex.a;
	} else if (pixel_shader == 14) { // MapObjAdditiveMaskedEnvMetal
		f.diffuse = mix((tex.rgb * tex2.rgb * 2.0) + (tex3.rgb * clamp(tex3.a * a2, 0.0, 1.0)), tex.rgb, vec3(tex.a));
	} else if (pixel_shader == 15) { // MapObjTwoLayerDiffuseMod2x
		vec3 layer2 = mix(tex.rgb, tex2.rgb, vec3(tex2.a));
		f.diffuse = mix(layer2, tex.rgb, vec3(a2)) * tex3.rgb * 2.0;
		f.opacity = tex.a;
	} else if (pixel_shader == 16) { // MapObjTwoLayerDiffuseMod2xNA
		f.diffuse = mix(tex.rgb, (tex.rgb * tex2.rgb) * 2.0, vec3(a2));
		f.opacity = tex.a;
	} else if (pixel_shader == 17) { // MapObjTwoLayerDiffuseAlpha
		vec3 layer2 = mix(tex.rgb, tex2.rgb, vec3(tex2.a));
		f.diffuse = (mix(layer2, tex.rgb, vec3(tex3.a)) * tex3.rgb) * 2.0;
		f.opacity = tex.a;
	} else if (pixel_shader == 19) {
		f = parallax_fragment(tex, tex3, i);
	} else if (pixel_shader == 20) {
		f = df_fragment(i);
	}
	return f;
}

void vertex() {
	world_position = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	second_mocv_alpha = has_second_mocv ? CUSTOM2.x : 1.0;
	moc2 = has_moc2 ? CUSTOM1 : vec4(0.0, 0.0, 0.0, 1.0);
	// calcWMOVertMat: missing MOTV sets are (1,1) (wmoGroupGeom.cpp getVBO).
	// MOUV UV animation (makeWmoUVAnimVec) is not ported.
	vec2 uv2 = has_uv2 ? UV2 : vec2(1.0);
	vec2 uv3 = has_uv3 ? CUSTOM0.xy : vec2(1.0);
	fourth_uv = has_uv4 ? CUSTOM0.zw : vec2(1.0);
	second_uv = uv2;
	third_uv = uv3;
	vec3 view_position = (MODELVIEW_MATRIX * vec4(VERTEX, 1.0)).xyz;
	vec3 view_normal = normalize(MODELVIEW_NORMAL_MATRIX * NORMAL);
	if (vertex_shader == 1) { // MapObjDiffuse_T1_Refl
		second_uv = reflect(normalize(view_position), view_normal).xy;
	} else if (vertex_shader == 2) { // MapObjDiffuse_T1_Env_T2
		second_uv = pos_to_tex_coord(view_position, view_normal);
		third_uv = uv2;
	} else if (vertex_shader == 5) { // MapObjDiffuse_Comp_Refl
		third_uv = reflect(normalize(view_position), view_normal).xy;
	} else if (vertex_shader == 6) { // MapObjDiffuse_Comp_Terrain
		second_uv = view_position.xy * 0.239999995;
	}
}

void fragment() {
	vec4 tex = gamma(texture(base_texture, UV) * base_color);
	vec4 tex2 = gamma(texture(second_texture, second_uv));
	vec4 tex3 = gamma(texture(third_texture, third_uv));
	// caclWMOFragMat alpha test on the first texture.
	if (blend_mode == 1 && tex.a < (128.0 / 255.0)) {
		discard;
	}
	WmoInputs inputs = WmoInputs(UV, second_uv, third_uv, fourth_uv, VERTEX, NORMAL, second_mocv_alpha, moc2);
	WmoFragment texel = wmo_fragment(tex, tex2, tex3, inputs);
	// Any ALPHA write moves a Godot material to the unsorted, depth-less
	// transparent pass; only GxBlend 2/3 variants define WMO_BLENDED.
#ifdef WMO_BLENDED
	ALPHA = texel.opacity;
#endif
	ALBEDO = vec3(1.0);
	ROUGHNESS = 1.0;
	world_normal = normalize((INV_VIEW_MATRIX * vec4(NORMAL, 0.0)).xyz);
	camera_position = CAMERA_POSITION_WORLD;
	diffuse_authored = texel.diffuse;
	emission_authored = linear_to_gamma(emissive);
	vec3 mocv = has_mocv ? COLOR.rgb * 2.0 : vec3(0.0);
	exterior_weight = exterior_lit ? 1.0 : (has_mocv ? COLOR.a : 0.0);
	float exterior_dot = clamp(dot(world_normal, -normalize(sun_direction)), 0.0, 1.0);
	// Retail's fixed interior sun only weights the ambients; it adds no direct light.
	vec3 interior_sun = normalize(vec3(-0.30822, -0.9, 0.30822));
	float interior_dot = clamp(dot(world_normal, -interior_sun), 0.0, 1.0);
	vec3 interior = retail_ambient(interior_ambient + mocv, interior_ambient + mocv, interior_ambient + mocv, world_normal, interior_dot);
	vec3 exterior = retail_ambient(ambient + mocv, horizon_ambient + mocv, ground_ambient + mocv, world_normal, exterior_dot);
	// calcLight adds the pixel-shader emissive after lighting; F_UNLIT keeps matDiffuse only.
	ambient_authored = diffuse_authored * mix(interior, exterior, exterior_weight) + emission_authored + texel.emissive;
	if (unlit) {
		ambient_authored = diffuse_authored + emission_authored;
	}
	// Retain interior/ambient when no directional light exists. light() adds only the
	// gamma-encoded difference of shadowed direct, keeping the authored-space sum.
	EMISSION = gamma_to_linear(apply_retail_fog(ambient_authored));
}

void light() {
	if (LIGHT_IS_DIRECTIONAL && !unlit) {
		vec3 normal = normalize(world_normal);
		float n_dot_l = clamp(dot(normal, -normalize(sun_direction)), 0.0, 1.0);
		vec3 with_direct = ambient_authored + diffuse_authored * exterior_weight * direct * n_dot_l * ATTENUATION;
		vec3 lit = gamma_to_linear(apply_retail_fog(with_direct));
		vec3 base = gamma_to_linear(apply_retail_fog(ambient_authored));
		DIFFUSE_LIGHT += lit - base;
	}
#ifndef WMO_BLENDED
	if (!LIGHT_IS_DIRECTIONAL && !unlit) {
		// M2 point lights reach opaque surfaces only (WebWowViewerCpp
		// pointLight.frag.slang:43-58); LIGHT_COLOR carries Godot's PI.
		float n_dot_l = max(dot(NORMAL, LIGHT), 0.0);
		DIFFUSE_LIGHT += gamma_to_linear(diffuse_authored) * LIGHT_COLOR / PI * m2_point_light_attenuation(ATTENUATION, SPECULAR_AMOUNT) * n_dot_l;
	}
#endif
}
