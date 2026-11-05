extends SceneTree

# v3 proofs: foot IK on a real Stormwind staircase (collision ramp under render treads)
# and Death and Decay draping on flat ground, a raised surface edge, steps, and behind a
# wall. Offline: no server, real authored map, real player model, real spell kit.
#
# CAPTURE_CASE: footik | drape_flat | drape_rug | drape_steps | drape_wall
#
# Headless frames run far faster than real time, so every wait is a wall-clock wait.

const OUT_DIR := "/syncthing/AgentShared/2026-11-05/foot-ik-ground-drape/v3/"
# Stormwind staircase: render treads from 107.95 down to 106.20 between x -8566 and -8562.
const STAIR_BOTTOM := Vector3(-8564.0, 107.4, -778.0)
const STAIR_TOP := Vector3(-8566.0, 0.0, -778.0)
# Open flat ground beside the city wall (collision floor equals the drawn ground).
const FLAT_GROUND := Vector3(-8590.0, 96.9, -770.0)
# Stormwind rug lying on a floor, at the stairs' foot.
const RUG := Vector3(-8550.4, 108.76, -720.9)
# The foot of the cliff under the ledge: seen from above the ledge, the wall hides the
# far half of the effect.
const WALL_FOOT := Vector3(-8559.0, 99.6, -770.0)

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
	elif case == "drape_flat":
		await capture_drape("flat", FLAT_GROUND)
	elif case == "drape_rug":
		await capture_drape("rug", RUG)
	elif case == "drape_steps":
		await capture_drape("steps", STAIR_BOTTOM)
	elif case == "drape_wall":
		await capture_drape("wall", WALL_FOOT)
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

func load_world(point: Vector3) -> bool:
	client.preview_set_focus(point)
	var error: String = client.preview_world_map("Azeroth", point)
	if error != "":
		fail("map Azeroth: " + error)
		return false
	return await wait_until(
		func():
			var state: Dictionary = client.account_state()
			return client.terrain_height_at(point.x, point.z) != null \
				and state.terrain.pending_count == 0,
		"terrain", 240000)

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

func ground_at(point: Vector3) -> Dictionary:
	return client.preview_probe_surface(point.x, point.z, point.y if point.y != 0.0 else 120.0)

func place_camera(eye: Vector3, focus: Vector3) -> void:
	if camera == null:
		camera = Camera3D.new()
		camera.far = 3000.0
		root.add_child(camera)
		camera.current = true
	camera.position = eye
	camera.look_at(focus)
	var error: String = client.preview_world_lighting(focus, eye)
	if error != "":
		fail("lighting: " + error)

func capture(name: String, with_ui: bool) -> void:
	# Offline, the client shows its out-of-world screen, which covers the whole viewport;
	# keep captures scene-only and rely on the asserted ui_skin for the Forever proof.
	if not with_ui:
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
	print("CAPTURE skin=%s case=%s ui=%s file=%s" % [client.account_state().ui_skin,
		OS.get_environment("CAPTURE_CASE"), with_ui, path])

func spawn_decay(point: Vector3) -> bool:
	var started := Time.get_ticks_msec()
	var error := ""
	while Time.get_ticks_msec() < started + 120000:
		await process_frame
		error = client.preview_spell_kit(DECAY_VISUAL, DECAY_KIT, DECAY_SPELL, point)
		if error == "":
			return true
	fail("spell kit: " + error)
	return false

# Side-on, waist-height, 4.5 yd from the legs, framing the feet and two treads.
func stair_camera(feet: Vector3) -> void:
	place_camera(feet + Vector3(0.0, 1.0, 4.5), feet + Vector3(0.0, 0.45, 0.0))

func capture_foot_ik() -> void:
	if not await load_world(STAIR_BOTTOM):
		return
	client.set_world_minutes(1440.0)
	client.preview_set_focus(STAIR_BOTTOM)
	if not await wait_objects():
		return
	await wait_ms(1200)
	var spot: Dictionary = client.preview_probe_surface(STAIR_BOTTOM.x, STAIR_BOTTOM.z, 120.0)
	if spot.floor == null or spot.surface == null or is_nan(spot.surface):
		fail("no stair surface at the capture point")
		return
	var tread: float = spot.surface
	var floor_y: float = spot.floor
	print("STAIR floor=%.2f surface=%.2f" % [floor_y, tread])
	# Face up the stairs (-x); the camera looks along +z, side-on to the stride.
	var error: String = client.preview_character(1, 2, 0, Vector3(STAIR_BOTTOM.x, floor_y, STAIR_BOTTOM.z), PI)
	if error != "":
		fail("character: " + error)
		return
	client.preview_character_clip(0, 0, 0.0)
	client.preview_pause_character(0, true)
	client.preview_foot_ik_enabled(false)
	await wait_ms(1500)
	# Calibrate: sink the authored feet a third of a yard under the tread so the
	# before frame shows the feet inside the step and the after frame on top of it.
	var stance: Dictionary = client.preview_foot_ik_state().characters[0]
	var feet: float = (stance.animated[0] + stance.animated[1]) * 0.5
	var node_y: float = floor_y + (tread - 0.35) - feet
	client.preview_move_character(0, Vector3(STAIR_BOTTOM.x, node_y, STAIR_BOTTOM.z), PI)
	await wait_ms(1200)
	place_camera(Vector3(STAIR_BOTTOM.x, node_y + 0.75, STAIR_BOTTOM.z + 2.8),
		Vector3(STAIR_BOTTOM.x, node_y + 0.35, STAIR_BOTTOM.z))
	await wait_ms(400)
	print("PLACED node=%.2f tread=%.2f" % [node_y, tread])
	# The same framing without the character, to prove where the legs and treads are.
	client.preview_remove_characters()
	await wait_ms(400)
	await capture("foot-ik-stairs-empty", false)
	error = client.preview_character(1, 2, 0, Vector3(STAIR_BOTTOM.x, floor_y, STAIR_BOTTOM.z), PI)
	if error != "":
		fail("character: " + error)
		return
	client.preview_character_clip(0, 0, 0.0)
	client.preview_pause_character(0, true)
	client.preview_foot_ik_enabled(false)
	client.preview_move_character(0, Vector3(STAIR_BOTTOM.x, node_y, STAIR_BOTTOM.z), PI)
	await wait_ms(1200)
	var feet_point := Vector3(STAIR_BOTTOM.x, node_y, STAIR_BOTTOM.z)
	print("FRAME feet=%s head=%s" % [camera.unproject_position(feet_point),
		camera.unproject_position(feet_point + Vector3(0.0, 1.9, 0.0))])
	print("STAND-BEFORE " + JSON.stringify(client.preview_foot_ik_state()))
	await capture("foot-ik-stairs-stand-before", false)
	client.preview_foot_ik_enabled(true)
	await wait_ms(1500)
	print("STAND-AFTER " + JSON.stringify(client.preview_foot_ik_state()))
	await capture("foot-ik-stairs-stand-after", false)
	# Walk up: three stride frames, each one tread higher, at successive positions.
	client.preview_foot_ik_enabled(true)
	client.preview_character_clip(0, 4, 0.0)
	client.preview_pause_character(0, false)
	for step in 3:
		var up := float(step)
		var at := Vector3(STAIR_BOTTOM.x - up, node_y + 0.43 * up, STAIR_BOTTOM.z)
		client.preview_move_character(0, at, PI)
		place_camera(Vector3(at.x, at.y + 0.75, at.z + 2.8), Vector3(at.x, at.y + 0.35, at.z))
		var lift := 0.0
		var deadline := Time.get_ticks_msec() + 5000
		while Time.get_ticks_msec() < deadline:
			await process_frame
			var entry: Dictionary = client.preview_foot_ik_state().characters[0]
			lift = abs(entry.animated[0] - entry.animated[1])
			if lift > 0.12:
				break
		client.preview_pause_character(0, true)
		print("WALK-%d lift=%.3f at=%s" % [step, lift, JSON.stringify(at)])
		await wait_ms(500)
		await capture("foot-ik-stairs-walk-%d" % step, false)
		client.preview_pause_character(0, false)
	finish()

# The collision floor and visible surface under a column, searched from a few heights
# above the terrain: an authored floor can sit anywhere above it.
func ground_probe(x: float, z: float, near: float) -> Dictionary:
	var terrain: float = client.terrain_height_at(x, z)
	var origins := [near, terrain + 10.0, terrain + 3.0, terrain + 1.5, terrain + 20.0, terrain + 30.0]
	for origin in origins:
		if origin == 0.0:
			continue
		var probe: Dictionary = client.preview_probe_surface(x, z, origin)
		if probe.floor == null or is_nan(probe.floor):
			continue
		if probe.surface == null or is_nan(probe.surface):
			continue
		return probe
	return {}

func capture_drape(label: String, point: Vector3) -> void:
	if not await load_world(point):
		return
	client.preview_set_focus(point)
	if not await wait_objects():
		return
	await wait_ms(1500)
	var probe: Dictionary = ground_probe(point.x, point.z, point.y)
	if probe.is_empty():
		fail("no collision floor and render surface at the capture point")
		return
	# Anchor the effect on the ground it is meant to lie on.
	point.y = probe.floor
	print("PROBE-%s %s" % [label, JSON.stringify(probe)])
	# 45 degrees above, close enough that the 17.7 yd disc fills the frame.
	if label == "wall":
		place_camera(Vector3(point.x - 7.5, point.y + 9.4, point.z + 1.0), point)
	else:
		place_camera(point + Vector3(3.6, 3.6, 4.8), point)
	if label != "wall":
		print("FRAME-%s center=%s left=%s right=%s radius8=%s" % [label,
			camera.unproject_position(point),
			camera.unproject_position(point + Vector3(-3.0, 0.0, 0.0)),
			camera.unproject_position(point + Vector3(3.0, 0.0, 0.0)),
			camera.unproject_position(point + Vector3(8.0, 0.0, 0.0))])
	# Before: draping off, the authored kit placement (a yard above its anchor). Both
	# captures freeze the kit at the same animation time, so only draping differs.
	client.preview_set_drape(false)
	if not await spawn_decay(point):
		return
	await wait_ms(1800)
	pause_kits(true)
	await wait_ms(300)
	print("KIT-%s before %s" % [label, JSON.stringify(client.spell_visuals_state())])
	await capture("ground-drape-%s-before" % label, false)
	client.preview_clear_kits()
	await wait_ms(400)
	# After: draping on, same kit at the same point.
	client.preview_set_drape(true)
	if not await spawn_decay(point):
		return
	await wait_ms(1800)
	pause_kits(true)
	await wait_ms(300)
	await capture("ground-drape-%s-after" % label, false)
	finish()

# Freezes every kit animation at the same time in both captures.
func pause_kits(paused: bool) -> void:
	for child in client.get_children():
		if not str(child.name).begins_with("PreviewKit"):
			continue
		for player in child.find_children("*", "WowAnimationPlayer", true, false):
			player.call("set_paused", paused)

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
