shader_type spatial;
render_mode fog_disabled, cull_disabled;

void fragment() {
	// Fixed authored emission. Options must change postprocessing, not this source.
	ALBEDO = vec3(0.0);
	EMISSION = vec3(4.0);
}
