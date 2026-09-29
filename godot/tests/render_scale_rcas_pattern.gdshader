shader_type spatial;
render_mode unshaded, fog_disabled, cull_disabled;

void fragment() {
	// Wide, nonzero RGB stripes leave stable edges after 3D upscaling.
	float stripe = step(0.5, fract(UV.x * 32.0));
	ALBEDO = mix(vec3(0.2, 0.27, 0.32), vec3(0.4, 0.36, 0.38), stripe);
}
