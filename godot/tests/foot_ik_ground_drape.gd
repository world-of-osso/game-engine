extends SceneTree

# v3 proofs: foot IK on a real Stormwind staircase (collision ramp under render treads)
# and Death and Decay draping on flat ground, a raised surface edge, steps, and behind a
# wall. Offline: no server, real authored map, real player model, real spell kit.
#
# CAPTURE_CASE: footik | drape_flat | drape_rug | drape_steps | drape_wall
#
# Headless frames run far faster than real time, so every wait is a wall-clock wait.

const OUT_DIR := "/syncthing/AgentShared/2026-11-05/foot-ik-ground-drape/trigger/"
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
	DirAccess.make_dir_recursive_absolute(OUT_DIR)
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
	elif case == "drape_trigger":
		await capture_trigger(DECAY_SPELL, "decay", [[0, "0.5"], [2500, "3"], [5500, "6"], [8500, "9"]])
	elif case == "drape_consecration":
		await capture_trigger(26573, "consecration", [[1000, "1"]])
	elif case == "drape_flamestrike":
		await capture_trigger(2120, "flamestrike", [[1000, "1"]])
	elif case == "drape_life":
		await capture_life()
	elif case == "footik_fps":
		await capture_fps()
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
	if not await wait_until(
		func():
			var state: Dictionary = client.account_state()
			return client.terrain_height_at(point.x, point.z) != null \
				and state.terrain.pending_count == 0,
		"terrain", 240000):
		return false
	# The tile's objects are queued after the terrain is parsed; an empty queue right
	# after the map loads is not a loaded world.
	var started := Time.get_ticks_msec()
	var next_report := started
	while Time.get_ticks_msec() < started + 240000:
		await process_frame
		if objects_idle() and spawned_objects() > 0:
			return true
		if Time.get_ticks_msec() >= next_report:
			next_report += 5000
			print("WORLD %s terrain_pending=%s" % [
				client.preview_objects_debug(),
				client.account_state().terrain.pending_count])
	fail("preview objects never finished loading")
	return false

func spawned_objects() -> int:
	var debug: Dictionary = JSON.parse_string(client.preview_objects_debug())
	return debug.spawned_wmos + debug.spawned_doodads

func objects_idle() -> bool:
	var debug: Dictionary = JSON.parse_string(client.preview_objects_debug())
	return debug.waiting_assets == 0 and debug.pending == 0 and debug.loading == 0 \
		and debug.waiting == 0

func wait_objects() -> bool:
	var started := Time.get_ticks_msec()
	while Time.get_ticks_msec() < started + 180000:
		await process_frame
		if objects_idle() and spawned_objects() > 0:
			return true
	fail("preview objects never finished loading")
	return false

# Waits for a collision floor and a visible render surface at least `lift` yards above
# it (the WMO treads and the rug spawn after the terrain does).
func wait_surface(x: float, z: float, near: float, lift: float) -> Dictionary:
	var deadline := Time.get_ticks_msec() + 180000
	var probe := {}
	var next_report := Time.get_ticks_msec()
	while Time.get_ticks_msec() < deadline:
		probe = ground_probe(x, z, near)
		if not probe.is_empty() and probe.surface >= probe.floor + lift:
			return probe
		if Time.get_ticks_msec() >= next_report:
			next_report += 5000
			print("SURFACE raw=%s probe=%s objects=%s" % [
				JSON.stringify(client.preview_probe_surface(x, z, near)),
				JSON.stringify(probe),
				client.preview_objects_debug()])
		await wait_ms(500)
	return {}

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

func capture(name: String, with_ui: bool) -> Image:
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
		return null
	var path := OUT_DIR.path_join(name + ".png")
	var error := image.save_png(path)
	if error != OK:
		fail("cannot save " + path)
		return null
	print("CAPTURE skin=%s case=%s ui=%s file=%s" % [client.account_state().ui_skin,
		OS.get_environment("CAPTURE_CASE"), with_ui, path])
	return image

# Pixels that differ between two captures, with their bounding box.
func image_diff(before: Image, after: Image, threshold: float) -> Dictionary:
	var count := 0
	var low := Vector2i(before.get_width(), before.get_height())
	var high := Vector2i(-1, -1)
	for y in before.get_height():
		for x in before.get_width():
			var a := before.get_pixel(x, y)
			var b := after.get_pixel(x, y)
			if abs(a.r - b.r) + abs(a.g - b.g) + abs(a.b - b.b) > threshold:
				count += 1
				low = Vector2i(mini(low.x, x), mini(low.y, y))
				high = Vector2i(maxi(high.x, x), maxi(high.y, y))
	return {"count": count, "min": low, "max": high}

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

# Foot-to-tread gap (solved foot minus its target, 0 = on the tread) and knee angles.
func print_stance(tag: String) -> void:
	var entry: Dictionary = client.preview_foot_ik_state().characters[0]
	print("%s root_offset=%.3f gaps=%.3f,%.3f knees_deg=%.1f,%.1f animated_deg=%.1f,%.1f" % [tag,
		entry.root_offset,
		entry.solved[0] - entry.targets[0],
		entry.solved[1] - entry.targets[1],
		rad_to_deg(entry.knee_bend[0]),
		rad_to_deg(entry.knee_bend[1]),
		rad_to_deg(entry.knee_rest[0]),
		rad_to_deg(entry.knee_rest[1])])

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
	var spot: Dictionary = await wait_surface(STAIR_BOTTOM.x, STAIR_BOTTOM.z, 120.0, 0.2)
	if spot.is_empty():
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
	# Low and side-on to the run, just above tread height and far enough back to keep the
	# whole body in frame, so the tread edge and the riser stay visible.
	place_camera(Vector3(STAIR_BOTTOM.x, node_y + 0.40, STAIR_BOTTOM.z + 3.4),
		Vector3(STAIR_BOTTOM.x, node_y + 0.30, STAIR_BOTTOM.z))
	await wait_ms(400)
	print("PLACED node=%.2f tread=%.2f" % [node_y, tread])
	# The same framing without the character, to prove where the legs and treads are.
	client.preview_remove_characters()
	await wait_ms(400)
	await capture("foot-ik-stairs-empty", false)
	if closed:
		return
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
	print_stance("STAND-BEFORE")
	print("STAND-BEFORE " + JSON.stringify(client.preview_foot_ik_state()))
	var before_image: Image = await capture("foot-ik-stairs-stand-before", false)
	if closed:
		return
	client.preview_foot_ik_enabled(true)
	await wait_ms(1500)
	print_stance("STAND-AFTER")
	print("STAND-AFTER " + JSON.stringify(client.preview_foot_ik_state()))
	var after_image: Image = await capture("foot-ik-stairs-stand-after", false)
	if before_image != null and after_image != null:
		print("DIFF stand-before-vs-after %s" % JSON.stringify(
			image_diff(before_image, after_image, 0.09)))
	if closed:
		return
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
		print_stance("WALK-%d" % step)
		await wait_ms(500)
		await capture("foot-ik-stairs-walk-%d" % step, false)
		if closed:
			return
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

# Ten paired fixed-pose windows with foot IK off and on in one client: wall-clock and
# engine process milliseconds per frame, the cost the solve adds to a still frame.
func capture_fps() -> void:
	if not await load_world(STAIR_BOTTOM):
		return
	client.preview_set_focus(STAIR_BOTTOM)
	if not await wait_objects():
		return
	var spot: Dictionary = await wait_surface(STAIR_BOTTOM.x, STAIR_BOTTOM.z, 120.0, 0.2)
	if spot.is_empty():
		fail("no stair surface at the capture point")
		return
	var error: String = client.preview_character(1, 2, 0,
		Vector3(STAIR_BOTTOM.x, spot.floor, STAIR_BOTTOM.z), PI)
	if error != "":
		fail("character: " + error)
		return
	client.preview_character_clip(0, 0, 0.0)
	client.preview_pause_character(0, true)
	await wait_ms(1200)
	place_camera(Vector3(STAIR_BOTTOM.x, spot.floor + 0.75, STAIR_BOTTOM.z + 2.8),
		Vector3(STAIR_BOTTOM.x, spot.floor + 0.35, STAIR_BOTTOM.z))
	var off := []
	var on := []
	for round in 10:
		client.preview_foot_ik_enabled(false)
		await wait_ms(500)
		off.append(await sample_frames(120))
		client.preview_foot_ik_enabled(true)
		await wait_ms(500)
		on.append(await sample_frames(120))
	print("FPS off=%s" % JSON.stringify(off))
	print("FPS on=%s" % JSON.stringify(on))
	print("FPS state %s" % JSON.stringify(client.preview_foot_ik_state()))
	finish()

func sample_frames(count: int) -> Array:
	var started := Time.get_ticks_usec()
	var process := 0.0
	for index in count:
		await process_frame
		process += Performance.get_monitor(Performance.TIME_PROCESS) * 1000.0
	var wall_ms := float(Time.get_ticks_usec() - started) / float(count) / 1000.0
	return [wall_ms, process / float(count)]

# The meshes the previewed kit has spawned under its anchors.
func kit_meshes() -> int:
	var count := 0
	for child in client.get_children():
		if str(child.name).begins_with("PreviewKit"):
			count += child.find_children("*", "MeshInstance3D", true, false).size()
	return count

# Spawns the kit and freezes it shortly after its meshes appear, so both captures show
# the same animation time and the effect has not expired yet.
func spawn_and_pause(point: Vector3) -> bool:
	if not await spawn_decay(point):
		return false
	var deadline := Time.get_ticks_msec() + 20000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if kit_meshes() > 0:
			await wait_ms(1200)
			pause_kits(true)
			await wait_ms(200)
			print("KIT-%s meshes=%d" % [OS.get_environment("CAPTURE_CASE"), kit_meshes()])
			return true
	fail("kit meshes never appeared")
	return false

func capture_drape(label: String, point: Vector3) -> void:
	if not await load_world(point):
		return
	client.preview_set_focus(point)
	if not await wait_objects():
		return
	await wait_ms(1500)
	var lift := 0.0
	if label == "steps":
		lift = 0.2
	elif label == "rug":
		lift = 0.01
	var probe: Dictionary = await wait_surface(point.x, point.z, point.y, lift)
	if probe.is_empty():
		fail("no collision floor and render surface at the capture point")
		return
	# Anchor the effect on the ground it is meant to lie on.
	point.y = probe.floor
	print("PROBE-%s %s" % [label, JSON.stringify(probe)])
	if label == "steps":
		# The ramp-to-tread gap along the flight, and across it, decides the rise window.
		for offset in range(-8, 9):
			var at := point + Vector3(0.5 * float(offset), 0.0, 0.0)
			var column: Dictionary = client.preview_probe_surface(at.x, at.z, point.y + 0.5)
			print("STEPCOLUMN x=%.1f floor=%.2f surface=%.2f" % [at.x,
				column.floor, column.surface])
	# 45 degrees above, close enough that the 17.7 yd disc fills the frame.
	if label == "wall":
		place_camera(Vector3(point.x - 7.5, point.y + 9.4, point.z + 1.0), point)
	else:
		place_camera(point + Vector3(3.6, 3.6, 4.8), point)
	# The kit's real scale: attach 0.75 x effect name 3.0 over a 7.85 yd quad.
	var half := 7.85 * 0.75 * 3.0 * 0.5
	print("DISC-%s center=%s corners=%s" % [label,
		camera.unproject_position(point),
		[camera.unproject_position(point + Vector3(-half, 0.0, -half)),
		camera.unproject_position(point + Vector3(half, 0.0, -half)),
		camera.unproject_position(point + Vector3(half, 0.0, half)),
		camera.unproject_position(point + Vector3(-half, 0.0, half))]])
	# No effect at all, to measure how much of the disc's footprint the effect covers.
	await capture("ground-drape-%s-none" % label, false)
	if closed:
		return
	# Before: draping off, the authored kit placement (a yard above its anchor). Both
	# captures freeze the kit at the same animation time, so only draping differs.
	client.preview_set_drape(false)
	if not await spawn_and_pause(point):
		return
	print("KIT-%s before %s" % [label, JSON.stringify(client.spell_visuals_state())])
	await capture("ground-drape-%s-before" % label, false)
	if closed:
		return
	client.preview_clear_kits()
	await wait_ms(400)
	# After: draping on, same kit at the same point.
	client.preview_set_drape(true)
	if not await spawn_and_pause(point):
		return
	print("KIT-%s after %s" % [label, JSON.stringify(client.spell_visuals_state())])
	await capture("ground-drape-%s-after" % label, false)
	if closed:
		return
	finish()

# Freezes every kit animation at the same time in both captures.
# The effect over its lifetime: which tracks we evaluate, sampled at 0, 0.5, 2 and 5 s.
func capture_life() -> void:
	var point := FLAT_GROUND
	if not await load_world(point):
		return
	client.preview_set_focus(point)
	if not await wait_objects():
		return
	await wait_ms(1500)
	var probe: Dictionary = await wait_surface(point.x, point.z, point.y, 0.0)
	if probe.is_empty():
		fail("no ground at the capture point")
		return
	point.y = probe.floor
	place_camera(point + Vector3(3.6, 3.6, 4.8), point)
	client.preview_set_drape(true)
	await capture("ground-drape-life-none", false)
	if not await spawn_decay(point):
		return
	var started := Time.get_ticks_msec()
	for at in [0, 500, 1500, 2000, 2500, 3000, 4000, 5000]:
		var wait: int = int(at) - int(Time.get_ticks_msec() - started)
		if wait > 0:
			await wait_ms(wait)
		print("LIFE t=%d materials=%s" % [at, JSON.stringify(client.preview_kit_materials())])
		await capture("ground-drape-life-%d" % at, false)
		if closed:
			return
	finish()

# Area-trigger ground visuals: replay the trigger's kits like a server update does and
# capture the effect while it is up.
func capture_trigger(spell: int, prefix: String, shots: Array) -> void:
	var point := FLAT_GROUND
	if not await load_world(point):
		return
	client.preview_set_focus(point)
	if not await wait_objects():
		return
	await wait_ms(1200)
	place_camera(point + Vector3(3.6, 3.6, 4.8), point)
	await capture("area-trigger-%s-none" % prefix, false)
	# Control: the same kit through the preview path that was visible in v5, to separate a
	# replay bug from a fixture/camera problem.
	if prefix == "decay":
		# `preview_spell_kit` answers "preparing" until the kit's assets are in; retry like
		# the v5 drape cases did, then wait for the models to spawn.
		var started_kit := Time.get_ticks_msec()
		while Time.get_ticks_msec() < started_kit + 60000:
			await process_frame
			if client.preview_spell_kit(DECAY_VISUAL, DECAY_KIT, DECAY_SPELL, point) == "":
				break
		var spawned := Time.get_ticks_msec()
		while kit_meshes() == 0 and Time.get_ticks_msec() < spawned + 60000:
			await process_frame
		print("area trigger control: meshes=%d" % kit_meshes())
		await wait_ms(1200)
		pause_kits(true)
		await capture("area-trigger-%s-preview-kit" % prefix, false)
		client.preview_clear_kits()
		await wait_ms(300)
		await capture("area-trigger-%s-none2" % prefix, false)
	var started := Time.get_ticks_msec()
	var pulse := 0
	for shot in shots:
		var at: int = shot[0]
		var wait: int = at - int(Time.get_ticks_msec() - started)
		if wait > 0:
			await wait_ms(wait)
		client.preview_area_trigger(spell, point, 8.0, 10.0, pulse)
		pulse += 1
		# The one-shot fades in over its first 600 ms; capture at its steady frame.
		await wait_ms(800)
		await capture("area-trigger-%s-%s" % [prefix, shot[1]], false)
		if closed:
			return
	finish()

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
