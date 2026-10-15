extends SceneTree

func _initialize() -> void:
	check.call_deferred()

func check() -> void:
	if not ClassDB.class_exists("WowTradingPostMountProbe"):
		push_error("Mounted preview composition is unavailable")
		quit(1)
		return
	var probe = ClassDB.instantiate("WowTradingPostMountProbe")
	if not probe.rejects_missing_saddle():
		push_error("Missing saddle used a fallback or took rider ownership")
		quit(1)
		return
	for cycle in range(3):
		var group: Node3D = probe.compose(root)
		if group == null:
			push_error("Mount composition failed")
			quit(1)
			return
		var mount: Node3D = group.get_node("MountActor/NpcVisualRoot")
		var rider: Node3D = mount.get_node("NpcModel/Skeleton3D/AttachmentBone0/Attachment0/PlayerRiderActor")
		var actor: Node3D = group.get_node("MountActor")
		if not actor.scale.is_equal_approx(Vector3.ONE * 2.8) or not actor.position.is_equal_approx(Vector3(0, -2.8, 0)):
			push_error("Authored normalized mount transform was lost")
			quit(1)
			return
		if not rider.scale.is_equal_approx(Vector3.ONE):
			push_error("Rider did not retain authored scale1")
			quit(1)
			return
		var rider_scale := rider.scale
		var mount_transform := actor.transform
		var display_transform := mount.transform
		if not mount.scale.is_equal_approx(Vector3.ONE * 2.0) or not is_equal_approx(mount.rotation.y, PI / 2):
			push_error("Authored actor overwrote display transform")
			quit(1)
			return
		probe.rotate(1.2)
		if actor.transform != mount_transform or mount.transform != display_transform or rider.scale != rider_scale or not is_equal_approx(group.rotation.y, 1.2):
			push_error("Yaw replaced actor scale or rider transform")
			quit(1)
			return
		probe.hide_rider()
		if is_instance_valid(rider):
			push_error("Hidden rider survived teardown")
			quit(1)
			return
		probe.clear()
		if is_instance_valid(group) or is_instance_valid(mount):
			push_error("Mount actor survived preview teardown")
			quit(1)
			return
		probe.clear()
		await process_frame
	print("PASS: authored mount transform, independent yaw, seated rider ownership, hide and repeated cleanup")
	quit(0)
