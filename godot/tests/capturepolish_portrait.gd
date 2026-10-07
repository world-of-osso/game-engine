extends SceneTree

# Offline player portrait readiness, same human male/class as buffcancel capture.
# GODOT_CAPTURE_PATH names an output PNG; artifacts use its parent directory.
var fixture: Node
var output := OS.get_environment("GODOT_CAPTURE_PATH").get_base_dir()
var skin := ""

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	for forever in [false, true]:
		skin = "forever" if forever else "modern"
		if not await check_portrait(forever):
			fixture.free()
			quit(1)
			return
		fixture.free()
		await process_frame
	quit(0)

func check_portrait(forever: bool) -> bool:
	fixture = ClassDB.instantiate("PlayerPortraitFixture")
	root.add_child(fixture)
	if not check_error(fixture.initialize(forever)) or not check_error(fixture.tick()):
		return false
	await settle(3)
	var early: Dictionary = fixture.portrait_state()
	print("PORTRAIT_EARLY skin=", skin, " pending=", early.pending, " model_shown=", early.model_shown, " pixels=", lit_pixels(early.image))
	capture("portrait-early.png", early)
	var ready := false
	var started := Time.get_ticks_msec()
	while Time.get_ticks_msec() - started < 60000:
		if not check_error(fixture.tick()):
			return false
		await process_frame
		await RenderingServer.frame_post_draw
		var state: Dictionary = fixture.portrait_state()
		if state.model_shown and state.mask_loaded and not state.pending:
			ready = true
			break
	if not ready:
		push_error("Player portrait did not finish loading within 60 seconds")
		return false
	await settle(10)
	var shown: Dictionary = fixture.portrait_state()
	var lit := lit_pixels(shown.image)
	print("PORTRAIT_READY skin=", skin, " pending=", shown.pending, " model_shown=", shown.model_shown, " pixels=", lit, " appearance=", shown.appearance)
	capture("portrait-ready.png", shown)
	await settle(120)
	var late: Dictionary = fixture.portrait_state()
	print("PORTRAIT_LATE skin=", skin, " pixels=", lit_pixels(late.image))
	capture("portrait-late.png", late)
	if lit < 20:
		# Diagnose one-shot render timing, not an implicit acceptance retry.
		fixture.redraw()
		await settle(10)
		var redrawn: Dictionary = fixture.portrait_state()
		print("PORTRAIT_DIAGNOSTIC_REDRAW skin=", skin, " pixels=", lit_pixels(redrawn.image))
		capture("portrait-redrawn.png", redrawn)
		push_error("Loaded player portrait is black; diagnostic redraw is not normal proof")
		return false
	print("PASS capturepolish_player_portrait_ready_is_nonblack_without_redraw skin=", skin)
	return true

func settle(frames: int) -> void:
	for frame in range(frames):
		await process_frame
		await RenderingServer.frame_post_draw

func lit_pixels(image: Image) -> int:
	if image == null or image.is_empty():
		return 0
	var count := 0
	for y in range(image.get_height()):
		for x in range(image.get_width()):
			var pixel := image.get_pixel(x, y)
			if pixel.a > 0.5 and maxf(pixel.r, maxf(pixel.g, pixel.b)) > 0.12:
				count += 1
	return count

func capture(name: String, state: Dictionary) -> void:
	var filename := skin + "-" + name
	root.get_texture().get_image().save_png(output.path_join(filename))
	var image: Image = state.image
	if image != null:
		image.save_png(output.path_join(filename.get_basename() + "-raw.png"))
	state.erase("image")
	var file := FileAccess.open(output.path_join(filename.get_basename() + ".json"), FileAccess.WRITE)
	file.store_string(JSON.stringify(state, "\t"))

func check_error(error: String) -> bool:
	if error.is_empty():
		return true
	push_error(error)
	return false
