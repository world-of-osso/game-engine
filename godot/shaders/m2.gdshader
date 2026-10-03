shader_type spatial;
// The binder creates source variants for the authored blend, cull and depth state.
render_mode ambient_light_disabled, fog_disabled, specular_disabled, cull_back, blend_mix;
#include "res://shaders/m2_point_light.gdshaderinc"
#include "res://shaders/tree_bend.gdshaderinc"

// Retail M2 batch shading after WebWowViewerCpp 1a8cccb: calcM2VertexMat and
// calcM2FragMaterial (commonM2Material.slang), the mesh colour, texture weights and
// specular-after-light of m2shader_text.slang, and calcLight (commonLightFunctions.slang).
// Textures hold authored bytes and are sampled unconverted, as the reference's UNORM
// textures are; every colour below is authored (gamma) space until blend_output.
uniform sampler2D base_texture : filter_linear_mipmap, repeat_enable;
uniform sampler2D second_texture : filter_linear_mipmap, repeat_enable;
uniform sampler2D third_texture : filter_linear_mipmap, repeat_enable;
uniform sampler2D fourth_texture : filter_linear_mipmap, repeat_enable;
// M2Texture flags of texture slot n at bits 2n (wrap U) and 2n + 1 (wrap V); an axis
// without its flag clamps to the edge.
uniform int texture_wrap = 255;
// getPixelShaderId / getVertexShaderId (m2Object.cpp).
uniform int pixel_shader = 0;
uniform int vertex_shader = 0;
// Texture transforms of the batch's first two matrix slots: (u, v, 1) -> (u', v', 1).
uniform mat3 texture_matrix_1 = mat3(1.0);
uniform mat3 texture_matrix_2 = mat3(1.0);
// uTexSampleAlpha: the batch's texture weights 0..2.
uniform vec3 texture_weights = vec3(1.0);
// Batch colour track RGB: the combiners' meshColor.
uniform vec3 mesh_color = vec3(1.0);
// meshOpacity: colour alpha times texture weight 0 (unless the batch skips it).
uniform float transparency = 1.0;
// Retail scenery distance fade of the placement (1 unless it is in its fade band).
uniform float scenery_opacity = 1.0;
uniform int render_flags = 0;
uniform int gx_blend = 0;

// RetailSceneLight, in authored sRGB-byte / 255 space.
uniform vec3 ambient = vec3(1.0);
uniform vec3 horizon_ambient = vec3(1.0);
uniform vec3 ground_ambient = vec3(1.0);
uniform vec3 direct;
uniform vec3 sun_direction = vec3(0.0, -1.0, 0.0);
#include "res://shaders/retail_fog.gdshaderinc"

// WMO doodad interior light (WebWowViewerCpp M2 intLight, m2Object.cpp:1405-1432):
// calcLight mixes interior and exterior light by exterior_blend; 1 is exterior only.
uniform float exterior_blend = 1.0;
uniform vec3 interior_ambient;
uniform vec3 interior_horizon_ambient;
uniform vec3 interior_ground_ambient;
uniform vec3 interior_direct;
// The direction interior light travels: the scene default (-0.30822, -0.30822, -0.9)
// in WoW axes, or the doodad's personal direction from its MOLT light or group.
uniform vec3 interior_sun_direction = vec3(-0.30822, -0.9, 0.30822);

varying vec3 world_position;
varying vec3 world_normal;
varying vec3 camera_position;
varying vec3 authored_color;
varying vec3 authored_specular;

vec3 gamma_to_linear(vec3 gamma_color) {
	vec3 low = gamma_color / 12.92;
	vec3 high = pow((max(gamma_color, vec3(0.0)) + 0.055) / 1.055, vec3(2.4));
	return mix(high, low, lessThanEqual(gamma_color, vec3(0.04045)));
}

// posToTexCoord (commonFunctions.slang), from the view-space eye-to-vertex vector.
vec2 environment_coord(vec3 view_position, vec3 normal) {
	vec3 reflection = reflect(normalize(view_position), normalize(normal));
	vec3 lifted = vec3(reflection.x, reflection.y, reflection.z + 1.0);
	return -(normalize(lifted).xy * 0.5) + vec2(0.5);
}

// edgeScan (commonFunctions.slang).
float edge_scan(vec3 view_position, vec3 normal) {
	float facing = clamp(dot(-normalize(view_position), normal), 0.0, 1.0);
	return clamp(2.7 * facing * facing - 0.4, 0.0, 1.0);
}

vec2 transformed(mat3 matrix, vec2 uv) {
	return (matrix * vec3(uv, 1.0)).xy;
}

// Clamp-to-edge on axes whose wrap flag is clear; the samplers repeat.
vec4 sample_slot(sampler2D slot_texture, vec2 uv, int slot) {
	int wrap = (texture_wrap >> (slot * 2)) & 3;
	if (wrap != 3) {
		vec2 half_texel = 0.5 / vec2(textureSize(slot_texture, 0));
		vec2 clamped = clamp(uv, half_texel, 1.0 - half_texel);
		uv = vec2((wrap & 1) != 0 ? uv.x : clamped.x, (wrap & 2) != 0 ? uv.y : clamped.y);
	}
	return texture(slot_texture, uv);
}

// makeFog2 with the batch's GX blend.
vec3 apply_retail_fog(vec3 color) {
	return retail_fog(color, world_position, camera_position, gx_blend);
}

// commonLightFunctions.slang applyAndMixAmbients, without precomputed light.
vec3 retail_ambient(vec3 sky, vec3 horizon, vec3 ground, vec3 normal, float n_dot_l) {
	float n_dot_up = dot(normal, vec3(0.0, 1.0, 0.0));
	vec3 hemisphere;
	if (n_dot_up >= 0.0) {
		hemisphere = mix(horizon, sky, n_dot_up);
	} else {
		hemisphere = mix(horizon, ground, -n_dot_up);
	}
	return mix(hemisphere * 0.7, hemisphere * 1.1, 0.5 + 0.5 * n_dot_l);
}

// The written colour, clamped like the reference's UNORM attachment. GxBlend_Mod2x blends
// DST_COLOR, SRC_COLOR: the destination times twice the colour, doubled in authored
// space so grey 0.5 stays neutral in the linear framebuffer.
vec3 blend_output(vec3 rgb) {
	rgb = clamp(rgb, vec3(0.0), vec3(1.0));
	return gamma_to_linear(gx_blend == 5 ? rgb * 2.0 : rgb);
}

void vertex() {
	if (tree_bend_enabled) {
		tree_bend(VERTEX, NORMAL, tree_pivot_0, tree_rotation_0, COLOR.r);
		tree_bend(VERTEX, NORMAL, tree_pivot_1, tree_rotation_1, COLOR.g);
	}
	world_position = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}

void fragment() {
	// forEachVisibleMesh skips a batch while its opacity is ~0.
	if (transparency < 0.0001) {
		discard;
	}
	// Godot flips the normal of back faces; the reference shades both sides with it.
	vec3 normal = normalize(FRONT_FACING ? NORMAL : -NORMAL);
	vec2 env = environment_coord(VERTEX, normal);
	vec2 t1 = transformed(texture_matrix_1, UV);
	vec2 t2 = transformed(texture_matrix_2, UV2);
	vec2 uv1 = t1;
	vec2 uv2 = vec2(0.0);
	vec2 uv3 = vec2(0.0);
	float edge_fade = 1.0;
	switch (vertex_shader) {
		case 1: uv1 = env; break; // Diffuse_Env
		case 2: uv2 = t2; break; // Diffuse_T1_T2
		case 3: uv2 = env; break; // Diffuse_T1_Env
		case 4: uv1 = env; uv2 = t1; break; // Diffuse_Env_T1
		case 5: uv1 = env; uv2 = env; break; // Diffuse_Env_Env
		case 6: uv2 = env; uv3 = t1; break; // Diffuse_T1_Env_T1
		case 7: uv2 = t1; break; // Diffuse_T1_T1
		case 8: uv2 = t1; uv3 = t1; break; // Diffuse_T1_T1_T1
		case 9: edge_fade = edge_scan(VERTEX, normal); break; // Diffuse_EdgeFade_T1
		case 10: uv1 = t2; break; // Diffuse_T2
		case 11: uv2 = env; uv3 = t2; break; // Diffuse_T1_Env_T2
		case 12: uv2 = t2; edge_fade = edge_scan(VERTEX, normal); break; // Diffuse_EdgeFade_T1_T2
		case 13: uv1 = env; edge_fade = edge_scan(VERTEX, normal); break; // Diffuse_EdgeFade_Env
		case 14: uv2 = t2; uv3 = t1; break; // Diffuse_T1_T2_T1
		case 15: uv2 = t2; break; // Diffuse_T1_T2_T3
		case 16: uv1 = t2; break; // Color_T1_T2_T3
		default: break; // Diffuse_T1, BW_Diffuse_T1(_T2)
	}
	vec4 mesh = vec4(mesh_color, transparency) * edge_fade;
	vec3 w = texture_weights;
	// Shaders 26 and 28 sample their crossfade layers at the first coordinate.
	bool crossfade = pixel_shader == 26 || pixel_shader == 28;
	vec4 tex = sample_slot(base_texture, uv1, 0);
	vec4 tex2 = sample_slot(second_texture, crossfade ? uv1 : uv2, 1);
	vec4 tex3 = sample_slot(third_texture, crossfade ? uv1 : uv3, 2);
	vec3 diffuse = mesh.rgb * tex.rgb;
	vec3 specular = vec3(0.0);
	float discard_alpha = 1.0;
	bool can_discard = false;
	switch (pixel_shader) {
		case 1: discard_alpha = tex.a; can_discard = true; break; // Combiners_Mod
		case 2: diffuse *= tex2.rgb; discard_alpha = tex2.a; can_discard = true; break; // Opaque_Mod
		case 3: diffuse *= tex2.rgb * 2.0; discard_alpha = tex2.a * 2.0; can_discard = true; break; // Opaque_Mod2x
		case 4: diffuse *= tex2.rgb * 2.0; break; // Opaque_Mod2xNA
		case 5: diffuse *= tex2.rgb; break; // Opaque_Opaque
		case 6: diffuse *= tex2.rgb; discard_alpha = tex.a * tex2.a; can_discard = true; break; // Mod_Mod
		case 7: diffuse *= tex2.rgb * 2.0; discard_alpha = tex.a * tex2.a * 2.0; can_discard = true; break; // Mod_Mod2x
		case 8: discard_alpha = tex.a + tex2.a; can_discard = true; specular = tex2.rgb; break; // Mod_Add
		case 9: diffuse *= tex2.rgb * 2.0; discard_alpha = tex.a; can_discard = true; break; // Mod_Mod2xNA
		case 10: discard_alpha = tex.a; can_discard = true; specular = tex2.rgb; break; // Mod_AddNA
		case 11: diffuse *= tex2.rgb; discard_alpha = tex.a; can_discard = true; break; // Mod_Opaque
		case 12: diffuse = mesh.rgb * mix(tex.rgb * tex2.rgb * 2.0, tex.rgb, tex.a); break; // Opaque_Mod2xNA_Alpha
		case 13: specular = tex2.rgb * tex2.a; break; // Opaque_AddAlpha
		case 14: specular = tex2.rgb * tex2.a * (1.0 - tex.a); break; // Opaque_AddAlpha_Alpha
		case 15: // Opaque_Mod2xNA_Alpha_Add
			diffuse = mesh.rgb * mix(tex.rgb * tex2.rgb * 2.0, tex.rgb, tex.a);
			specular = tex3.rgb * tex3.a * w.b;
			break;
		case 16: discard_alpha = tex.a; can_discard = true; specular = tex2.rgb * tex2.a; break; // Mod_AddAlpha
		case 17: // Mod_AddAlpha_Alpha
			discard_alpha = tex.a + tex2.a * (0.3 * tex2.r + 0.59 * tex2.g + 0.11 * tex2.b);
			can_discard = true;
			specular = tex2.rgb * tex2.a * (1.0 - tex.a);
			break;
		case 18: diffuse = mesh.rgb * mix(mix(tex.rgb, tex2.rgb, tex2.a), tex.rgb, tex.a); break; // Opaque_Alpha_Alpha
		case 19: diffuse = mesh.rgb * mix(tex.rgb * tex2.rgb * 2.0, tex3.rgb, tex3.a); break; // Opaque_Mod2xNA_Alpha_3s
		case 20: specular = tex2.rgb * tex2.a * w.g; break; // Opaque_AddAlpha_Wgt
		case 21: discard_alpha = tex.a + tex2.a; can_discard = true; specular = tex2.rgb * (1.0 - tex.a); break; // Mod_Add_Alpha
		case 22: diffuse = mesh.rgb * mix(tex.rgb * tex2.rgb, tex.rgb, tex.a); break; // Opaque_ModNA_Alpha
		case 23: discard_alpha = tex.a; can_discard = true; specular = tex2.rgb * tex2.a * w.g; break; // Mod_AddAlpha_Wgt
		case 24: diffuse = mesh.rgb * mix(tex.rgb, tex2.rgb, tex2.a); specular = tex.rgb * tex.a * w.r; break; // Opaque_Mod_Add_Wgt
		case 25: { // Opaque_Mod2xNA_Alpha_UnshAlpha
			float glow = clamp(tex3.a * w.b, 0.0, 1.0);
			diffuse = mesh.rgb * mix(tex.rgb * tex2.rgb * 2.0, tex.rgb, tex.a) * (1.0 - glow);
			specular = tex3.rgb * glow;
			break;
		}
		case 26: { // Mod_Dual_Crossfade
			vec4 layered = mix(mix(tex, tex2, clamp(w.g, 0.0, 1.0)), tex3, clamp(w.b, 0.0, 1.0));
			diffuse = mesh.rgb * layered.rgb;
			discard_alpha = layered.a;
			can_discard = true;
			break;
		}
		case 28: { // Mod_Masked_Dual_Crossfade: its mask samples the second coordinate.
			vec4 layered = mix(mix(tex, tex2, clamp(w.g, 0.0, 1.0)), tex3, clamp(w.b, 0.0, 1.0));
			diffuse = mesh.rgb * layered.rgb;
			discard_alpha = layered.a * sample_slot(fourth_texture, uv2, 3).a;
			can_discard = true;
			break;
		}
		case 27: diffuse = mesh.rgb * mix(mix(tex.rgb * tex2.rgb * 2.0, tex3.rgb, tex3.a), tex.rgb, tex.a); break; // Opaque_Mod2xNA_Alpha_Alpha
		case 29: diffuse = mesh.rgb * mix(tex.rgb, tex2.rgb, tex2.a); break; // Opaque_Alpha
		// Guild shaders with the reference's unit generic parameters.
		case 30: diffuse = mesh.rgb * mix(tex.rgb * mix(vec3(1.0), tex2.rgb, tex2.a), tex3.rgb, tex3.a); discard_alpha = tex.a; can_discard = true; break;
		case 31: diffuse = mesh.rgb * tex.rgb * mix(vec3(1.0), tex2.rgb, tex2.a); discard_alpha = tex.a; can_discard = true; break;
		case 32: diffuse = mesh.rgb * mix(tex.rgb * mix(vec3(1.0), tex2.rgb, tex2.a), tex3.rgb, tex3.a); break;
		case 33: discard_alpha = tex.a; can_discard = true; break; // Mod_Depth
		case 34: diffuse = vec3(0.0); discard_alpha = tex.a; can_discard = true; break; // Illum
		case 35: { // Mod_Mod_Mod_Const
			vec4 product = tex * tex2 * tex3;
			diffuse = mesh.rgb * product.rgb;
			discard_alpha = product.a;
			can_discard = true;
			break;
		}
		case 36: diffuse *= tex2.rgb; discard_alpha = tex.a * tex2.a; can_discard = true; break; // Mod_Mod_Depth
		default: break; // Combiners_Opaque
	}
	if (gx_blend == 1 && can_discard && discard_alpha < 0.501960814) {
		discard;
	}
	// The reference's UNORM colour attachment clamps the written alpha and colour.
	vec4 color = vec4(diffuse, clamp(gx_blend <= 1 ? mesh.a : discard_alpha * mesh.a, 0.0, 1.0));
	ALBEDO = vec3(1.0);
	ROUGHNESS = 1.0;
	ALPHA = color.a * scenery_opacity;
	authored_color = color.rgb;
	authored_specular = specular * mesh.rgb;
	camera_position = CAMERA_POSITION_WORLD;
	if ((render_flags & 1) != 0) {
		vec3 rgb = color.rgb + authored_specular;
		if ((render_flags & 2) == 0) {
			rgb = apply_retail_fog(rgb);
		}
		EMISSION = blend_output(rgb);
	}
	world_normal = normalize((INV_VIEW_MATRIX * vec4(normal, 0.0)).xyz);
}

void light() {
	if (LIGHT_IS_DIRECTIONAL && (render_flags & 1) == 0) {
		vec3 normal = normalize(world_normal);
		float n_dot_l = clamp(dot(normal, -normalize(sun_direction)), 0.0, 1.0);
		vec3 light_color = retail_ambient(ambient, horizon_ambient, ground_ambient, normal, n_dot_l) + direct * n_dot_l * ATTENUATION;
		if (exterior_blend < 1.0) {
			float interior_dot = clamp(dot(normal, -normalize(interior_sun_direction)), 0.0, 1.0);
			vec3 interior = retail_ambient(interior_ambient, interior_horizon_ambient, interior_ground_ambient, normal, interior_dot) + interior_direct * interior_dot;
			light_color = mix(interior, light_color, exterior_blend);
		}
		// calcLight: (matDiffuse * light) + specular.
		vec3 rgb = authored_color * light_color + authored_specular;
		if ((render_flags & 2) == 0) {
			rgb = apply_retail_fog(rgb);
		}
		DIFFUSE_LIGHT += blend_output(rgb);
	} else if (!LIGHT_IS_DIRECTIONAL && (render_flags & 1) == 0 && gx_blend <= 1) {
		// M2 point lights reach opaque surfaces only: WebWowViewerCpp applies them in
		// its deferred pass (pointLight.frag.slang:43-58). LIGHT_COLOR carries Godot's PI.
		float n_dot_l = max(dot(NORMAL, LIGHT), 0.0);
		DIFFUSE_LIGHT += gamma_to_linear(authored_color) * LIGHT_COLOR / PI * m2_point_light_attenuation(ATTENUATION, SPECULAR_AMOUNT) * n_dot_l;
	}
}
