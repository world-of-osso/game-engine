extends SceneTree

# Real WorldUnits.advance with synthetic native nodes, authored seat5766/2764 rows.
# Proves attachment/follow/release only; no model, network or pixel acceptance.
var fixture: Node3D
var failures: Array[String] = []

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	fixture = ClassDB.instantiate("VehicleAttachmentFixture")
	root.add_child(fixture)
	for npc in [false, true]:
		var previous_failures := failures.size()
		check(fixture.initialize(npc) == "", "initialize host npc=" + str(npc))
		var attachment_id := 39 if npc else 40
		var point := fixture.get_node("WorldUnits/Host/NpcVisualRoot/NpcModel/Skeleton3D/AttachmentBone%d/Attachment%d" % [attachment_id, attachment_id]) as Node3D
		var rider := fixture.get_node("WorldUnits/Rider/PlayerModel") as Node3D
		for pose in [Vector4(10, 2, -20, 0.0), Vector4(-8, 3, 17, 1.2)]:
			fixture.advance_host(Vector3(pose.x, pose.y, pose.z), pose.w)
			check(rider.global_transform.is_equal_approx(point.global_transform), "rider follows authored seat transform npc=" + str(npc) + " rider=" + str(rider.global_transform) + " seat=" + str(point.global_transform))
		check(fixture.release_rider() == "", "release rider")
		fixture.advance_host(Vector3(22, 4, -13), -0.6)
		check(rider.transform.is_equal_approx(Transform3D.IDENTITY), "exit restores rider local transform")
		check(rider.get_parent().name == "Rider", "exit restores independent rider parent")
		if failures.size() == previous_failures:
			print("PASS vehicle_attachment npc=", npc)
	fixture.queue_free()
	await process_frame
	quit(0 if failures.is_empty() else 1)

func check(ok: bool, message: String) -> void:
	if not ok:
		failures.append(message)
		push_error(message)
