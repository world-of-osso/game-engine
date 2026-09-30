shader_type spatial;
render_mode unshaded, fog_disabled, cull_disabled;

void fragment() {
	// High-contrast nonzero stripes retain measurable RCAS edges after bilinear upscaling.
	float stripe = step(0.5, fract(UV.x * 32.0));
	ALBEDO = mix(vec3(0.1, 0.12, 0.14), vec3(0.8, 0.75, 0.7), stripe);
	// Right-side black patch leaves the existing x=80..349 stripe scan untouched.
	if (UV.x > 0.85) {
		ALBEDO = vec3(0.0);
	}
}
