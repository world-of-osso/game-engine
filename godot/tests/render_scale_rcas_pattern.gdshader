shader_type spatial;
render_mode unshaded, fog_disabled, cull_disabled;

void fragment() {
	// High-contrast nonzero stripes retain measurable RCAS edges after bilinear upscaling.
	float stripe = step(0.5, fract(UV.x * 32.0));
	ALBEDO = mix(vec3(0.1, 0.12, 0.14), vec3(0.8, 0.75, 0.7), stripe);
}
