// Retail's M2 point-light attenuation from Godot's omni ATTENUATION, for light().
//
// WebWowViewerCpp pointLight.frag.slang:51-58: full inside the attenuation start, then
// linear to none at its end, and the attenuated colour is squared:
//   attenuation = 1 - clamp((d - start) / (end - start), 0, 1)
//   light = (attenuation * color)^2 * N.L
// Godot's colour linearisation stands in for squaring the colour; this returns the
// squared attenuation. The lights (assets/m2_lights.rs) have range = end, attenuation
// exponent 0 and no shadow, so ATTENUATION = (1 - x^4)^2 with x = d / end
// (scene_forward_lights_inc.glsl get_omni_attenuation), and carry start / end in their
// specular parameter, which reaches light() as SPECULAR_AMOUNT = 2 * specular.
float m2_point_light_attenuation(float godot_attenuation, float specular_amount) {
	float x = sqrt(sqrt(max(1.0 - sqrt(godot_attenuation), 0.0)));
	float start = specular_amount * 0.5;
	float ramp = 1.0 - clamp((x - start) / max(1.0 - start, 1e-4), 0.0, 1.0);
	return ramp * ramp;
}
