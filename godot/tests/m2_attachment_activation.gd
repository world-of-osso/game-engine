extends SceneTree

# Private solar spell1081566 copy has only its attachment tracks changed.
# Child update/callback eligibility and shadow restoration are measured independently
# from color visibility and continuously advancing native clocks.
func _initialize() -> void:
	call_deferred("run_fixture")

func stop_failure(message: String) -> void:
	push_error(message)
	quit(1)

func freeze_processes(node: Node) -> void:
	node.set_process(false)
	for child in node.get_children():
		freeze_processes(child)

func meshes(node: Node) -> Array[GeometryInstance3D]:
	var result: Array[GeometryInstance3D] = []
	if node is GeometryInstance3D:
		result.append(node)
	for child in node.get_children():
		result.append_array(meshes(child))
	return result

func run_fixture() -> void:
	var loader: Object = ClassDB.instantiate("WowAssetLoader")
	var loaded: Dictionary = loader.load_m2(OS.get_environment("M2_ATTACHMENT_FIXTURE"))
	if loaded.has("error"):
		stop_failure("parent spell: " + loaded.error)
		return
	var parent: Node3D = loaded.node
	root.add_child(parent)
	var point := parent.get_node("Skeleton3D/AttachmentBone0/Attachment0")
	# HumanMaleHD contains the authored $SCD marker used by the callback regression.
	# Its actual equipment stays on the production-selected hand attachment.
	var equipped: Dictionary = loader.load_player(1, 0, 1, [{"slot": "MainHand", "item_id": 25, "inventory_type": 21}])
	if equipped.has("error"):
		stop_failure("native equipped item: " + equipped.error)
		return
	var child: Node3D = equipped.node
	point.add_child(child)
	var item := child.find_child("EquipmentMainHand", true, false) as Node3D
	if item == null:
		stop_failure("actual EquipmentMainHand missing")
		return
	var identity := item.get_instance_id()
	var item_parent := item.get_parent()
	freeze_processes(parent)
	var parent_player: Node = parent.get_node("M2Animation")
	var child_player: Node = child.get_node("M2Animation")
	var skeleton := child.get_node("Skeleton3D") as Skeleton3D
	var geometry := meshes(item)
	if geometry.is_empty():
		stop_failure("equipped child has no shadow-casting geometry")
		return
	for mesh in geometry:
		mesh.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_DOUBLE_SIDED
	parent_player.play_sequence(0, true)
	var records := []
	var last_time := 0.0
	for time in [0.0, 99.0, 100.0, 199.0, 200.0]:
		parent_player.advance_time_ms(time - last_time)
		last_time = time
		var before := skeleton.get_bone_pose(1)
		child_player.advance_time_ms(20.0)
		var changed := not before.is_equal_approx(skeleton.get_bone_pose(1))
		var eligible: bool = child_player.callbacks_eligible()
		if eligible != bool(point.get_meta("m2_attachment_active", true)):
			stop_failure("recursive callback gate disagrees with authored attachment")
			return
		records.append({"parent_ms": time, "active": eligible, "pose_changed": changed,
			"child_ms": child_player.animation_phase().time_ms,
			"shadow": geometry[0].cast_shadow, "color_visible": item.is_visible_in_tree(),
			"same_identity": item.get_instance_id() == identity and item.get_parent() == item_parent,
			"missing_control": parent.get_node("Skeleton3D/AttachmentBone15/Attachment15").get_meta("m2_attachment_active", true),
			"constant_control": parent.get_node("Skeleton3D/AttachmentBone16/Attachment16").get_meta("m2_attachment_active", true)})
	print("A7_NATIVE_MEASURE ", JSON.stringify(records))
	for index in records.size():
		var active: bool = index < 2 or index == 4
		var record: Dictionary = records[index]
		if record.active != active or not record.same_identity or not record.color_visible or not record.missing_control or not record.constant_control:
			stop_failure("attachment participation/identity/color at %s" % record)
			return
		if record.shadow != (GeometryInstance3D.SHADOW_CASTING_SETTING_DOUBLE_SIDED if active else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF):
			stop_failure("shadow participation at %s" % record)
			return
		if not active and record.pose_changed:
			stop_failure("inactive child still evaluated its pose: %s" % record)
			return
		if absf(record.child_ms - float(index + 1) * 20.0) > 0.01:
			stop_failure("existing native child clock changed: %s" % record)
			return
	# Sequence1 stays active; switching away from inactive keys must refresh immediately.
	parent_player.play_sequence(1, true)
	parent_player.play_sequence(0, true)
	parent_player.advance_time_ms(100.0)
	if point.get_meta("m2_attachment_active", true):
		stop_failure("sequence-switch precondition did not reach inactive100ms")
		return
	# A newly attached descendant must inherit the inactive gate even when its
	# direct parent's child count is unchanged between animation samples.
	var first := MeshInstance3D.new()
	first.mesh = BoxMesh.new()
	first.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_DOUBLE_SIDED
	item.add_child(first)
	parent_player.advance_time_ms(0.0)
	if first.cast_shadow != GeometryInstance3D.SHADOW_CASTING_SETTING_OFF:
		stop_failure("new descendant under inactive attachment still casts shadows")
		return
	item.remove_child(first)
	first.free()
	var replacement := MeshInstance3D.new()
	replacement.mesh = BoxMesh.new()
	replacement.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_DOUBLE_SIDED
	item.add_child(replacement)
	parent_player.advance_time_ms(0.0)
	if replacement.cast_shadow != GeometryInstance3D.SHADOW_CASTING_SETTING_OFF:
		stop_failure("same-count descendant replacement still casts shadows")
		return
	parent_player.play_sequence(1, true)
	if replacement.cast_shadow != GeometryInstance3D.SHADOW_CASTING_SETTING_DOUBLE_SIDED:
		stop_failure("reactivation did not restore replacement shadow mode")
		return
	if not point.get_meta("m2_attachment_active", true):
		stop_failure("sequence switch retained inactive state")
		return
	# A missing-track attachment and an authored always-active attachment are controls.
	for id in [15, 16]:
		var control := parent.get_node("Skeleton3D/AttachmentBone%d/Attachment%d" % [id, id])
		if not control.get_meta("m2_attachment_active", true):
			stop_failure("always-active/missing-track control became inactive")
			return
	print("PASS A7 native animated child and actual EquipmentMainHand retain identity/color/clock while update/shadow eligibility follows keys; sequence and missing/always controls")
	quit(0)
