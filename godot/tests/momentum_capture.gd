extends SceneTree

# Native Run -> Stand capture. Run with --fixed-fps 144 so GPU readback and
# buffered PNG writes cannot skip simulation-time samples. No animation clock
# or momentum state is mocked: WowAnimationPlayer processes the moving skeleton.
const FPS := 144.0
const RUN_SPEED := 7.0
const RUN_FRAMES := 52
const STOP_X := -1.7
const SHOULDER := 27
const STOP_FRAMES := [0, 10, 43, 72]

var shots := ""
var model: Node3D
var skeleton: Skeleton3D
var animation: Node
var images: Array[Image] = []
var names: Array[String] = []
var rest: Quaternion
var failed := false

func _initialize() -> void:
	call_deferred("run_capture")

func run_capture() -> void:
	root.size = Vector2i(1920, 1080)
	shots = OS.get_environment("GODOT_MOMENTUM_SCREENSHOTS")
	if shots.is_empty() or not DirAccess.dir_exists_absolute(shots):
		fail("Requires an existing GODOT_MOMENTUM_SCREENSHOTS directory")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await frames(72)
	var scene := client.find_child("DebugCharacter", true, false)
	if scene == null:
		fail("--screen debugcharacter did not open the debug character scene")
		return
	# The scene owns an orbit controller. Disable only that controller, not
	# its children, before taking ownership of the camera for this fixture.
	scene.set_process(false)
	model = scene.get_node("DebugCharacterGeoset") as Node3D
	skeleton = model.find_children("*", "Skeleton3D", true, false)[0] as Skeleton3D
	animation = model.find_children("*", "WowAnimationPlayer", true, false)[0]
	model.rotation = Vector3.ZERO
	model.position = Vector3(STOP_X, 0, 0)
	var camera := scene.get_node("Camera") as Camera3D
	camera.look_at_from_position(Vector3(-2.2, 1.3, 3.6), Vector3(-2.2, 1.0, 0))
	await frames(216)
	rest = skeleton.get_bone_pose_rotation(SHOULDER)
	await capture("00-rest.png")
	await run_then_stop()
	if failed:
		return
	for index in images.size():
		var path := shots.path_join(names[index])
		var saved := images[index].save_png(path)
		if saved != OK:
			fail("Save %s: %s" % [path, error_string(saved)])
			return
	print("PASS: momentum Run -> Stand capture (fixed simulation FPS, not wall time)")
	quit(0)

func run_then_stop() -> void:
	var start_x := STOP_X - RUN_SPEED * RUN_FRAMES / FPS
	model.position = Vector3(start_x, 0, 0)
	await frames(216)
	if not animation.play_animation(5, true):
		fail("Model has no Run clip")
		return
	for frame in range(1, RUN_FRAMES + 1):
		await process_frame
		model.position = Vector3(start_x + RUN_SPEED * frame / FPS, 0, 0)
		await RenderingServer.frame_post_draw
	if animation.current_animation_id() != 5:
		fail("Moving model is not playing Run")
		return
	buffer_frame("01-running.png")
	if not animation.play_animation(0, true):
		fail("Model has no Stand clip")
		return
	# First stationary processed frame is stop 0: the velocity tracker detects
	# the stop and integrates one 6.94 ms spring step before drawing it.
	for frame in range(STOP_FRAMES[-1] + 1):
		await process_frame
		await RenderingServer.frame_post_draw
		if STOP_FRAMES.has(frame):
			buffer_frame("%02d-stop-%03dms.png" % [names.size(), roundi(frame * 1000.0 / FPS)])
	if animation.current_animation_id() != 0:
		fail("Stopped model is not playing Stand")

func frames(count: int) -> void:
	for frame in count:
		await process_frame
		var delta := root.get_process_delta_time()
		if absf(delta - 1.0 / FPS) > 0.00001:
			fail("Run capture with --fixed-fps 144; delta=%f" % delta)
			return

func capture(name: String) -> void:
	await RenderingServer.frame_post_draw
	buffer_frame(name)

func buffer_frame(name: String) -> void:
	var turned := rad_to_deg(rest.angle_to(skeleton.get_bone_pose_rotation(SHOULDER)))
	print("MOMENTUM %s clip=%d shoulder=%.2f deg x=%.6f" % [name, animation.current_animation_id(), turned, model.position.x])
	images.append(root.get_texture().get_image())
	names.append(name)

func fail(message: String) -> void:
	failed = true
	push_error(message)
	quit(1)
