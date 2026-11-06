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
## File-name prefix: "master-" for the before-everything baseline.
var file_prefix := OS.get_environment("CAPTURE_PREFIX")
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

var client: Node
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
	# Ten yards from the anchor, where the calibrated distance law keeps a hit at master's size.
	place_camera(Vector3(ROW.x, floor_y + 3.4, ROW.z + 10.0),
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
		"measure":
			await measure_one()
		_:
			fail("unknown CAPTURE_CASE " + case)
			return
	print("CAPTURE skin=%s case=%s DONE" % [client.account_state().ui_skin, case])
	client.free()
	quit(0)

## One hit alone: the rendered ink box, against the one the fixture can compute without a
## second render, so the no-overlap metric uses a box that matches the pixels.
func measure_one() -> void:
	var before: Image = await grab()
	client.preview_combat_text(HIT, 4000, anchor(Vector3(-7.0, 0.0, 0.0)))
	client.preview_combat_text(CRIT, 4001, anchor(Vector3(7.0, 0.0, 0.0)))
	freeze(0.4)
	var after: Image = await capture("measure", before)
	# One text at a time, with and without on adjacent frames so scene drift cannot widen it.
	for label in text_labels():
		label.visible = false
		var without: Image = await grab()
		label.visible = true
		var with: Image = await grab()
		var center: Vector2 = camera.unproject_position(label.global_position)
		print("MEASURE text=%s ink=%s reserved=%s center=%s font=%d pixel=%.6f" % [
			label.text, diff_box(with, without, 0.2),
			engine_box(center, float(label.font_size), float(label.pixel_size), label.text),
			center, label.font_size, label.pixel_size])
	report_overlap()
	var before_edges := dark_edge(after)
	for label in text_labels():
		label.outline_size = 24
	var thick: Image = await grab()
	var after_edges := dark_edge(thick)
	print("OUTLINE size8=%d size24=%d text_pixels=%d" % [
		before_edges, after_edges, bright_text_pixels(thick)])
	save_jpeg(thick, "measure-outline24")

## Bright text pixels: the ink the outline has to surround.
func bright_text_pixels(image: Image) -> int:
	var count := 0
	for y in image.get_height():
		for x in image.get_width():
			var pixel := image.get_pixel(x, y)
			if maxf(pixel.r, maxf(pixel.g, pixel.b)) > 0.75:
				count += 1
	return count

## Bounding box of the pixels whose colour moved by `threshold` between two frames.
func diff_box(a: Image, b: Image, threshold: float) -> Rect2:
	var low := Vector2i(a.get_width(), a.get_height())
	var high := Vector2i(-1, -1)
	for y in a.get_height():
		for x in a.get_width():
			var p := a.get_pixel(x, y)
			var q := b.get_pixel(x, y)
			if absf(p.r - q.r) + absf(p.g - q.g) + absf(p.b - q.b) > threshold:
				low = Vector2i(mini(low.x, x), mini(low.y, y))
				high = Vector2i(maxi(high.x, x), maxi(high.y, y))
	if high.x < 0:
		return Rect2()
	return Rect2(low, high - low)

## Every live world text, in the order the client added them.
func text_labels() -> Array:
	var labels := []
	for child in client.get_children():
		if child is Label3D and not child.has_meta("combat_text_shadow"):
			labels.append(child)
	return labels

## Bounding box of the pixels that differ between two frames inside a window around `center`,
## so scene noise outside the text cannot widen it.
func ink_box(after: Image, before: Image, center: Vector2, half_width: int = 320) -> Rect2:
	var low := Vector2i(after.get_width(), after.get_height())
	var high := Vector2i(-1, -1)
	var x0 := maxi(0, int(center.x) - half_width)
	var x1 := mini(after.get_width() - 1, int(center.x) + half_width)
	var y0 := maxi(0, int(center.y) - 240)
	var y1 := mini(after.get_height() - 1, int(center.y) + 240)
	for y in range(y0, y1 + 1):
		for x in range(x0, x1 + 1):
			if is_text_pixel(after.get_pixel(x, y), before.get_pixel(x, y)):
				low = Vector2i(mini(low.x, x), mini(low.y, y))
				high = Vector2i(maxi(high.x, x), maxi(high.y, y))
	if high.x < 0:
		return Rect2()
	return Rect2(low, high - low)

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
	freeze(0.4)
	var after: Image = await capture("set", before)
	check_count(9, "nine world texts over three units")
	report_text(after, before)
	report_overlap()

## Headless frames are far slower than real time, so a 1.5 s text would expire between the
## spawn and the capture: stop time and set the age the capture shows.
func freeze(seconds: float) -> void:
	client.preview_combat_text_advance(seconds)
	client.preview_combat_text_hold(true)

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
	check(dropped == 4, "a fifth text evicts the oldest slot, it does not grow the unit")
	freeze(0.4)
	var after: Image = await capture("limit", before)
	report_text(after, before)
	report_overlap()

## Non-crit styles, so no slot is replaced: 13 units hold four each.
const SPREAD_STYLES := [HIT, HEAL, PERIODIC, PET_MELEE]

## Fifty numbers over thirteen units: the perf scene, and the density the slot rule allows.
func capture_spread() -> void:
	var before: Image = await grab()
	spawn_fifty()
	freeze(0.4)
	var live: int = client.spells_state().combat_text
	print("SPREAD live=%d" % live)
	check(live == 50, "fifty simultaneous numbers over thirteen units")
	var after: Image = await capture("spread", before)
	report_text(after, before)
	report_overlap()
	measure_cost()

func spawn_fifty() -> void:
	# Thirteen units spread across the visible ground, as a raid would stand.
	for index in 50:
		var unit := 3000 + index / 4
		var column := float(index / 4)
		var at := anchor(Vector3(-10.8 + 1.8 * column, 0.0, 1.2 * (index % 4) - 1.8))
		client.preview_combat_text(SPREAD_STYLES[index % 4], unit, at)

func measure_perf() -> void:
	spawn_fifty()
	freeze(0.2)
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
## The same screen box the engine reserves for a text (combat_text::SCREEN_PX_PER_EM), so the
## metric measures the placement rule itself.
func engine_box(center: Vector2, font: float, pixel: float, text: String) -> Rect2:
	var em := 1699.0 * font * pixel
	var height := em * 0.7
	var width := em * (0.5 * text.length() + 0.25)
	return Rect2(center - Vector2(width * 0.5, height * 0.5), Vector2(width, height))

## Screen boxes of the live texts, from the node's own font size and pixel size.
func report_overlap() -> void:
	var boxes := []
	for label in text_labels():
		var center: Vector2 = camera.unproject_position(label.global_position)
		boxes.append({
			"text": label.text,
			"rect": engine_box(center, float(label.font_size), float(label.pixel_size), label.text),
		})
	var clashes := 0
	for i in boxes.size():
		for j in range(i + 1, boxes.size()):
			if boxes[i]["rect"].intersects(boxes[j]["rect"]):
				clashes += 1
				if clashes <= 6:
					print("OVERLAP %s %s %s %s" % [
						boxes[i]["text"], boxes[i]["rect"], boxes[j]["text"], boxes[j]["rect"]])
	print("OVERLAP_COUNT %d of %d pairs" % [clashes, boxes.size() * (boxes.size() - 1) / 2])

func report_label_properties() -> void:
	var labels := text_labels()
	if labels.is_empty():
		print("LABEL_PROPS none")
		return
	var label: Label3D = labels[0]
	print("LABEL_PROPS text=%s outline=%d outline_modulate=%s modulate=%s font=%s" % [
		label.text, label.outline_size, label.outline_modulate, label.modulate,
		label.font.resource_path if label.font != null else "<builtin>"])

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
	report_label_properties()
	print("TEXT_PIXELS white=%d green=%d orange=%d yellow=%d dark=%d" % [
		white, green, orange, yellow, dark_edge(after)])
	check(white + green + orange + yellow > 0, "the frame drew coloured combat text")

## Near-black pixels around the text: the outline/shadow that makes it readable on bright
## ground. Retail's world font carries a black shadow (`GxuFontAddShadow`).
func dark_edge(image: Image) -> int:
	var dark := 0
	for y in range(1, image.get_height() - 1):
		for x in range(1, image.get_width() - 1):
			var pixel := image.get_pixel(x, y)
			if maxf(pixel.r, maxf(pixel.g, pixel.b)) > 0.12:
				continue
			var bright := false
			for offset in [Vector2i(1, 0), Vector2i(-1, 0), Vector2i(0, 1), Vector2i(0, -1)]:
				var neighbour := image.get_pixel(x + offset.x, y + offset.y)
				if maxf(neighbour.r, maxf(neighbour.g, neighbour.b)) > 0.75:
					bright = true
					break
			if bright:
				dark += 1
	return dark

func is_text_pixel(pixel: Color, background: Color) -> bool:
	var delta := absf(pixel.r - background.r) + absf(pixel.g - background.g) + absf(pixel.b - background.b)
	return delta > 0.15

func anchor(offset: Vector3) -> Vector3:
	return Vector3(ROW.x, floor_y + 3.4, ROW.z) + offset

func check_count(expected: int, label: String) -> void:
	check(client.spells_state().combat_text == expected, label)

func check(condition: bool, label: String) -> void:
	if condition:
		print("PASS " + label)
	elif file_prefix != "":
		# The baseline capture shows what master does; its contract differs on purpose.
		print("BASELINE " + label)
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
	save_jpeg(before, file_prefix + name + "-notext")
	save_jpeg(after, file_prefix + name)
	var diff := image_diff(before, after, 0.15)
	print("CAPTURE case=%s ui=%s file=%s notext=%s" % [
		name, client.account_state().ui_skin, OUT_DIR.path_join(file_prefix + name + ".jpg"),
		OUT_DIR.path_join(file_prefix + name + "-notext.jpg")])
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
			if absf(a.r - b.r) + absf(a.g - b.g) + absf(a.b - b.b) > threshold:
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
