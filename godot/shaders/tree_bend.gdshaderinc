// Each M2 placement owns its batch materials; no global instance-uniform slots.
uniform bool tree_bend_enabled = false;
uniform vec3 tree_pivot_0 = vec3(0.0);
uniform vec3 tree_pivot_1 = vec3(0.0);
uniform vec3 tree_rotation_0 = vec3(0.0);
uniform vec3 tree_rotation_1 = vec3(0.0);

vec3 tree_rotate(vec3 vector, vec3 axis, float sine, float cosine) {
	return vector * cosine + cross(axis, vector) * sine
		+ axis * dot(axis, vector) * (1.0 - cosine);
}

// rotation is the model-local rotation axis multiplied by its angle in radians.
void tree_bend(inout vec3 position, inout vec3 normal, vec3 pivot, vec3 rotation, float weight) {
	float angle = length(rotation);
	if (weight <= 0.0 || angle == 0.0) {
		return;
	}
	vec3 axis = rotation / angle;
	float sine = sin(angle * weight);
	float cosine = cos(angle * weight);
	position = pivot + tree_rotate(position - pivot, axis, sine, cosine);
	normal = tree_rotate(normal, axis, sine, cosine);
}
