shader_type spatial;
render_mode unshaded;

void fragment() {
	float checker = mod(floor(UV.x * 48.0) + floor(UV.y * 48.0), 2.0);
	ALBEDO = vec3(checker);
}
