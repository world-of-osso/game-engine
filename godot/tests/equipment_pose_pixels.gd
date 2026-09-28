extends RefCounted

const DIAGNOSTICS := "res://../data/diagnostics/godot-conversion/"
const MIN_CHANGED_SAMPLES := 5

func check(tree: SceneTree, preview: Node3D) -> String:
	var chest := preview.get_node_or_null("EquipmentChest") as Node3D
	var skeleton := preview.get_node_or_null("Skeleton3D") as Skeleton3D
	var animation := preview.get_node_or_null("M2Animation") as WowAnimationPlayer
	if chest == null or skeleton == null or animation == null:
		return "Collection preview lacks chest, character skeleton, or animation"
	var joint := find_weighted_chest_joint(chest, skeleton)
	if joint < 0:
		return "Collection chest has no visible skinned mesh with a weighted character joint"
	var scene := preview.get_parent() as Node3D
	if scene == null:
		return "Collection preview has no scene to isolate"
	var prior_visibility := {}
	for node in scene.find_children("*", "GeometryInstance3D", true, false):
		if not chest.is_ancestor_of(node):
			prior_visibility[node] = node.visible
			node.visible = false
	var was_paused := tree.paused
	var was_processing := animation.is_processing()
	var was_chest_visible := chest.visible
	var original_pose := skeleton.get_bone_pose(joint)
	animation.set_paused(true)
	animation.set_process(false)
	tree.paused = true
	var result := await probe_chest_pixels(tree, chest, skeleton, joint)
	skeleton.set_bone_pose(joint, original_pose)
	chest.visible = was_chest_visible
	for node in prior_visibility:
		node.visible = prior_visibility[node]
	animation.set_process(was_processing)
	animation.set_paused(false)
	tree.paused = was_paused
	return result

func find_weighted_chest_joint(chest: Node3D, skeleton: Skeleton3D) -> int:
	var scores := {}
	for node in chest.find_children("*", "MeshInstance3D", true, false):
		var instance := node as MeshInstance3D
		if not instance.is_visible_in_tree() or instance.mesh == null or instance.skin == null:
			continue
		for surface in instance.mesh.get_surface_count():
			var arrays := instance.mesh.surface_get_arrays(surface)
			var bones: PackedInt32Array = arrays[Mesh.ARRAY_BONES]
			var weights: PackedFloat32Array = arrays[Mesh.ARRAY_WEIGHTS]
			for index in min(bones.size(), weights.size()):
				if weights[index] <= 0.01 or bones[index] < 0 or bones[index] >= instance.skin.get_bind_count():
					continue
				var joint := instance.skin.get_bind_bone(bones[index])
				if joint >= 0 and joint < skeleton.get_bone_count():
					scores[joint] = scores.get(joint, 0.0) + weights[index]
	var best_joint := -1
	var best_weight := 0.0
	for joint in scores:
		if scores[joint] > best_weight:
			best_joint = joint
			best_weight = scores[joint]
	return best_joint

func probe_chest_pixels(tree: SceneTree, chest: Node3D, skeleton: Skeleton3D, joint: int) -> String:
	chest.visible = false
	var without_chest := await capture_frame(tree)
	var baseline_path := DIAGNOSTICS + "collection-chest-hidden.png"
	var save_error := without_chest.save_png(baseline_path)
	if save_error != OK:
		return "Could not save collection chest hidden screenshot: " + error_string(save_error)
	chest.visible = true
	var resting := await capture_frame(tree)
	var rest_path := DIAGNOSTICS + "collection-chest-rest.png"
	save_error = resting.save_png(rest_path)
	if save_error != OK:
		return "Could not save collection chest rest screenshot: " + error_string(save_error)
	var chest_pixels := count_changed_pixels(without_chest, resting)
	if chest_pixels < MIN_CHANGED_SAMPLES:
		return "Isolated collection chest contributes too few GPU pixels: " + str(chest_pixels)
	var rotation := Quaternion(Vector3.UP, 0.65) * skeleton.get_bone_pose_rotation(joint)
	skeleton.set_bone_pose_rotation(joint, rotation)
	skeleton.set_bone_pose_position(joint, skeleton.get_bone_pose_position(joint) + Vector3(0.6, 0.2, 0.0))
	var posed := await capture_frame(tree)
	var posed_path := DIAGNOSTICS + "collection-chest-posed.png"
	save_error = posed.save_png(posed_path)
	if save_error != OK:
		return "Could not save collection chest posed screenshot: " + error_string(save_error)
	var pose_pixels := count_changed_pixels(resting, posed)
	if pose_pixels < MIN_CHANGED_SAMPLES:
		return "Weighted joint " + str(joint) + " did not deform isolated chest GPU pixels: " + str(pose_pixels)
	print("PASS: collection chest visible samples=", chest_pixels, " weighted joint=", joint, " posed samples=", pose_pixels, " hidden=", baseline_path, " rest=", rest_path, " posed=", posed_path)
	return ""

func capture_frame(tree: SceneTree) -> Image:
	for frame in 2:
		await RenderingServer.frame_post_draw
	return tree.root.get_texture().get_image()

func count_changed_pixels(first: Image, second: Image) -> int:
	var changed := 0
	for y in range(0, first.get_height(), 2):
		for x in range(0, first.get_width(), 2):
			if not first.get_pixel(x, y).is_equal_approx(second.get_pixel(x, y)):
				changed += 1
	return changed
