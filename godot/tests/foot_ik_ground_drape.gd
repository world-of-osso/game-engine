extends SceneTree

# Client-only visual proof: foot IK on real Stormwind cathedral steps and ground spell
# draping of a real Death and Decay kit. Loads authored maps offline (no server), adds a
# real player model / a real spell kit, and captures Forever frames.
#
# CAPTURE_CASE: footik | drape_steps | drape_flat
#
# Headless frames run far faster than real time, so every wait here is a wall-clock wait:
# the object loader, the spell assets and the IK smoothing all advance with time.

const OUT_DIR := "/syncthing/AgentShared/2026-11-05/foot-ik-ground-drape/"
const STORMWIND_STEP := Vector3(-8539.452, 135.47, -738.3701)
# Open Elwynn ground east of Stormwind, for the flat-ground drape.
const ELWYNN_FLAT := Vector3(-7939.0, 0.0, -138.0)
const DECAY_VISUAL := 173378
const DECAY_KIT := 230208
const DECAY_SPELL := 43265

var client: Node3D
var camera: Camera3D
var closed := false

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	var case := OS.get_environment("CAPTURE_CASE")
	if case == "":
		fail("CAPTURE_CASE is required")
		return
	root.size = Vector2i(1280, 720)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await wait_ms(1500)
	var skin_error: String = client.preview_world_skin(true)
	if skin_error != "" or client.account_state().ui_skin != "Forever":
		fail("Forever skin unavailable: " + skin_error)
		return
	if case == "footik":
		await capture_foot_ik()
	elif case == "drape_steps":
		await capture_drape("Azeroth", STORMWIND_STEP, "steps")
	elif case == "drape_flat":
		await capture_drape("Azeroth", ELWYNN_FLAT, "flat")
	else:
		fail("unknown CAPTURE_CASE " + case)

func wait_ms(ms: int) -> void:
	var deadline := Time.get_ticks_msec() + ms
	while Time.get_ticks_msec() < deadline:
		await process_frame

func wait_until(condition: Callable, label: String, timeout_ms: int) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if condition.call():
			return true
	fail("timeout: " + label)
	return false

# Average milliseconds per rendered frame over `frames` frames.
func average_frame_ms(frames: int) -> float:
	var start := Time.get_ticks_usec()
	for frame in frames:
		await process_frame
	return float(Time.get_ticks_usec() - start) / 1000.0 / frames

func load_world(directory: String, point: Vector3) -> bool:
	client.preview_set_focus(point)
	var error: String = client.preview_world_map(directory, point)
	if error != "":
		fail("map " + directory + ": " + error)
		return false
	return await wait_until(
		func():
			var state: Dictionary = client.account_state()
			return client.terrain_height_at(point.x, point.z) != null \
				and state.terrain.pending_count == 0,
		"terrain " + directory, 240000)

func place_camera(point: Vector3, eye: Vector3, focus: Vector3) -> void:
	camera = Camera3D.new()
	camera.far = 3000.0
	root.add_child(camera)
	camera.current = true
	camera.position = eye
	camera.look_at(focus)
	var error: String = client.preview_world_lighting(point, eye)
	if error != "":
		fail("lighting: " + error)

func capture(name: String) -> void:
	hide_ui(client)
	for frame in 3:
		await process_frame
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	if image == null or image.is_empty():
		fail("capture " + name + " produced no image")
		return
	var path := OUT_DIR.path_join(name + ".png")
	var error := image.save_png(path)
	if error != OK:
		fail("cannot save " + path)
		return
	print("CAPTURE skin=%s case=%s file=%s" % [client.account_state().ui_skin, OS.get_environment("CAPTURE_CASE"), path])

# Whether the spawned world's object loader has nothing left to place.
func objects_idle() -> bool:
	var debug: Dictionary = JSON.parse_string(client.preview_objects_debug())
	return debug.waiting_assets == 0 and debug.pending == 0 and debug.loading == 0

func wait_objects() -> bool:
	var started := Time.get_ticks_msec()
	while Time.get_ticks_msec() < started + 180000:
		await process_frame
		if objects_idle():
			return true
	fail("preview objects never finished loading")
	return false

func capture_foot_ik() -> void:
	if not await load_world("Azeroth", STORMWIND_STEP):
		return
	client.set_world_minutes(1440.0)
	client.preview_set_focus(STORMWIND_STEP)
	print("MILESTONE world loaded")
	var ready := await wait_until(
		func():
			var probe: Dictionary = client.preview_probe_surface(
				STORMWIND_STEP.x, STORMWIND_STEP.z, STORMWIND_STEP.y)
			return probe.surface > probe.floor + 0.15,
		"render step spawned", 240000)
	if not ready:
		return
	print("PROBE " + JSON.stringify(client.preview_probe_surface(
		STORMWIND_STEP.x, STORMWIND_STEP.z, STORMWIND_STEP.y)))
	place_camera(STORMWIND_STEP, STORMWIND_STEP + Vector3(2.2, 1.2, 2.2), STORMWIND_STEP + Vector3(0.0, 0.55, 0.0))
	await wait_ms(500)
	await capture("foot-ik-stormwind-steps-empty")
	print("MILESTONE empty captured")
	var error: String = client.preview_character(1, 2, 0, STORMWIND_STEP, 2.4)
	print("MILESTONE character added: " + error)
	if error != "":
		fail("character: " + error)
		return
	client.preview_character_clip(0, 0, 0.0)
	client.preview_pause_character(0, true)
	client.preview_foot_ik_enabled(false)
	await wait_ms(1200)
	print("BEFORE " + JSON.stringify(client.preview_foot_ik_state()))
	await capture("foot-ik-stormwind-stand-before")
	client.preview_foot_ik_enabled(true)
	await wait_ms(1500)
	print("AFTER " + JSON.stringify(client.preview_foot_ik_state()))
	await capture("foot-ik-stormwind-stand-after")
	# Mid-stride walk pose: run the Walk clip and pause on the first frame where one foot is
	# clearly lifted, so the stance foot is planted on the tread and the other swings.
	client.preview_foot_ik_enabled(false)
	client.preview_character_clip(0, 4, 0.0)
	client.preview_pause_character(0, false)
	var lift_seen := 0.0
	var stride_deadline := Time.get_ticks_msec() + 5000
	while Time.get_ticks_msec() < stride_deadline:
		await process_frame
		var entry: Dictionary = client.preview_foot_ik_state().characters[0]
		lift_seen = abs(entry.animated[0] - entry.animated[1])
		if lift_seen > 0.15:
			break
	client.preview_pause_character(0, true)
	print("WALK-STRIDE lift=%.3f" % lift_seen)
	await wait_ms(300)
	print("WALK-BEFORE " + JSON.stringify(client.preview_foot_ik_state()))
	await capture("foot-ik-stormwind-walk-before")
	client.preview_foot_ik_enabled(true)
	await wait_ms(1500)
	print("WALK-AFTER " + JSON.stringify(client.preview_foot_ik_state()))
	await capture("foot-ik-stormwind-walk-after")
	# Cost of the solve on this machine: the same standing pose, IK off then on.
	var off_ms := await average_frame_ms(200)
	client.preview_foot_ik_enabled(false)
	var plain_ms := await average_frame_ms(200)
	client.preview_foot_ik_enabled(true)
	var on_ms := await average_frame_ms(200)
	print("TIMING foot_ik_off=%.2fms foot_ik_on=%.2fms (first=%s)" % [plain_ms, on_ms, off_ms])
	finish()

func capture_drape(directory: String, point: Vector3, label: String) -> void:
	if not await load_world(directory, point):
		return
	if point.y == 0.0:
		point.y = client.terrain_height_at(point.x, point.z)
	client.preview_set_focus(point)
	place_camera(point, point + Vector3(0.0, 3.4, 7.0), point)
	if not await wait_objects():
		return
	# Terrain meshes finish a frame or two after the streaming counters reach zero.
	await wait_ms(1500)
	var probe: Dictionary = client.preview_probe_surface(point.x, point.z, point.y)
	print("PROBE-" + label + " " + JSON.stringify(probe))
	await capture("ground-drape-%s-bare" % label)
	var started_kit := ""
	var deadline := Time.get_ticks_msec() + 120000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		started_kit = client.preview_spell_kit(DECAY_VISUAL, DECAY_KIT, DECAY_SPELL, point)
		if started_kit == "":
			break
	if started_kit != "":
		fail("spell kit: " + started_kit)
		return
	print("DRAPE-KITS started")
	await wait_ms(1200)
	await capture("ground-drape-%s-forever" % label)
	if label == "steps":
		# Straddle the stair edge: half the effect stays on the tread, half on the floor.
		point += Vector3(1.8, 0.0, 0.0)
		camera.position = point + Vector3(0.0, 3.0, 6.0)
		camera.look_at(point)
		var edge: String = client.preview_spell_kit(DECAY_VISUAL, DECAY_KIT, DECAY_SPELL, point)
		if edge != "":
			fail("step edge kit: " + edge)
			return
		await wait_ms(1200)
		await capture("ground-drape-steps-edge-forever")
	finish()

func hide_ui(node: Node) -> void:
	for child in node.get_children():
		if child is CanvasLayer:
			child.visible = false
		elif child is Control:
			child.hide()
		else:
			hide_ui(child)

func finish() -> void:
	if closed:
		return
	closed = true
	if is_instance_valid(client):
		client.free()
	print("PASS: " + OS.get_environment("CAPTURE_CASE"))
	quit(0)

func fail(error: String) -> void:
	if closed:
		return
	closed = true
	push_error(error)
	if is_instance_valid(client):
		client.free()
	quit(1)
