extends SceneTree

var client: Node

func _initialize() -> void:
	call_deferred("run_test")

func fail(message: String) -> void:
	push_error(message)
	quit(1)

func select_animation(player: Node, animation_id: int) -> bool:
	# Select by authored ID using the existing sequence API, not a test-only host method.
	for index in range(1024):
		if not player.play_sequence(index, true):
			break
		if player.current_animation_id() == animation_id:
			return true
	fail("Authored animation missing: " + str(animation_id))
	return false

func changed_from_rest(skeleton: Skeleton3D) -> int:
	var changed := 0
	for bone in skeleton.get_bone_count():
		var pose := skeleton.get_bone_pose(bone)
		var rest := skeleton.get_bone_rest(bone)
		if pose.origin.distance_to(rest.origin) > 0.001 or pose.basis.get_rotation_quaternion().angle_to(rest.basis.get_rotation_quaternion()) > 0.01:
			changed += 1
	return changed

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var loaded: Dictionary = client.load_model_scene("res://../data/models/1011653.m2")
	if loaded.has("error"):
		fail("Load HumanMale HD external-animation fixture: " + str(loaded.error))
		return
	var scene := client.get_node("ModelScene")
	var player := scene.find_child("M2Animation", true, false)
	var skeleton := scene.find_child("Skeleton3D", true, false) as Skeleton3D
	if player == null or skeleton == null:
		fail("External-animation fixture has no animation player/skeleton")
		return
	player.set_paused(true)
	for animation_id in [97, 100]:
		if not select_animation(player, animation_id):
			return
		player.set_paused(false)
		if not player.advance_time_ms(5000.0):
			fail("Could not advance external animation " + str(animation_id))
			return
		player.set_paused(true)
		var changed := changed_from_rest(skeleton)
		if changed <= 10:
			fail("External animation %d has only %d posed bones" % [animation_id, changed])
			return
		for frame in range(4):
			await process_frame
		await RenderingServer.frame_post_draw
		var path := ProjectSettings.globalize_path("res://../data/diagnostics/godot-conversion/external-animation-%d.png" % animation_id)
		var error := root.get_texture().get_image().save_png(path)
		if error != OK:
			fail("Capture external animation: " + error_string(error))
			return
		print("PASS: authored external animation ", animation_id, " poses ", changed, " bones; screenshot=", path)
	client.queue_free()
	await process_frame
	quit(0)
