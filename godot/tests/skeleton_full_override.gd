extends SceneTree

func _initialize() -> void:
	var skeleton := Skeleton3D.new()
	root.add_child(skeleton)
	skeleton.add_bone("root")
	var shear := Transform3D(Basis(Vector3(1, 0, 0), Vector3(0.5, 1, 0), Vector3(0, 0, 1)), Vector3(1, 2, 3))
	skeleton.set_bone_pose_scale(0, Vector3.ZERO)
	skeleton.set_bone_global_pose_override(0, shear, 1.0, true)
	skeleton.force_update_all_bone_transforms()
	var actual := skeleton.get_bone_global_pose(0)
	if not actual.is_equal_approx(shear):
		push_error("Full override did not preserve the authored affine transform: %s" % actual)
		quit(1)
		return
	skeleton.clear_bones_global_pose_override()
	skeleton.set_bone_pose_scale(0, Vector3.ONE)
	var target := Transform3D(Basis.IDENTITY, Vector3(4, 2, 0))
	skeleton.set_bone_global_pose_override(0, target, 0.5, true)
	skeleton.force_update_all_bone_transforms()
	if not skeleton.get_bone_global_pose(0).is_equal_approx(Transform3D.IDENTITY.interpolate_with(target, 0.5)):
		push_error("Partial override changed its interpolation")
		quit(1)
		return
	print("SKELETON_FULL_OVERRIDE_PASS")
	skeleton.free()
	quit(0)
