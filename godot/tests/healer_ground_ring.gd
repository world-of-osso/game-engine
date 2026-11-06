extends SceneTree

# Healer ground ring proofs (PLAN USER 11-05). Offline: a real authored map, real player
# models and the production ring renderer, with a fixture-supplied party. Forever only.
#
# CAPTURE_CASE: party | chevron | stairs | fps
#
# Headless frames run far faster than real time, so every wait is a wall-clock wait.
# Captures are saved as 960x540 JPEGs for review and diffed against a no-ring frame.

const OUT_DIR := "/syncthing/AgentShared/2026-11-05/healer-ground-ring/"
# Open ground inside the Stormwind wall (the foot IK capture's flat point).
const FLAT := Vector3(-8590.0, 96.9, -770.0)
# A clean terrain row south of the city pavement, seen from above the wall.
const ROW := Vector3(-8591.0, 96.9, -770.0)
# Stormwind staircase: render treads from 107.95 down to 106.20 between x -8566 and -8562.
const STAIRS := Vector3(-8564.0, 107.4, -778.0)
const HOLY_PRIEST := 257
const HURT_ENTITY := 5002

var client: Node3D
var camera: Camera3D
var known := PackedInt64Array([527, 585, 2061, 139])
var spawned := 0
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
	if case == "party":
		await capture_party(FLAT)
	elif case == "chevron":
		await capture_chevron(FLAT)
	elif case == "stairs":
		await capture_stairs()
	elif case == "fps":
		await capture_fps(FLAT)
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

# Waits until the object queue is idle twice in a row, so the first capture is not the
# frame the surrounding doodads finish spawning on.
func settle_scene() -> void:
	var idle_streak := 0
	var deadline := Time.get_ticks_msec() + 180000
	while Time.get_ticks_msec() < deadline and idle_streak < 2:
		await wait_ms(500)
		idle_streak = idle_streak + 1 if objects_idle() else 0
	await wait_ms(1000)

func ground_at(point: Vector3) -> Dictionary:
	return client.preview_probe_surface(point.x, point.z, point.y if point.y != 0.0 else 120.0)

func floor_at(point: Vector3) -> float:
	var probe := ground_at(point)
	if probe.is_empty() or probe.floor == null or is_nan(probe.floor):
		fail("no collision floor at %s" % point)
		return point.y
	return probe.floor

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
	if not with_ui:
		hide_ui(client)
	for frame in 3:
		await process_frame
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	if image == null or image.is_empty():
		fail("capture " + name + " produced no image")
		return null
	save_jpeg(image, name)
	print("CAPTURE skin=%s case=%s ui=%s file=%s" % [client.account_state().ui_skin,
		OS.get_environment("CAPTURE_CASE"), with_ui, OUT_DIR.path_join(name + ".jpg")])
	return image

func save_jpeg(image: Image, name: String) -> void:
	var scaled: Image = image.duplicate()
	scaled.resize(960, 540, Image.INTERPOLATE_LANCZOS)
	var error: Error = scaled.save_jpg(OUT_DIR.path_join(name + ".jpg"), 0.9)
	if error != OK:
		fail("cannot save " + name + ".jpg")

func hide_ui(node: Node) -> void:
	for child in node.get_children():
		if child is CanvasLayer:
			child.visible = false
		elif child is Control:
			child.hide()
		else:
			hide_ui(child)

# Hide the out-of-world screen but keep the ring chevron layer, which lives under it.
func show_chevron_layer() -> void:
	hide_ui(client)
	var layer := client.get_node_or_null("HealerGroundRingChevrons")
	if layer == null:
		fail("chevron layer was not created")
		return
	layer.visible = true

# Pixels that differ between two captures, with their bounding box.
func image_diff(before: Image, after: Image, threshold: float, y_min: int = 0) -> Dictionary:
	var count := 0
	var low := Vector2i(before.get_width(), before.get_height())
	var high := Vector2i(-1, -1)
	for y in range(y_min, before.get_height()):
		for x in before.get_width():
			var a := before.get_pixel(x, y)
			var b := after.get_pixel(x, y)
			if abs(a.r - b.r) + abs(a.g - b.g) + abs(a.b - b.b) > threshold:
				count += 1
				low = Vector2i(mini(low.x, x), mini(low.y, y))
				high = Vector2i(maxi(high.x, x), maxi(high.y, y))
	var pixels := before.get_width() * (before.get_height() - y_min)
	return {
		"count": count,
		"fraction": float(count) / float(pixels),
		"min": [low.x, low.y],
		"max": [high.x, high.y],
	}

# The strongest colour change in a 5x5 window at a world point, between two captures.
func ring_pixel_delta(on: Image, off: Image, point: Vector3) -> float:
	var screen := camera.unproject_position(point)
	var best := 0.0
	for dy in range(-2, 3):
		for dx in range(-2, 3):
			var x := int(screen.x) + dx
			var y := int(screen.y) + dy
			if x < 0 or y < 0 or x >= on.get_width() or y >= on.get_height():
				continue
			var a := on.get_pixel(x, y)
			var b := off.get_pixel(x, y)
			best = maxf(best, abs(a.r - b.r) + abs(a.g - b.g) + abs(a.b - b.b))
	return best

func install_party(members: Array, spec: int = HOLY_PRIEST) -> void:
	var error: String = client.preview_healer_ring_party(spec, known, members)
	if error != "":
		fail("healer ring party: " + error)
		return
	await wait_until(
		func(): return client.preview_healer_ring_state().catalog == "ready",
		"dispel catalog", 120000)
	await wait_ms(1200)

func spawn_character(position: Vector3, yaw: float = PI) -> void:
	var error: String = client.preview_character(1, 5, 0, position, yaw)
	if error != "":
		fail("character: " + error)
	client.preview_character_clip(spawned, 0, 0.0)
	client.preview_pause_character(spawned, true)
	spawned += 1

func member(name: String, health: float, position: Vector3, debuff: int = 0,
		entity: int = 0) -> Dictionary:
	return {
		"name": name,
		"health": health,
		"position": position,
		"debuff": debuff,
		"entity": entity,
	}

func state_member(state: Dictionary, name: String) -> Dictionary:
	for entry in state["members"]:
		if entry["name"] == name:
			return entry
	fail("no ring state for " + name)
	return {}

# Mixed health, one removable magic debuff and one poison the spec cannot remove.
func party_members(floor_y: float) -> Array:
	return [
		member("Healthy", 1.0, Vector3(-8596.0, floor_y, -770.0)),
		member("Hurt", 0.62, Vector3(-8593.5, floor_y, -770.0), 0, HURT_ENTITY),
		member("Low", 0.28, Vector3(-8591.0, floor_y, -770.0)),
		member("MagicDebuff", 0.55, Vector3(-8588.5, floor_y, -770.0), 1),
		member("PoisonDebuff", 0.45, Vector3(-8586.0, floor_y, -770.0), 4),
	]

func capture_party(point: Vector3) -> void:
	if not await load_world(point):
		return
	client.set_world_minutes(1440.0)
	var floor_y := floor_at(ROW)
	var members := party_members(floor_y)
	for index in members.size():
		spawn_character(Vector3(members[index].position.x, floor_y, members[index].position.z))
	await install_party(members)
	client.preview_set_focus(ROW)
	place_camera(Vector3(-8591.0, floor_y + 6.5, -758.5),
		Vector3(-8591.0, floor_y, -769.0))
	await settle_scene()
	var state: Dictionary = client.preview_healer_ring_state()
	print("RING_STATE " + JSON.stringify(state))
	var hurt := state_member(state, "Hurt")
	var low := state_member(state, "Low")
	var magic := state_member(state, "MagicDebuff")
	var poison := state_member(state, "PoisonDebuff")
	var healthy := state_member(state, "Healthy")
	check(state["catalog"] == "ready" and state["healer"], "Holy Priest is a healer spec")
	check(state["scope"] == "All", "default scope shows a healer's party")
	check(abs(hurt["alpha"] - 1.0) < 0.001 and hurt["ring_visible"], "Hurt ring visible")
	check(abs(hurt["arc_start_deg"] - 136.8) < 1.0, "Hurt arc starts at 62% of the ring")
	check(abs(low["arc_start_deg"] - 259.2) < 1.0, "Low arc starts at 28% of the ring")
	check(not healthy["ring_visible"], "Healthy ring stays hidden at 100%")
	check(magic["dispel"] == "Magic" and magic["dispel_visible"], "magic dispel ring shown")
	check(poison["dispel"] == "" and not poison["dispel_visible"],
		"poison is not dispellable by this spec")
	check(hurt["mesh_vertices"] > 10, "arc mesh built")
	var on: Image = await capture("party-rings", false)
	client.preview_healer_ring_enabled(false)
	await wait_ms(60)
	var off: Image = await capture("party-no-rings", false)
	await wait_ms(700)
	var off_later: Image = await capture("party-no-rings-later", false)
	client.preview_healer_ring_enabled(true)
	if on == null or off == null or off_later == null:
		return
	var diff := image_diff(off, on, 0.06)
	print("DIFF party-rings-vs-no-rings " + JSON.stringify(diff))
	var drift := image_diff(off, off_later, 0.06)
	print("DIFF party-no-rings-drift " + JSON.stringify(drift))
	var ground := image_diff(off, on, 0.06, 150)
	print("DIFF party-ground-region " + JSON.stringify(ground))
	check(diff["count"] > 2000, "rings change the frame")
	check(ground["count"] > 4 * drift["count"] + 1000,
		"ring pixels dominate the frame drift")
	check(ground["max"][1] < 650, "rings sit at ground level")
	# Arc length: a pixel inside the drawn arc changed, one in the clockwise gap did not.
	var hurt_anchor: Vector3 = hurt["anchor"]
	var hurt_in := ring_point(hurt_anchor, 200.0)
	var hurt_gap := ring_point(hurt_anchor, 60.0)
	var low_anchor: Vector3 = low["anchor"]
	var low_in := ring_point(low_anchor, 300.0)
	var low_gap := ring_point(low_anchor, 100.0)
	var deltas := {
		"hurt_in": ring_pixel_delta(on, off, hurt_in),
		"hurt_gap": ring_pixel_delta(on, off, hurt_gap),
		"low_in": ring_pixel_delta(on, off, low_in),
		"low_gap": ring_pixel_delta(on, off, low_gap),
	}
	print("RING_PIXELS " + JSON.stringify(deltas))
	check(deltas["hurt_in"] > 0.10, "Hurt arc pixel drawn at 200 degrees")
	check(deltas["hurt_gap"] < 0.06, "Hurt gap stays empty at 60 degrees")
	check(deltas["low_in"] > 0.10, "Low arc pixel drawn at 300 degrees")
	check(deltas["low_gap"] < 0.06, "Low gap stays empty at 100 degrees")
	# Click-to-target: the ring hit test returns this member's entity inside the arc and
	# nothing in the gap.
	await wait_ms(600)
	var picked: int = client.preview_healer_ring_pick_at(camera.unproject_position(hurt_in))
	var gap_pick: int = client.preview_healer_ring_pick_at(camera.unproject_position(hurt_gap))
	print("RING_PICK inside=%d gap=%d" % [picked, gap_pick])
	check(picked == HURT_ENTITY, "clicking the ring targets its member")
	check(gap_pick == -1, "clicking the empty gap targets nothing")
	finish()

func ring_point(anchor: Vector3, degrees: float) -> Vector3:
	return Vector3(anchor.x + sin(deg_to_rad(degrees)) * 0.97, anchor.y + 0.02,
		anchor.z - cos(deg_to_rad(degrees)) * 0.97)

func capture_chevron(point: Vector3) -> void:
	if not await load_world(point):
		return
	client.set_world_minutes(1440.0)
	var floor_y := floor_at(ROW)
	var members := [
		member("Visible", 0.5, Vector3(-8591.0, floor_y, -770.0)),
		member("Behind", 0.4, Vector3(-8591.0, floor_y, -745.0)),
	]
	spawn_character(Vector3(members[1].position.x, floor_y, members[1].position.z))
	await install_party(members)
	place_camera(Vector3(-8591.0, floor_y + 5.0, -760.0),
		Vector3(-8591.0, floor_y + 0.5, -769.0))
	await settle_scene()
	show_chevron_layer()
	await wait_ms(300)
	var state: Dictionary = client.preview_healer_ring_state()
	print("CHEVRON_STATE " + JSON.stringify(state))
	var behind := state_member(state, "Behind")
	var visible := state_member(state, "Visible")
	check(behind.has("chevron"), "off-screen ally gets a chevron")
	check(not visible.has("chevron"), "on-screen ally gets no chevron")
	if behind.has("chevron"):
		var chevron: Dictionary = behind["chevron"]
		check(chevron["x"] >= 0 and chevron["x"] <= 1280 and chevron["y"] >= 0 \
			and chevron["y"] <= 720, "chevron sits inside the viewport")
	var on: Image = await capture("chevron-rings", true)
	client.preview_healer_ring_enabled(false)
	await wait_ms(400)
	var off: Image = await capture("chevron-no-rings", true)
	client.preview_healer_ring_enabled(true)
	if on != null and off != null:
		print("DIFF chevron-rings-vs-no-rings " + JSON.stringify(image_diff(off, on, 0.06)))
	finish()

func capture_stairs() -> void:
	if not await load_world(STAIRS):
		return
	client.set_world_minutes(1440.0)
	client.preview_set_focus(STAIRS)
	await wait_ms(2000)
	var bottom := floor_at(STAIRS)
	var members := [
		member("TreadLow", 0.4, Vector3(-8563.2, bottom, -778.0)),
		member("TreadHigh", 0.7, Vector3(-8565.2, bottom, -778.0)),
	]
	for index in members.size():
		spawn_character(Vector3(members[index].position.x, bottom, members[index].position.z))
	await install_party(members)
	place_camera(Vector3(-8564.2, bottom + 4.0, -773.6),
		Vector3(-8564.2, bottom + 0.4, -778.0))
	await settle_scene()
	var state: Dictionary = client.preview_healer_ring_state()
	print("STAIR_STATE " + JSON.stringify(state))
	var low := state_member(state, "TreadLow")
	var high := state_member(state, "TreadHigh")
	check(low["ring_visible"] and high["ring_visible"], "both stair rings visible")
	var span := maxf(
		abs(low["mesh_max_y"] - low["mesh_min_y"]),
		abs(high["mesh_max_y"] - high["mesh_min_y"]))
	print("STAIR_SPAN %.3f" % span)
	check(span > 0.10, "ring follows the treads instead of a flat plane")
	var on: Image = await capture("stairs-rings", false)
	client.preview_healer_ring_enabled(false)
	await wait_ms(400)
	var off: Image = await capture("stairs-no-rings", false)
	client.preview_healer_ring_enabled(true)
	if on != null and off != null:
		print("DIFF stairs-rings-vs-no-rings " + JSON.stringify(image_diff(off, on, 0.06)))
	finish()

# Six paired fixed-camera windows with the rings suspended and resumed in one client:
# wall-clock and engine process milliseconds per frame, plus the feature's own frame cost.
func capture_fps(point: Vector3) -> void:
	if not await load_world(point):
		return
	client.set_world_minutes(1440.0)
	var floor_y := floor_at(ROW)
	var members := party_members(floor_y)
	await install_party(members)
	place_camera(Vector3(-8591.0, floor_y + 6.5, -758.5),
		Vector3(-8591.0, floor_y, -769.0))
	var off := []
	var on := []
	for round in 6:
		client.preview_healer_ring_suspend(true)
		await wait_ms(500)
		off.append(await sample_frames(120))
		client.preview_healer_ring_suspend(false)
		await wait_ms(500)
		on.append(await sample_frames(120))
	print("FPS off=%s" % JSON.stringify(off))
	print("FPS on=%s" % JSON.stringify(on))
	var off_ms := 0.0
	var on_ms := 0.0
	for sample in off:
		off_ms += sample[1]
	for sample in on:
		on_ms += sample[1]
	var off_sorted := off.duplicate()
	var on_sorted := on.duplicate()
	off_sorted.sort_custom(func(a, b): return a[1] < b[1])
	on_sorted.sort_custom(func(a, b): return a[1] < b[1])
	var state: Dictionary = client.preview_healer_ring_state()
	print("FPS_MEAN off_ms=%.4f on_ms=%.4f delta_ms=%.4f" % [
		off_ms / off.size(), on_ms / on.size(), (on_ms - off_ms) / on.size()])
	print("FPS_MEDIAN off_ms=%.4f on_ms=%.4f delta_ms=%.4f" % [
		off_sorted[off_sorted.size() / 2][1], on_sorted[on_sorted.size() / 2][1],
		on_sorted[on_sorted.size() / 2][1] - off_sorted[off_sorted.size() / 2][1]])
	print("FPS_RING_COST update_us_mean=%.4f frames=%d drape_samples=%d" % [
		state["update_us_mean"], state["update_frames"], state["drape_samples"]])
	check(state["update_us_mean"] < 500.0, "ring update stays under 0.5 ms per frame")
	finish()

func sample_frames(count: int) -> Array:
	var started := Time.get_ticks_usec()
	var process := 0.0
	for index in count:
		await process_frame
		process += Performance.get_monitor(Performance.TIME_PROCESS) * 1000.0
	var wall_ms := float(Time.get_ticks_usec() - started) / float(count) / 1000.0
	return [wall_ms, process / float(count)]

func check(condition: bool, label: String) -> void:
	if not condition:
		fail("FAILED: " + label)
	else:
		print("PASS " + label)

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
