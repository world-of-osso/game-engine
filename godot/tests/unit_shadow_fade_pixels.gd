extends SceneTree

# The unit shadow radius on real models, sun and terrain, offline: three held player
# models stand 30, 47.5 and 60 yd from the camera. One frame state is captured with an
# unlimited radius and again with the High radius (50 yd, fade band 45..50): the near
# shadow is unchanged, the band one keeps a dithered share, the far one is gone and no
# model's own pixels change.

var out_dir := OS.get_environment("CAPTURE_DIR") if OS.get_environment("CAPTURE_DIR") != "" \
		else "/syncthing/AgentShared/2026-11-07/fps-120-close/"
const ORIGIN := Vector3(-8590.0, 96.9, -770.0)
const DIRECTION := Vector3(1.0, 0.0, 0.0)
const DISTANCES := [30.0, 47.5, 60.0]
const RADIUS := 50.0
# Afternoon sun, for shadows longer than the models are wide.
const WORLD_MINUTES := 960.0

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
	client.preview_set_focus(ORIGIN)
	var error: String = client.preview_world_map("Azeroth", ORIGIN)
	if error != "":
		return fail("map Azeroth: " + error)
	if not await wait_until(func(): return client.terrain_height_at(ORIGIN.x, ORIGIN.z) != null \
			and client.account_state().terrain.pending_count == 0, 240000):
		return fail("terrain never loaded")
	client.set_world_minutes(WORLD_MINUTES)
	var feet: Array[Vector3] = []
	for index in DISTANCES.size():
		var at: Vector3 = ORIGIN + DIRECTION * DISTANCES[index]
		var height = client.terrain_height_at(at.x, at.z)
		if height == null:
			return fail("no terrain under character %d" % index)
		at.y = height
		feet.append(at)
		error = client.preview_character(1, 1, 0, at, PI * 0.5)
		if error != "":
			return fail("character: " + error)
		client.preview_character_clip(index, 0, 0.0)
		client.preview_pause_character(index, true)
	var eye: Vector3 = ORIGIN + Vector3(0.0, 14.0, 0.0) - DIRECTION * 4.0
	camera = Camera3D.new()
	camera.far = 3000.0
	root.add_child(camera)
	camera.current = true
	camera.position = eye
	camera.look_at(ORIGIN + DIRECTION * 45.0)
	error = client.preview_world_lighting(ORIGIN + DIRECTION * 45.0, eye)
	if error != "":
		return fail("lighting: " + error)
	hide_ui(client)
	if not await wait_until(objects_idle, 240000):
		return fail("objects never finished loading")
	await wait_ms(3000)
	client.preview_unit_shadow_radius(eye, INF)
	var unlimited := await capture("unit-shadow-fade-unlimited")
	client.preview_unit_shadow_radius(eye, RADIUS)
	var radius := await capture("unit-shadow-fade-radius50")
	client.preview_unit_shadow_radius(eye, INF)
	var restored := await capture("unit-shadow-fade-restored")
	if closed:
		return
	var failures: Array[String] = []
	var changes: Array[float] = []
	for index in feet.size():
		changes.append(changed_share(unlimited, radius, feet[index]))
	print("UNIT_SHADOW_FADE changed near=%.4f band=%.4f far=%.4f restored=%.4f" % [
		changes[0], changes[1], changes[2], changed_share(unlimited, restored, feet[1])])
	if changes[0] > 0.002:
		failures.append("near shadow changed (%.4f)" % changes[0])
	if changes[1] <= 0.002 or changes[1] >= changes[2]:
		failures.append("band shadow change %.4f not between near and far %.4f" % [changes[1], changes[2]])
	if changes[2] <= 0.01:
		failures.append("far shadow unchanged (%.4f)" % changes[2])
	if changed_share(unlimited, restored, feet[1]) > 0.002:
		failures.append("unlimited radius did not restore the band shadow")
	for failure in failures:
		push_error("UNIT_SHADOW_FADE_FAIL " + failure)
	if failures.is_empty():
		print("UNIT_SHADOW_FADE_PASS")
	finish(0 if failures.is_empty() else 1)

# The share of pixels in a 9x5 yd ground window around `foot` that differ by more
# than 4/255 in any channel.
func changed_share(a: Image, b: Image, foot: Vector3) -> float:
	var corners := [Vector3(-4.5, 0, -2.5), Vector3(4.5, 0, -2.5), Vector3(-4.5, 0, 2.5), Vector3(4.5, 0, 2.5)]
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
	var changed := 0
	var total := 0
	for y in range(y0, y1 + 1):
		for x in range(x0, x1 + 1):
			total += 1
			var p := a.get_pixel(x, y)
			var q := b.get_pixel(x, y)
			if maxf(absf(p.r - q.r), maxf(absf(p.g - q.g), absf(p.b - q.b))) > 4.0 / 255.0:
				changed += 1
	return float(changed) / float(maxi(total, 1))

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
