extends SceneTree

# Momentum sway capture on `--screen debugcharacter` (docs/specs/momentum-sway.md): the
# left character, turned to face +X in profile, runs 2.5 yd at 7 yd/s and stops dead;
# frames before the run and after the stop go to GODOT_MOMENTUM_SCREENSHOTS, with the
# left shoulder's (bone 27, HumanMale HD) local rotation change from rest.
# Run: godot --path godot -s res://tests/momentum_capture.gd -- --screen debugcharacter
const RUN_SPEED := 7.0
const STOP_X := -1.7
const START_X := -4.2
const SHOULDER := 27
const AFTER_STOP_MS := [0, 33, 66, 100, 150, 250, 400, 700]

var shots := ""
var model: Node3D
var skeleton: Skeleton3D

func _initialize() -> void:
	call_deferred("run_capture")

func run_capture() -> void:
	root.size = Vector2i(1280, 720)
	shots = OS.get_environment("GODOT_MOMENTUM_SCREENSHOTS")
	if shots.is_empty() or not DirAccess.dir_exists_absolute(shots):
		fail("Momentum capture requires an existing GODOT_MOMENTUM_SCREENSHOTS directory")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await settle(500)
	var scene := client.find_child("DebugCharacter", true, false)
	if scene == null:
		fail("--screen debugcharacter did not open the debug character scene")
		return
	model = scene.get_node("DebugCharacterGeoset") as Node3D
	skeleton = model.find_children("*", "Skeleton3D", true, false)[0] as Skeleton3D
	model.rotation = Vector3.ZERO
	model.position = Vector3(STOP_X, 0, 0)
	var camera := scene.get_node("Camera") as Camera3D
	camera.look_at_from_position(Vector3(-2.2, 1.3, 3.6), Vector3(-2.2, 1.0, 0))
	await settle(1500)
	var rest := skeleton.get_bone_pose_rotation(SHOULDER)
	await save_shot("00-rest.png", rest)
	model.position = Vector3(START_X, 0, 0)
	await settle(1500)
	var start := Time.get_ticks_usec()
	while model.position.x < STOP_X:
		await process_frame
		var x := START_X + RUN_SPEED * (Time.get_ticks_usec() - start) / 1e6
		model.position = Vector3(minf(x, STOP_X), 0, 0)
	var stopped := Time.get_ticks_msec()
	for index in AFTER_STOP_MS.size():
		while Time.get_ticks_msec() - stopped < AFTER_STOP_MS[index]:
			await process_frame
		await save_shot("%02d-stop-%03dms.png" % [index + 1, Time.get_ticks_msec() - stopped], rest)
	print("PASS: momentum capture")
	quit(0)

func save_shot(name: String, rest: Quaternion) -> void:
	await RenderingServer.frame_post_draw
	var turned := rad_to_deg(rest.angle_to(skeleton.get_bone_pose_rotation(SHOULDER)))
	print("MOMENTUM %s shoulder %.2f deg x=%.2f" % [name, turned, model.position.x])
	var path := shots.path_join(name)
	var saved := root.get_texture().get_image().save_png(path)
	if saved != OK:
		fail("Save %s: %s" % [path, error_string(saved)])

func settle(ms: int) -> void:
	var until := Time.get_ticks_msec() + ms
	while Time.get_ticks_msec() < until:
		await process_frame

func fail(message: String) -> void:
	push_error(message)
	quit(1)
