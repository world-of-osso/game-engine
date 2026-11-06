extends SceneTree

## Floating combat text proofs, offline in Forever: authored Stormwind ground, a real player
## model and the production world-text path (docs/specs/floating-combat-text.md).
##
## CAPTURE_CASE: set | limit | spread | perf
##
## `preview_combat_text(index, unit, at)` floats one scripted world text of the production
## mapping over `at`; `index % 9` is hit, crit, heal, heal crit, Dodge, Absorb, pet melee,
## pet spell, periodic tick. `preview_combat_text_cost(iterations)` reports the microseconds
## one frame spends advancing every live text.
##
## Captures are 960x540 JPEGs, each paired with the same frame before any text so the diff is
## the text alone. Run against the release library.

const OUT_DIR := "/syncthing/AgentShared/2026-11-06/combat-text/"
## Open ground inside the Stormwind wall (the healer-ring capture's flat row).
const ROW := Vector3(-8591.0, 96.9, -770.0)
const HIT := 0
const CRIT := 1
const HEAL := 2
const HEAL_CRIT := 3
const DODGE := 4
const ABSORB := 5
const PET_MELEE := 6
const PET_SPELL := 7
const PERIODIC := 8
## Longest frame an already-running text may take on the release library.
const FRAME_BUDGET_US := 100.0

var client: Node3D
var camera: Camera3D
var spawned := 0
var floor_y := 0.0

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
	if not await load_world(ROW):
		return
	floor_y = floor_at(ROW)
	client.set_world_minutes(1440.0)
	spawn_character(Vector3(ROW.x, floor_y, ROW.z))
	client.preview_set_focus(ROW)
	place_camera(Vector3(ROW.x, floor_y + 3.0, ROW.z + 8.0),
		Vector3(ROW.x, floor_y + 1.8, ROW.z))
	await settle_scene()
	match case:
		"set":
			await capture_set()
		"limit":
			await capture_limit()
		"spread":
			await capture_spread()
		"perf":
			await measure_perf()
		_:
			fail("unknown CAPTURE_CASE " + case)
			return
	print("CAPTURE skin=%s case=%s DONE" % [client.account_state().ui_skin, case])
	client.free()
	quit(0)

## Nine texts over three pseudo-units (three each, under the four-slot limit), so every
## style the mapping produces appears in one frame.
func capture_set() -> void:
	var before: Image = await grab()
	var text_indices := [HIT, CRIT, HEAL, HEAL_CRIT, DODGE, ABSORB, PET_MELEE, PET_SPELL, PERIODIC]
	for index in text_indices.size():
		var slot := index / 3
		var at := anchor(Vector3(-1.4 + 1.4 * slot, 0.0, 0.0))
		var live: int = client.preview_combat_text(text_indices[index], 1000 + slot, at)
		print("SPAWN index=%d unit=%d live=%d" % [text_indices[index], 1000 + slot, live])
	var after: Image = await capture("set", before)
	check_count(9, "nine world texts over three units")
	report_text(after, before)

## One unit holds four; a new crit replaces that unit's crit; a fifth plain hit is dropped.
func capture_limit() -> void:
	var before: Image = await grab()
	var live := 0
	for index in [HIT, PET_MELEE, HEAL]:
		live = client.preview_combat_text(index, 2000, anchor(Vector3.ZERO))
	print("LIMIT after three plain hits live=%d" % live)
	check(live == 3, "three texts take three of the four slots")
	var crit: int = client.preview_combat_text(CRIT, 2000, anchor(Vector3.ZERO))
	print("LIMIT after a crit live=%d" % crit)
	check(crit == 4, "a crit takes the last free slot")
	var replaced: int = client.preview_combat_text(CRIT, 2000, anchor(Vector3.ZERO))
	print("LIMIT after a second crit live=%d" % replaced)
	check(replaced == 4, "a second crit replaces the first, it does not add a slot")
	var dropped: int = client.preview_combat_text(PERIODIC, 2000, anchor(Vector3.ZERO))
	print("LIMIT after a fifth plain text live=%d" % dropped)
	check(dropped == 4, "a plain text with no free slot is dropped")
	var after: Image = await capture("limit", before)
	report_text(after, before)

## Non-crit styles, so no slot is replaced: 13 units hold four each.
const SPREAD_STYLES := [HIT, HEAL, PERIODIC, PET_MELEE]

## Fifty numbers over thirteen units: the perf scene, and the density the slot rule allows.
func capture_spread() -> void:
	var before: Image = await grab()
	spawn_fifty()
	var live: int = client.spells_state().combat_text
	print("SPREAD live=%d" % live)
	check(live == 50, "fifty simultaneous numbers over thirteen units")
	var after: Image = await capture("spread", before)
	report_text(after, before)
	measure_cost()

func spawn_fifty() -> void:
	for index in 50:
		var unit := 3000 + index / 4
		var column := float(index / 4)
		var at := anchor(Vector3(-3.6 + 0.6 * column, 0.0, 0.9 * (index % 4)))
		client.preview_combat_text(SPREAD_STYLES[index % 4], unit, at)

func measure_perf() -> void:
	spawn_fifty()
	check(client.spells_state().combat_text == 50, "fifty live numbers to measure")
	measure_cost()

func measure_cost() -> void:
	var samples := []
	for run in 3:
		samples.append(client.preview_combat_text_cost(2000))
	samples.sort()
	var median: float = samples[1]
	print("COST_US samples=%s median=%.3f budget=%.1f" % [samples, median, FRAME_BUDGET_US])
	check(median <= FRAME_BUDGET_US, "fifty numbers stay inside the per-frame budget")
	check(median > 0.0, "the cost probe measured something")

## Text alone: how many pixels the numbers cover, and how many of those are each style's
## colour, as the numeric description of a capture the reviewer cannot see.
func report_text(after: Image, before: Image) -> void:
	var diff := image_diff(before, after, 0.15)
	print("DIFF count=%d fraction=%.5f box=%s..%s" % [
		diff["count"], diff["fraction"], diff["min"], diff["max"]])
	var white := 0
	var green := 0
	var orange := 0
	var yellow := 0
	for y in after.get_height():
		for x in after.get_width():
			var pixel := after.get_pixel(x, y)
			if not is_text_pixel(pixel, before.get_pixel(x, y)):
				continue
			if pixel.g > 0.75 and pixel.r < 0.5 and pixel.b < 0.5:
				green += 1
			elif pixel.r > 0.75 and pixel.b < 0.4 and pixel.g > 0.3 and pixel.g < 0.65:
				orange += 1
			elif pixel.r > 0.75 and pixel.g > 0.75 and pixel.b < 0.4:
				yellow += 1
			elif pixel.r > 0.75 and pixel.g > 0.75 and pixel.b > 0.75:
				white += 1
	print("TEXT_PIXELS white=%d green=%d orange=%d yellow=%d" % [white, green, orange, yellow])
	check(white + green + orange + yellow > 0, "the frame drew coloured combat text")

func is_text_pixel(pixel: Color, background: Color) -> bool:
	var delta := abs(pixel.r - background.r) + abs(pixel.g - background.g) + abs(pixel.b - background.b)
	return delta > 0.15

func anchor(offset: Vector3) -> Vector3:
	return Vector3(ROW.x, floor_y + 3.4, ROW.z) + offset

func check_count(expected: int, label: String) -> void:
	check(client.spells_state().combat_text == expected, label)

func check(condition: bool, label: String) -> void:
	if condition:
		print("PASS " + label)
	else:
		fail(label)

func fail(message: String) -> void:
	print("FAIL " + message)
	client.free()
	quit(1)

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
	while Time.get_ticks_msec() < started + 240000:
		await process_frame
		if objects_idle():
			return true
	fail("preview objects never finished loading")
	return false

func objects_idle() -> bool:
	var debug: Dictionary = JSON.parse_string(client.preview_objects_debug())
	return debug.waiting_assets == 0 and debug.pending == 0 and debug.loading == 0 \
		and debug.waiting == 0

func settle_scene() -> void:
	var idle_streak := 0
	var deadline := Time.get_ticks_msec() + 180000
	while Time.get_ticks_msec() < deadline and idle_streak < 2:
		await wait_ms(500)
		idle_streak = idle_streak + 1 if objects_idle() else 0
	await wait_ms(1000)

func floor_at(point: Vector3) -> float:
	var probe: Dictionary = client.preview_probe_surface(point.x, point.z, point.y if point.y != 0.0 else 120.0)
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

func spawn_character(position: Vector3, yaw: float = PI) -> void:
	var error: String = client.preview_character(1, 5, 0, position, yaw)
	if error != "":
		fail("character: " + error)
	client.preview_character_clip(spawned, 0, 0.0)
	client.preview_pause_character(spawned, true)
	spawned += 1

## The frame with the text, after the same frame without it.
func capture(name: String, before: Image) -> Image:
	var after := await grab()
	save_jpeg(before, name + "-notext")
	save_jpeg(after, name)
	var diff := image_diff(before, after, 0.15)
	print("CAPTURE case=%s ui=%s file=%s notext=%s" % [
		name, client.account_state().ui_skin, OUT_DIR.path_join(name + ".jpg"),
		OUT_DIR.path_join(name + "-notext.jpg")])
	print("CAPTURE_DIFF case=%s count=%d fraction=%.5f" % [name, diff["count"], diff["fraction"]])
	return after

func grab() -> Image:
	hide_ui(client)
	for frame in 3:
		await process_frame
	await RenderingServer.frame_post_draw
	return root.get_texture().get_image()

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
	var pixels := before.get_width() * before.get_height()
	return {
		"count": count,
		"fraction": float(count) / float(pixels),
		"min": [low.x, low.y],
		"max": [high.x, high.y],
	}
