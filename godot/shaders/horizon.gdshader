shader_type spatial;
// The WDL horizon (solarityclient low-detail.vert/.frag, client 7D5E70: fully fogged): the
// map's distant terrain in the scene fog's colour, in front of the sky and behind all
// world geometry nearer than about 980 yards.
render_mode unshaded, cull_disabled, fog_disabled, shadows_disabled;
#include "res://shaders/retail_fog.gdshaderinc"

varying vec3 horizon_world_position;

vec3 gamma_to_linear(vec3 gamma_color) {
	vec3 low = gamma_color / 12.92;
	vec3 high = pow((max(gamma_color, vec3(0.0)) + 0.055) / 1.055, vec3(2.4));
	return mix(high, low, lessThanEqual(gamma_color, vec3(0.04045)));
}

void vertex() {
	horizon_world_position = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	POSITION = PROJECTION_MATRIX * MODELVIEW_MATRIX * vec4(VERTEX, 1.0);
	// Reverse-Z: nearer than the sky models (w * 1e-6), farther than the 0.1/1000-yard
	// camera's geometry inside ~980 yards.
	POSITION.z = POSITION.w * 0.000002;
}

void fragment() {
	vec3 relative = horizon_world_position - CAMERA_POSITION_WORLD;
	vec2 visibility = retail_fog_visibility(relative, horizon_world_position.y);
	ALBEDO = gamma_to_linear(retail_fog_color_at(relative, visibility.y, 0));
}
