extends SceneTree

const DATA := "res://../data/models/"
const EPSILON := 0.001

func fail(message: String) -> void:
	push_error(message)
	quit(1)

func _initialize() -> void:
	if not ClassDB.class_exists("WowAssetLoader"):
		fail("WowAssetLoader not registered")
		return
	check_models.call_deferred()

func check_models() -> void:
	var loader = ClassDB.instantiate("WowAssetLoader")
	if not await check_attachment(loader, "humanmale_hd", 5, 189, Vector3(-0.026222223, 1.7228644, 0.20848155), 65535):
		return
	if not await check_attachment(loader, "boar", 0, 48, Vector3(-0.36472428, 1.2703203, 0.0), 65535):
		return
	print("PASS: HD and boar authored attachment lookup, rest offset, and animated bone pose")
	loader = null
	quit(0)

func check_attachment(loader: Object, model_name: String, attachment_id: int, bone_index: int, authored_position: Vector3, absent_id: int) -> bool:
	var result: Dictionary = loader.load_m2(DATA + model_name + ".m2")
	if result.has("error"):
		fail(model_name + " load: " + str(result.error))
		return false
	var model: Node3D = result.node
	get_root().add_child(model)
	var skeleton := model.get_node("Skeleton3D") as Skeleton3D
	var attachment := skeleton.find_child("Attachment%d" % attachment_id, true, false) as Node3D
	if attachment == null or (attachment.get_parent() as BoneAttachment3D).bone_idx != bone_index:
		fail(model_name + " attachment %d must follow bone %d" % [attachment_id, bone_index])
		return false
	if model_name == "humanmale_hd":
		for id in [0, 1, 2]:
			var hand := skeleton.find_child("Attachment%d" % id, true, false) as Node3D
			if hand == null or (hand.get_parent() as BoneAttachment3D).bone_idx != [201, 206, 211][id]:
				fail("HD wrist/palm attachment %d mapped to wrong bone" % id)
				return false
	if skeleton.find_child("Attachment%d" % absent_id, true, false) != null:
		fail(model_name + " missing attachment ID %d was fabricated" % absent_id)
		return false
	(model.get_node_or_null("M2Animation") as Node).set_process(false)
	skeleton.reset_bone_poses()
	await process_frame
	if attachment.global_position.distance_to(authored_position) > EPSILON:
		fail(model_name + " attachment rest position: %s != %s" % [attachment.global_position, authored_position])
		return false
	var pivot := skeleton.get_bone_global_rest(bone_index).origin
	var local_offset := authored_position - pivot
	if attachment.position.distance_to(local_offset) > EPSILON:
		fail(model_name + " local offset does not preserve authored position relative to pivot")
		return false
	skeleton.set_bone_pose_rotation(bone_index, Quaternion(Vector3.UP, 0.65))
	await process_frame
	var expected := skeleton.get_bone_global_pose(bone_index) * local_offset
	if attachment.global_position.distance_to(expected) > EPSILON:
		fail(model_name + " attachment did not follow rotated bone: %s != %s" % [attachment.global_position, expected])
		return false
	if attachment.global_position.distance_to(authored_position) < EPSILON:
		fail(model_name + " attachment did not move")
		return false
	model.free()
	result.clear()
	return true
