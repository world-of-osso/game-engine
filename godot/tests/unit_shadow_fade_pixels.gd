extends SceneTree

# The unit shadow radius on real models, sun, terrain and city, offline: three held
# player models stand on sunlit cobbles before, halfway through and past the fade band
# (45..50 yd) of the High unit shadow radius. The radius is measured from a reference
# point 47.5 yd from the middle model on the line through the models (in the game, the
# world camera); the view stays close enough to see each shadow. One frame state is
# captured with an unlimited radius, the 50 yd radius, no radius and the unlimited
# radius again: the near shadow is unchanged, the band one keeps a dithered share and
# the far one is gone.

var out_dir := OS.get_environment("CAPTURE_DIR") if OS.get_environment("CAPTURE_DIR") != "" \
		else "/syncthing/AgentShared/2026-11-07/fps-120-close/"
# The Trade District street of the outdoor benchmark pose, seen from 20 yd up; the
# models stand where these 1280x720 screen points meet the sunlit plaza right of the
# fountain.
const EYE := Vector3(-8862.0, 116.4, -659.0)
const LOOK_AT := Vector3(-8848.16, 96.48, -644.14)
const SCREEN_POINTS := [Vector2(960, 640), Vector2(980, 540), Vector2(1000, 440)]
const PLAZA_HEIGHT := 96.4
const RADIUS := 50.0
const BAND_MIDDLE := 47.5
# Noon (half-minutes): the sun reaches the street between the houses.
const WORLD_MINUTES := 1440.0

var client: Node3D
var camera: Camera3D
var closed := false

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	root.size = Vector2i(1280, 720)
	DirAccess.make_dir_recursive_absolute(out_dir)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await wait_ms(1500)
	client.preview_set_focus(LOOK_AT)
	var error: String = client.preview_world_map("Azeroth", LOOK_AT)
	if error != "":
		return fail("map Azeroth: " + error)
	if not await wait_until(func(): return client.terrain_height_at(LOOK_AT.x, LOOK_AT.z) != null \
			and client.account_state().terrain.pending_count == 0, 240000):
		return fail("terrain never loaded")
	client.set_world_minutes(WORLD_MINUTES)
	camera = Camera3D.new()
	camera.far = 3000.0
	root.add_child(camera)
	camera.current = true
	camera.position = EYE
	camera.look_at(LOOK_AT)
	var feet: Array[Vector3] = []
	var distances: Array[float] = []
	for index in SCREEN_POINTS.size():
		var origin := camera.project_ray_origin(SCREEN_POINTS[index])
		var normal := camera.project_ray_normal(SCREEN_POINTS[index])
		var at := origin + normal * ((PLAZA_HEIGHT - origin.y) / normal.y)
		var height = client.terrain_height_at(at.x, at.z)
		if height == null:
			return fail("no terrain under character %d" % index)
		at.y = height
		feet.append(at)
		distances.append(at.distance_to(EYE))
		error = client.preview_character(1, 1, 0, at, PI * 0.5)
		if error != "":
			return fail("character: " + error)
		client.preview_character_clip(index, 0, 0.0)
		client.preview_pause_character(index, true)
	var along := (feet[0] - feet[2]).normalized()
	var reference: Vector3 = feet[1] + along * BAND_MIDDLE
	for index in feet.size():
		distances[index] = feet[index].distance_to(reference)
	var radius_yards := RADIUS
	print("UNIT_SHADOW_FADE feet=%s distances=%s reference=%s" % [feet, distances, reference])
	if distances[0] >= radius_yards * 0.9 or distances[2] <= radius_yards:
		return fail("models do not straddle the fade band")
	error = client.preview_world_lighting(LOOK_AT, EYE)
	if error != "":
		return fail("lighting: " + error)
	hide_ui(client)
	if not await wait_until(objects_idle, 240000):
		return fail("objects never finished loading")
	await wait_ms(3000)
	client.preview_unit_shadow_radius(reference, INF)
	var unlimited := await capture("unit-shadow-fade-unlimited")
	client.preview_unit_shadow_radius(reference, radius_yards)
	var radius := await capture("unit-shadow-fade-radius")
	# Light travels along the sun's -Z: a body point 1 yd up lands this far along the ground.
	var sun := client.find_child("Sun", true, false) as DirectionalLight3D
	if sun == null:
		return fail("no sun")
	var travel := -sun.global_basis.z
	var shadow_offset := Vector3(travel.x, 0.0, travel.z) / -travel.y
	client.preview_unit_shadow_radius(reference, 0.0)
	var none := await capture("unit-shadow-fade-none")
	client.preview_unit_shadow_radius(reference, INF)
	var restored := await capture("unit-shadow-fade-restored")
	if closed:
		return
	# Each window's shadow is how much cutting every shadow changes it; a model keeps the share
	# of it the radius capture still matches. Animated water and swaying trees change
	# pixels between any two frames: the two unlimited captures measure that.
	var failures: Array[String] = []
	var removed: Array[float] = []
	for index in feet.size():
		var centre: Vector3 = feet[index] + shadow_offset
		var noise := changed_share(unlimited, restored, centre)
		var shadow := changed_share(unlimited, none, centre) - noise
		var cut := changed_share(unlimited, radius, centre) - noise
		if shadow <= 0.002:
			failures.append("model %d casts no visible shadow (%.4f)" % [index, shadow])
			removed.append(0.0)
			continue
		removed.append(cut / shadow)
		print("UNIT_SHADOW_FADE model %d distance=%.2f shadow=%.4f removed=%.3f noise=%.4f" % [
			index, distances[index], shadow, removed[index], noise])
	if failures.is_empty():
		if removed[0] > 0.05:
			failures.append("near shadow lost %.3f" % removed[0])
		if removed[1] < 0.15 or removed[1] > 0.85:
			failures.append("band shadow lost %.3f, expected a dithered share" % removed[1])
		if removed[2] < 0.9:
			failures.append("far shadow lost only %.3f" % removed[2])
	for failure in failures:
		push_error("UNIT_SHADOW_FADE_FAIL " + failure)
	if failures.is_empty():
		print("UNIT_SHADOW_FADE_PASS")
	finish(0 if failures.is_empty() else 1)

# The mean per-pixel colour change in a 3x3 yd ground window around `centre`, a
# model's shadow; changes of 4/255 or less in every channel count as none.
func changed_share(a: Image, b: Image, centre: Vector3) -> float:
	var foot := centre
	var corners := [Vector3(-1.5, 0, -1.5), Vector3(1.5, 0, -1.5), Vector3(-1.5, 0, 1.5), Vector3(1.5, 0, 1.5)]
	var low := Vector2(INF, INF)
	var high := Vector2(-INF, -INF)
	for corner in corners:
		var point := camera.unproject_position(foot + corner)
		low = low.min(point)
		high = high.max(point)
	var x0 := clampi(int(low.x), 0, a.get_width() - 1)
	var x1 := clampi(int(high.x), 0, a.get_width() - 1)
	var y0 := clampi(int(low.y), 0, a.get_height() - 1)
	var y1 := clampi(int(high.y), 0, a.get_height() - 1)
	var changed := 0.0
	var total := 0
	for y in range(y0, y1 + 1):
		for x in range(x0, x1 + 1):
			total += 1
			var p := a.get_pixel(x, y)
			var q := b.get_pixel(x, y)
			var change := maxf(absf(p.r - q.r), maxf(absf(p.g - q.g), absf(p.b - q.b)))
			if change > 4.0 / 255.0:
				changed += change
	return changed / float(maxi(total, 1))

func capture(name: String) -> Image:
	for frame in 4:
		await process_frame
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	if image == null or image.is_empty():
		fail("capture " + name + " produced no image")
		return null
	image.save_png(out_dir.path_join(name + ".png"))
	return image

func hide_ui(node: Node) -> void:
	for child in node.get_children():
		if child is CanvasLayer:
			child.visible = false
		elif child is Control:
			child.hide()
		else:
			hide_ui(child)

func objects_idle() -> bool:
	var debug: Dictionary = JSON.parse_string(client.preview_objects_debug())
	return debug.waiting_assets == 0 and debug.pending == 0 and debug.loading == 0 \
		and debug.waiting == 0 and debug.spawned_wmos + debug.spawned_doodads > 0

func wait_ms(ms: int) -> void:
	var deadline := Time.get_ticks_msec() + ms
	while Time.get_ticks_msec() < deadline:
		await process_frame

func wait_until(condition: Callable, timeout_ms: int) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if condition.call():
			return true
	return false

func fail(error: String) -> void:
	if closed:
		return
	push_error("UNIT_SHADOW_FADE_FAIL " + error)
	finish(1)

func finish(code: int) -> void:
	closed = true
	if is_instance_valid(client):
		client.free()
	quit(code)
