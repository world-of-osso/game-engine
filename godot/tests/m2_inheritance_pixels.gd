extends SceneTree

# Authentic silithid tank boss FDID125913: bone67 flags4, parent12.
# The baseline is the same fixed sampled pose with global overrides cleared:
# exactly the pre-A6 ordinary Skeleton3D hierarchy, not a second executable.
const FDID := 125913
const FLAGGED_BONE := 67
const SIZE := Vector2i(1920, 1080)

func _initialize() -> void:
	call_deferred("run_fixture")

func fail_fixture(message: String) -> void:
	push_error(message)
	quit(1)

func run_fixture() -> void:
	root.size = SIZE
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	for frame in 600:
		if client.get_node_or_null("M2Debug") != null:
			break
		await process_frame
	var stage := client.get_node_or_null("M2Debug")
	if stage == null:
		fail_fixture("m2debug stage did not start")
		return
	stage.set_process(false)
	var old := stage.get_node("M2DebugReferenceModel")
	stage.remove_child(old)
	old.queue_free()
	var loaded: Dictionary = ClassDB.instantiate("WowAssetLoader").load_m2_with_skin_fdids(
		"res://../data/models/%d.m2" % FDID, PackedInt64Array([125918, 125917, 0]))
	if loaded.has("error") or not loaded.missing_texture_fdids.is_empty():
		fail_fixture("authentic model could not load: %s" % loaded)
		return
	var model: Node3D = loaded.node
	model.name = "M2DebugReferenceModel"
	stage.add_child(model)
	var player: Node = model.get_node("M2Animation")
	player.set_process(false)
	if not player.advance_time_ms(750.0):
		fail_fixture("fixed authored pose did not advance")
		return
	var skeleton := model.get_node("Skeleton3D") as Skeleton3D
	if skeleton.get_bone_parent(FLAGGED_BONE) != 12:
		fail_fixture("authored parent identity changed")
		return
	var camera := stage.get_node("Camera") as Camera3D
	var bounds: AABB = model.get_meta("m2_bounds")
	var focus := bounds.get_center()
	var radius := maxf(bounds.size.length() * 0.5, 1.0)
	camera.look_at_from_position(focus + Vector3(1.4, 0.7, 1.8).normalized() * radius * 3.0, focus)
	var output := OS.get_environment("M2_ANIM_SCREENSHOTS")
	if output.is_empty():
		fail_fixture("M2_ANIM_SCREENSHOTS required")
		return
	DirAccess.make_dir_recursive_absolute(output)
	var adjusted := skeleton.get_bone_global_pose(FLAGGED_BONE)
	skeleton.clear_bones_global_pose_override()
	await process_frame
	await RenderingServer.frame_post_draw
	var before := root.get_texture().get_image()
	if before.get_size() != SIZE or before.save_png(output.path_join("a6-125913-before.png")) != OK:
		fail_fixture("fullHD before capture failed")
		return
	var ordinary := skeleton.get_bone_global_pose(FLAGGED_BONE)
	# Zero elapsed is deliberate: same clock and crossfade pose, restored composition.
	player.advance_time_ms(0.0)
	await process_frame
	await RenderingServer.frame_post_draw
	var after := root.get_texture().get_image()
	if after.get_size() != SIZE or after.save_png(output.path_join("a6-125913-after.png")) != OK:
		fail_fixture("fullHD after capture failed")
		return
	var restored := skeleton.get_bone_global_pose(FLAGGED_BONE)
	if not restored.is_equal_approx(adjusted):
		fail_fixture("zero elapsed restoration changed the adjusted pose")
		return
	print("A6_NATIVE fdid=", FDID, " bone=", FLAGGED_BONE, " ordinary=", ordinary, " adjusted=", adjusted)
	print("PASS A6 authentic low-bit model in m2debug at1920x1080; ordinary baseline/affine override same clock")
	quit(0)
