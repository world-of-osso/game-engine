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
	var child_data: Dictionary = loader.load_m2_with_skin_fdids("res://../data/models/126487.m2", PackedInt64Array([126494, 126495, 0]))
	if child_data.has("error"):
		stop_failure("animated child: " + child_data.error)
		return
	var point := parent.get_node("Skeleton3D/AttachmentBone0/Attachment0")
	var child: Node3D = child_data.node
	point.add_child(child)
	# Exercise an actual equipped native item, loaded through the production player path.
	var equipped: Dictionary = loader.load_player(1, 0, 1, [{"slot": "MainHand", "item_id": 25, "inventory_type": 21}])
	if equipped.has("error"):
		stop_failure("native equipped item: " + equipped.error)
		return
	var character: Node3D = equipped.node
	root.add_child(character)
	var item := character.find_child("EquipmentMainHand", true, false) as Node3D
	if item == null:
		stop_failure("actual EquipmentMainHand missing")
		return
	var identity := item.get_instance_id()
	item.reparent(point, false)
	freeze_processes(parent)
	freeze_processes(character)
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
			"same_identity": item.get_instance_id() == identity and item.get_parent() == point})
	print("A7_NATIVE_MEASURE ", JSON.stringify(records))
	for index in records.size():
		var active: bool = index < 2 or index == 4
		var record: Dictionary = records[index]
		if record.active != active or not record.same_identity or not record.color_visible:
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
	parent_player.play_sequence(0, true)
	parent_player.advance_time_ms(100.0)
	parent_player.play_sequence(1, true)
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
