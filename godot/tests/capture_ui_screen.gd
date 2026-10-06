extends SceneTree

# Renders one authored RegistryUi screen standalone for visual comparison.
# GODOT_CAPTURE_SCREEN: character_select | character_create | portrait_party | forever_portrait_party.
# GODOT_CAPTURE_PATH: PNG output.

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var screen = OS.get_environment("GODOT_CAPTURE_SCREEN")
	var output = OS.get_environment("GODOT_CAPTURE_PATH")
	var ui = ClassDB.instantiate("RegistryUi")
	root.add_child(ui)
	var method = "show_" + screen
	if output.is_empty() or not ui.has_method(method):
		push_error("Set GODOT_CAPTURE_PATH and a valid GODOT_CAPTURE_SCREEN")
		quit(1)
		return
	var error = ui.call(method)
	if error != "":
		push_error(error)
		quit(1)
		return
	for frame in range(3):
		await process_frame
		await RenderingServer.frame_post_draw
	var image = root.get_texture().get_image()
	if image == null or image.is_empty() or image.save_png(output) != OK:
		push_error("UI capture requires a rendering display")
		quit(1)
		return
	if screen in ["portrait_party", "forever_portrait_party"]:
		if not offline_party_health_is_desaturated(image):
			ui.queue_free()
			quit(1)
			return
	if screen in ["guild_ranks_preview", "forever_guild_ranks_preview"]:
		if not guild_rank_footer_has_two_lines(image):
			ui.queue_free()
			quit(1)
			return
	print("PASS: rendered ", screen, " captured")
	ui.queue_free()
	quit(0)

# The rank disclaimer occupies two single-line rows inside the left column.
# Assert real rendered pixels; the previous 20px multiline label overlapped both rows.
func guild_rank_footer_has_two_lines(image: Image) -> bool:
	if image.get_width() != 1920 or image.get_height() != 1080:
		push_error("Guild rank capture requires 1920x1080")
		return false
	for band in [Vector2i(742, 762), Vector2i(764, 788)]:
		var gold_pixels = 0
		for y in range(band.x, band.y):
			for x in range(459, 667):
				var color = image.get_pixel(x, y)
				if color.r > 0.6 and color.g > 0.4 and color.b < 0.35:
					gold_pixels += 1
		if gold_pixels < 10:
			push_error("Guild rank disclaimer line missing in band ", band)
			return false
	return true

# Concrete four-member preview: third member at (22,147+2*63), health (45,19).
# Sample right of the Offline label, inside the full offline fill under both skins.
func offline_party_health_is_desaturated(image: Image) -> bool:
	for pixel in [Vector2i(130, 294), Vector2i(130, 296)]:
		var color = image.get_pixelv(pixel)
		var spread = maxf(color.r, maxf(color.g, color.b)) - minf(color.r, minf(color.g, color.b))
		if spread > 2.0 / 255.0 or color.r < 0.1:
			push_error("Offline party health is not filled/desaturated at ", pixel, ": ", color)
			return false
	return true
