extends SceneTree

# Renders one authored RegistryUi screen standalone for visual comparison.
# GODOT_CAPTURE_SCREEN: character_select | character_create | portrait_party | forever_portrait_party.
# GODOT_CAPTURE_PATH: PNG output.
# castbaranim_preview: GODOT_CASTBAR_SKIN, GODOT_CASTBAR_PHASE, GODOT_CASTBAR_TIME.

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
	for frame in range(120 if screen in ["forever_damage_meter_preview", "castbaranim_preview"] else 3):
		await process_frame
		await RenderingServer.frame_post_draw
	if screen == "chatflush_preview":
		if not chatflush_corner_and_input(ui):
			ui.queue_free()
			quit(1)
			return
		await process_frame
		await RenderingServer.frame_post_draw
	var image = root.get_texture().get_image()
	if image == null or image.is_empty() or image.save_png(output) != OK:
		push_error("UI capture requires a rendering display")
		quit(1)
		return
	if screen == "forever_damage_meter_preview":
		if not forever_meter_matches_reference(ui, image):
			ui.queue_free()
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
	if screen == "castbaranim_preview":
		if not await castbar_snapshot_matches(ui, image, output):
			ui.queue_free()
			quit(1)
			return
	print("PASS: rendered ", screen, " captured")
	ui.queue_free()
	quit(0)

# Production native controls: corner compensation, no neighbour overlap, usable input.
func chatflush_corner_and_input(ui: Node) -> bool:
	var panel = ui.find_child("ChatFrame1FlareSkin", true, false).get_global_rect()
	var input = ui.find_child("ChatFrame1EditBox", true, false)
	var input_rect: Rect2 = input.get_global_rect()
	print("CHATFLUSH_NATIVE panel ", panel, "; input ", input_rect)
	if absf(panel.position.x + 2.0) > 0.01 or absf(panel.end.y - 2.0 - 1080.0) > 0.01:
		push_error("Chat visible edge is not corner-flush: ", panel)
		return false
	for name in ["MainActionBar", "MultiBarBottomLeft", "MultiBarBottomRight", "MainActionBarLeftEndCap", "MainActionBarRightEndCap", "PetActionBar", "PlayerFrame", "BagsBar", "DamageMeterFlareSkin"]:
		var other: Rect2 = ui.find_child(name, true, false).get_global_rect()
		print("CHATFLUSH_NATIVE ", name, " ", other)
		if panel.intersects(other) or input_rect.intersects(other):
			push_error("Chat/input overlaps ", name, ": ", other)
			return false
	if not Rect2(0, 0, 1920, 1080).encloses(input_rect) or not input.is_visible_in_tree():
		push_error("Chat input not visible inside screen: ", input_rect)
		return false
	input.grab_focus()
	input.insert_text_at_caret("chatflush input proof")
	if not input.has_focus() or input.text != "chatflush input proof":
		push_error("Chat input did not accept text")
		return false
	print("PASS: Forever corner, native overlaps and focused text input")
	return true

# Assert actual rendered endpoints, native font size and native icon/title rectangles.
func forever_meter_matches_reference(ui: Node, image: Image) -> bool:
	var title = ui.find_child("DamageMeterTypeName", true, false)
	var title_rect: Rect2 = title.get_global_rect()
	for name in ["DamageMeterSettingsIcon", "DamageMeterSessionDropdownIcon"]:
		var icon = ui.find_child(name, true, false)
		var icon_rect: Rect2 = icon.get_global_rect()
		if absf(icon_rect.get_center().y - title_rect.get_center().y) > 0.5:
			push_error(name, " not centred on title: ", icon_rect, " vs ", title_rect)
			return false
	var threat = ui.find_child("DamageMeterThreatTabName", true, false)
	if threat == null or threat.text != "Threat":
		push_error("Default second tab is not Threat")
		return false
	var expected_colors = [Color(0.25, 0.78, 0.92), Color(0.53, 0.53, 0.93), Color(0.96, 0.55, 0.73), Color(0.78, 0.61, 0.43), Color(0.0, 0.44, 0.87)]
	for row in range(1, 6):
		for suffix in ["Name", "Value"]:
			var label = ui.find_child("DamageMeterEntry%d%s" % [row, suffix], true, false)
			if label.get_theme_font_size("font_size") != 12:
				push_error("Row font is not 12")
				return false
		var fill = ui.find_child("DamageMeterEntry%dStatusBar" % row, true, false)
		var rect: Rect2 = fill.get_global_rect()
		for stop in [0, 1]:
			var x = rect.position.x + 2 if stop == 0 else rect.end.x - 3
			var expected: Color = expected_colors[row - 1] * (0.5 if stop == 0 else 1.0)
			for y in [rect.position.y + 4, rect.end.y - 5]:
				var pixel = image.get_pixel(int(x), int(y))
				if maxf(absf(pixel.r - expected.r), maxf(absf(pixel.g - expected.g), absf(pixel.b - expected.b))) > 0.035:
					push_error("Fill stop/highlight mismatch row ", row, " stop ", stop, " at ", Vector2(x, y), ": ", pixel, " vs ", expected)
					return false
	print("PASS: Forever meter rendered stops, fonts, Threat tab and icon centres")
	return true

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

# Frozen production reducer snapshots, with rendered feedback isolation for finishes.
func castbar_snapshot_matches(ui: Node, image: Image, output: String) -> bool:
	var phase = OS.get_environment("GODOT_CASTBAR_PHASE")
	var skin = OS.get_environment("GODOT_CASTBAR_SKIN")
	var track = ui.find_child("CastingBarBackground", true, false) as Control
	var fill = ui.find_child("CastingBarFill", true, false) as Control
	var label = ui.find_child("CastingBarSpellName", true, false) as Label
	if track == null or fill == null or label == null:
		push_error("Cast snapshot controls missing")
		return false
	var track_rect = track.get_global_rect()
	var fill_rect = fill.get_global_rect()
	var fraction = 0.5 if phase == "midcast" else (0.0 if phase == "channel" else 1.0)
	if absf(fill_rect.size.x - track_rect.size.x * fraction) > 0.01:
		push_error("Cast snapshot fill mismatch: ", fill_rect, " track ", track_rect)
		return false
	if phase in ["interrupted", "failed"] and label.text != ("Interrupted" if phase == "interrupted" else "Failed"):
		push_error("Cast result label mismatch: ", label.text)
		return false
	var spark = ui.find_child("CastingBarSpark", true, false) as Control
	if phase == "midcast":
		if spark == null or absf(spark.get_global_rect().get_center().x - fill_rect.end.x) > 0.01:
			push_error("Cast spark is not on the fill edge")
			return false
	elif spark != null and spark.is_visible_in_tree():
		push_error("Finished cast kept its spark")
		return false
	print("CASTBAR_NATIVE ", skin, " ", phase, " track=", track_rect, " fill=", fill_rect, " label=", label.text)
	var lit_pixels = 0
	for y in range(int(track_rect.position.y), int(track_rect.end.y)):
		for x in range(int(track_rect.position.x), int(track_rect.end.x)):
			var color = image.get_pixel(x, y)
			if maxf(color.r, maxf(color.g, color.b)) > 0.25:
				lit_pixels += 1
	if lit_pixels < 20:
		push_error("Cast art did not render")
		return false
	if phase not in ["finish", "channel"]:
		return true
	var flash = ui.find_child("CastingBarFlash", true, false) as Control
	if flash == null or absf(flash.modulate.a - 0.5) > 0.001:
		push_error("100.100 completion flash is not half-bright")
		return false
	for name in ["CastingBarFlash", "CastingBarEnergyGlow", "CastingBarFlakes01", "CastingBarFlakes02", "CastingBarFlakes03", "CastingBarBaseGlow", "CastingBarWispGlow", "CastingBarSparkles01", "CastingBarSparkles02"]:
		var effect = ui.find_child(name, true, false) as Control
		if effect != null:
			effect.hide()
	for frame in range(2):
		await process_frame
		await RenderingServer.frame_post_draw
	var control = root.get_texture().get_image()
	if control.save_png(output.get_basename() + "-without-feedback.png") != OK:
		push_error("Cannot save cast feedback isolation control")
		return false
	var changed_pixels = 0
	for y in range(image.get_height()):
		for x in range(image.get_width()):
			if image.get_pixel(x, y) != control.get_pixel(x, y):
				changed_pixels += 1
	if changed_pixels < 20:
		push_error("Cast completion FX did not change rendered pixels")
		return false
	print("PASS: cast feedback changed pixels=", changed_pixels)
	return true
