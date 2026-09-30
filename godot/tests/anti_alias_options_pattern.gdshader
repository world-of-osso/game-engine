shader_type spatial;
render_mode unshaded, fog_disabled, cull_disabled;

void fragment() {
	// Opaque saturated white: partial pixels must come from geometric coverage.
	ALBEDO = vec3(0.0);
	EMISSION = vec3(1.0);
}
