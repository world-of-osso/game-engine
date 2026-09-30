shader_type spatial;
// The binder creates source variants for the authored blend and cull state.
render_mode ambient_light_disabled, fog_disabled, specular_disabled, cull_back, blend_mix;
#include "res://shaders/m2_point_light.gdshaderinc"

uniform int effect_mode = 0; // 0: StandardMaterial-style single, 1: two-texture combiner.
uniform sampler2D base_texture : source_color, repeat_enable;
uniform sampler2D second_texture : source_color, repeat_enable;
uniform vec4 base_color : source_color = vec4(1.0);
// Batch colour track RGB (authored gamma space): the combiners' meshColor.
uniform vec3 mesh_color = vec3(1.0);
uniform int shader_id = 16;
uniform int uv_mode_1 = 0;
uniform int uv_mode_2 = 0;
uniform vec2 uv_offset_1;
uniform vec2 uv_offset_2;
uniform float transparency = 1.0;
// Retail scenery distance fade of the placement (1 unless it is in its fade band).
uniform float scenery_opacity = 1.0;
uniform float alpha_test = 0.0;
uniform int render_flags = 0;
uniform int gx_blend = 0;

// RetailSceneLight, in authored sRGB-byte / 255 space.
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

vec4 combine_textures(vec4 texture1, vec4 texture2, int id) {
	if (id == 0x4014) {
		return clamp(texture1 * texture2 * 2.0, vec4(0.0), vec4(1.0));
	} else if (id == 0x10) {
		return vec4(texture1.rgb * texture2.rgb, texture1.a);
	} else if (id == 0x11) {
		return texture1 * texture2;
	} else if (id == 0x4016) {
		return vec4(clamp(texture1.rgb * texture2.rgb * 2.0, vec3(0.0), vec3(1.0)), texture1.a);
	} else if (id == 0x8015 || id == 0x8002) {
		return vec4(texture1.rgb + texture2.rgb * texture2.a, 1.0);
	} else if (id == 0x8001) {
		return vec4(texture1.rgb * mix(texture2.rgb * 2.0, vec3(1.0), texture1.a), 1.0);
	} else if (id == 0x8003) {
		return vec4(texture1.rgb + texture2.rgb * texture2.a * texture1.a, 1.0);
	}
	// Original WGSL's explicit default branch is texture1.
	return texture1;
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

vec3 retail_fog_color() {
	if (gx_blend == 3 || gx_blend == 10 || gx_blend == 13) {
		return vec3(0.0);
	} else if (gx_blend == 4) {
		return vec3(1.0);
	} else if (gx_blend == 5) {
		return vec3(0.5);
	}
	return linear_to_gamma(fog_color);
}

vec3 apply_retail_fog(vec3 color) {
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

// GxBlend_Mod2x blends DST_COLOR, SRC_COLOR: the destination times twice the colour,
// doubled in authored space so grey 0.5 stays neutral in the linear framebuffer.
vec3 blend_output(vec3 rgb) {
	return gamma_to_linear(gx_blend == 5 ? rgb * 2.0 : rgb);
}

void vertex() {
	world_position = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}

void fragment() {
	vec2 first_uv = (uv_mode_1 == 1 ? UV2 : UV) + uv_offset_1;
	vec4 first = texture(base_texture, first_uv);
	vec4 color;
	if (effect_mode == 1) {
		vec2 second_uv = (uv_mode_2 == 1 ? UV2 : UV) + uv_offset_2;
		vec4 second = texture(second_texture, second_uv);
		color = combine_textures(vec4(linear_to_gamma(first.rgb), first.a), vec4(linear_to_gamma(second.rgb), second.a), shader_id);
		color.rgb *= mesh_color;
		color.a = clamp(color.a * transparency, 0.0, 1.0);
	} else {
		vec3 linear_diffuse = first.rgb * base_color.rgb * COLOR.rgb;
		color = vec4(linear_to_gamma(linear_diffuse) * mesh_color, clamp(first.a * base_color.a * COLOR.a * transparency, 0.0, 1.0));
	}
	if (color.a < alpha_test) {
		discard;
	}
	ALBEDO = vec3(1.0);
	ROUGHNESS = 1.0;
	ALPHA = color.a * scenery_opacity;
	authored_color = color.rgb;
	camera_position = CAMERA_POSITION_WORLD;
	if ((render_flags & 1) != 0) {
		vec3 rgb = color.rgb;
		if ((render_flags & 2) == 0) {
			rgb = apply_retail_fog(rgb);
		}
		EMISSION = blend_output(rgb);
	}
	world_normal = normalize((INV_VIEW_MATRIX * vec4(NORMAL, 0.0)).xyz);
}

void light() {
	if (LIGHT_IS_DIRECTIONAL && (render_flags & 1) == 0) {
		vec3 rgb = authored_color;
		vec3 normal = normalize(world_normal);
		float n_dot_l = clamp(dot(normal, -normalize(sun_direction)), 0.0, 1.0);
		vec3 light_color = retail_ambient(ambient, horizon_ambient, ground_ambient, normal, n_dot_l) + direct * n_dot_l * ATTENUATION;
		if (exterior_blend < 1.0) {
			float interior_dot = clamp(dot(normal, -normalize(interior_sun_direction)), 0.0, 1.0);
			vec3 interior = retail_ambient(interior_ambient, interior_horizon_ambient, interior_ground_ambient, normal, interior_dot) + interior_direct * interior_dot;
			light_color = mix(interior, light_color, exterior_blend);
		}
		rgb *= light_color;
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
