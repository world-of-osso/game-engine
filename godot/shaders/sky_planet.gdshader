shader_type spatial;
render_mode unshaded, fog_disabled, cull_disabled, blend_mix, depth_draw_never, shadows_disabled;

// A sun or moon disc: WebWowViewerCpp planetShader.vert/.frag.slang. A camera-facing quad
// `scale` yards wide at the node's position, its texture tinted by the exterior specular
// colour (LightData SunColor) and faded as it crosses the camera's horizontal plane.

// Authored texture bytes; the tint multiplies in authored colour, then converts to linear.
uniform sampler2D planet_texture : filter_linear, repeat_disable;
uniform vec3 color = vec3(1.0);
uniform float scale = 1.0;
varying float horizon_fade;

vec3 gamma_to_linear(vec3 c) {
	return mix(c / 12.92, pow((c + 0.055) / 1.055, vec3(2.4)), step(vec3(0.04045), c));
}

void vertex() {
	vec3 right = normalize(INV_VIEW_MATRIX[0].xyz);
	vec3 up = normalize(INV_VIEW_MATRIX[1].xyz);
	vec3 world = MODEL_MATRIX[3].xyz + (right * VERTEX.x + up * VERTEX.y) * scale;
	horizon_fade = clamp((world.y - CAMERA_POSITION_WORLD.y) * 2.5, 0.0, 1.0);
	POSITION = PROJECTION_MATRIX * VIEW_MATRIX * vec4(world, 1.0);
	// Reverse-Z far plane, as the other sky models: behind every world surface.
	POSITION.z = POSITION.w * 0.000001;
}

void fragment() {
	vec4 texel = texture(planet_texture, UV);
	ALBEDO = gamma_to_linear(texel.rgb * color);
	ALPHA = texel.a * horizon_fade;
}
