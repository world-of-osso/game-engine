extends RefCounted

const POSE_DELTA := 0.03

var animation: WowAnimationPlayer
var skeleton: Skeleton3D

func bind(player: Node3D) -> String:
	var model := player.find_child("PlayerModel", true, false) as Node3D
	if model == null:
		return "Native player lacks PlayerModel"
	animation = model.get_node_or_null("M2Animation") as WowAnimationPlayer
	skeleton = model.get_node_or_null("Skeleton3D") as Skeleton3D
	if animation == null or skeleton == null or skeleton.get_bone_count() == 0:
		return "Native PlayerModel lacks direct M2Animation or authored Skeleton3D"
	return ""

func capture_pose() -> Array[Transform3D]:
	var poses: Array[Transform3D] = []
	for bone in skeleton.get_bone_count():
		poses.append(skeleton.get_bone_pose(bone))
	return poses

func changed_from(stand_pose: Array[Transform3D]) -> bool:
	if skeleton.get_bone_count() != stand_pose.size():
		return false
	for bone in skeleton.get_bone_count():
		var current := skeleton.get_bone_pose(bone)
		var original := stand_pose[bone]
		if current.origin.distance_to(original.origin) > POSE_DELTA:
			return true
		if current.basis.get_rotation_quaternion().angle_to(original.basis.get_rotation_quaternion()) > POSE_DELTA:
			return true
	return false
