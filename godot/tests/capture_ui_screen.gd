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
	if screen == "forever_minimap_preview":
		if not await forever_minimap_has_opaque_header_and_badge(ui, image, output):
			ui.queue_free()
			quit(1)
			return
	print("PASS: rendered ", screen, " captured")
	ui.queue_free()
	quit(0)

# Production display begins below the 22-unit band; no tile pixels leak into it.
# Badge's gold ring must draw inside its derived rect, with empty space to the magnifier.
func forever_minimap_has_opaque_header_and_badge(ui: Node, image: Image, output: String) -> bool:
	if image.get_size() != Vector2i(1920, 1080):
		push_error("Minimap capture requires 1920x1080")
		return false
	for x in range(1700, 1900):
		var color = image.get_pixel(x, 29)
		if maxf(color.r, maxf(color.g, color.b)) > 1.0 / 255.0:
			push_error("Map leaked into the minimap title band at ", x, ": ", color)
			return false
	var badge = ui.find_child("MinimapDayNightBadge", true, false) as Control
	var launcher = ui.find_child("MinimapLauncherButton", true, false) as Control
	var band = ui.find_child("MinimapTitleBand", true, false) as Control
	var tracker = ui.find_child("ObjectiveTrackerFrameHeaderBackground", true, false) as Control
	if badge == null or launcher == null or band == null or tracker == null:
		push_error("Minimap capture controls missing")
		return false
	var badge_rect = Rect2(1662, 226, 33, 32)
	if badge.get_global_rect() != badge_rect or launcher.get_global_rect() != Rect2(1624, 230, 30, 30):
		push_error("Badge or approved magnifier placement changed")
		return false
	if band.get_global_rect() != Rect2(1668, 8, 244, 22):
		push_error("Title band size/inset changed")
		return false
	if badge_rect.intersects(launcher.get_global_rect()) or tracker.get_global_rect().intersects(Rect2(1660, 0, 260, 260)):
		push_error("Minimap overlaps approved neighboring controls")
		return false
	badge.hide()
	for frame in range(2):
		await process_frame
		await RenderingServer.frame_post_draw
	var without_badge = root.get_texture().get_image()
	if without_badge.save_png(output.get_basename() + "-without-badge.png") != OK:
		push_error("Cannot save badge isolation control")
		return false
	badge.show()
	var changed_pixels = 0
	for y in range(200, 275):
		for x in range(1624, 1920):
			if image.get_pixel(x, y) == without_badge.get_pixel(x, y):
				continue
			if not badge_rect.has_point(Vector2(x, y)):
				push_error("Badge changed pixels outside its rect: ", Vector2i(x, y))
				return false
			changed_pixels += 1
	if changed_pixels < 100:
		push_error("Badge art did not render: ", changed_pixels)
		return false
	print("PASS: opaque 22-unit minimap band; isolated badge pixels=", changed_pixels, "; magnifier/tracker clear")
	return true

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
