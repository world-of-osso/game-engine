extends "res://tests/toy_effects14_warning_signs_live.gd"
# Frame real loaded sign meshes; do not move the actor or invent a toy destination.
func capture(label: String) -> void:
	if label == "native":
		await super.capture(label)
		return
	var subject: Node3D = null
	for sample in proof_cases():
		if label.begins_with("toy%d-" % int(sample[0])):
			subject = scenery(str(sample[3]))
			break
	if subject == null: fail("loaded source sign missing for camera composition"); return
	var camera := root.get_camera_3d()
	if camera == null: fail("native camera missing"); return
	var bounds := subject_bounds(subject)
	var focus := bounds.get_center()
	var radius := bounds.size.length() / 2.0
	var distance := radius / sin(deg_to_rad(camera.fov / 2.0)) * 1.35
	var eye = unobstructed_eye(camera, focus, distance, bounds.size)
	if eye == null: fail("no unobstructed source-mesh camera direction"); return
	var transform := camera.global_transform
	client.set_process(false)
	camera.global_position = eye
	camera.look_at(focus, Vector3.UP)
	receipts.append({"phase": "source-mesh-camera", "subject": str(subject.global_position), "bounds": str(bounds), "eye": str(eye), "focus": str(focus)})
	await super.capture(label)
	camera.global_transform = transform
	client.set_process(true)

func subject_bounds(subject: Node3D) -> AABB:
	var meshes := subject.find_children("*", "MeshInstance3D", true, false)
	var first := meshes[0] as MeshInstance3D
	var bounds := first.global_transform * first.get_aabb()
	for index in range(1, meshes.size()):
		var mesh := meshes[index] as MeshInstance3D
		bounds = bounds.merge(mesh.global_transform * mesh.get_aabb())
	return bounds

func unobstructed_eye(camera: Camera3D, focus: Vector3, distance: float, size: Vector3):
	var space := camera.get_world_3d().direct_space_state
	# The thin horizontal bounds axis is the sign face normal, not its edge.
	var normal_yaw := PI / 2.0 if size.z < size.x else 0.0
	for index in range(8):
		var yaw := normal_yaw + TAU * float(index) / 8.0
		var direction := Vector3(cos(yaw), 0.4, sin(yaw)).normalized()
		var eye := focus + direction * distance
		var ray := PhysicsRayQueryParameters3D.create(focus, eye)
		ray.collide_with_areas = false
		if space.intersect_ray(ray).is_empty(): return eye
	return null
