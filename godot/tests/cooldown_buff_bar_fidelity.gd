extends SceneTree
# Logical 5.5px bar centres snap to physical pixels; model tests assert exact coordinates.
const PIXEL_SNAP_TOLERANCE = 0.51
# Offline production projection of supplied full/half/empty buff samples.
func _initialize() -> void:
	call_deferred("capture_buff_bars")

func capture_buff_bars() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	var output := OS.get_environment("BUFF_BAR_CAPTURE_PATH")
	if output.is_empty():
		push_error("Set BUFF_BAR_CAPTURE_PATH")
		quit(1)
		return
	for forever in [true]:
		var probe = ClassDB.instantiate("UiAuditProbe")
		root.add_child(probe)
		var error: String = probe.mount_cooldown_buff_bars(forever)
		if not error.is_empty():
			push_error(error)
			quit(1)
			return
		for frame in range(120):
			await process_frame
			await RenderingServer.frame_post_draw
		var ui: Node = probe.find_child("CooldownBuffAuditUI", true, false)
		export_buff_source_regions(ui, output, forever)
		if not verify_buff_geometry(ui):
			root.get_texture().get_image().save_png(output.path_join("cooldown-buff-geometry-failure.png"))
			quit(1)
			return
		var image := root.get_texture().get_image()
		if image.get_size() != Vector2i(1920, 1080):
			push_error("Native buff capture must be 1920x1080")
			quit(1)
			return
		var path := output.path_join("cooldown-buff-bars-%s.png" % ("forever" if forever else "modern"))
		if image.save_png(path) != OK:
			push_error("Cannot save " + path)
			quit(1)
			return
		print("PASS COOLDOWN_BUFF_BARS skin=", "forever" if forever else "modern", " source background/fill/pip full/half/empty capture=", path)
		probe.queue_free()
		await process_frame
	quit(0)

func export_buff_source_regions(ui: Node, output: String, forever: bool) -> void:
	for suffix in ["BarBackground", "BarFill", "BarPip"]:
		var region := ui.find_child("CooldownViewer902" + suffix, true, false) as Control
		for part in region.find_children("*", "TextureRect", true, false):
			var source: Image = part.texture.get_image()
			source.save_png(output.path_join("buff-source-%s-%s.png" % ["forever" if forever else "modern", suffix]))
			print("BUFF_SOURCE ", suffix, " size=", source.get_size(), " rect=", region.get_global_rect(), " z=", region.z_index)
	var label := ui.find_child("CooldownViewer901Name", true, false) as Label
	print("BUFF_NAME_LAYER z=", label.z_index)

func verify_buff_geometry(ui: Node) -> bool:
	for index in range(3):
		var prefix := "CooldownViewer%d" % (900 + index)
		var fraction := index * 0.5
		var row := ui.find_child(prefix, true, false) as Control
		var bar := ui.find_child(prefix + "Cooldown", true, false) as Control
		var background := ui.find_child(prefix + "BarBackground", true, false) as Control
		var fill := ui.find_child(prefix + "BarFill", true, false) as Control
		var pip := ui.find_child(prefix + "BarPip", true, false) as Control
		if row == null or bar == null or background == null or fill == null or pip == null:
			push_error("Missing source buff regions: " + prefix)
			return false
		if row.size.distance_to(Vector2(220, 30)) > 0.01 or bar.size.distance_to(Vector2(188, 19)) > 0.01:
			push_error("Buff row/bar differs from source template")
			return false
		var applications := ui.find_child(prefix + "Charges", true, false) as Label
		if applications == null or applications.text != "2":
			push_error("Applications count missing")
			return false
		# XML:237-241 gives 32x10 at BOTTOMRIGHT(-5,+5). Native Label may grow to its font height.
		if applications.size.x != 32 or absf(applications.get_global_rect().end.x - row.get_global_rect().position.x - 25) > PIXEL_SNAP_TOLERANCE or absf(applications.global_position.y - row.global_position.y - 15) > PIXEL_SNAP_TOLERANCE:
			push_error("Applications must retain XML size/anchor away from icon corner")
			return false
		if applications.get_global_rect().end.y > row.get_global_rect().end.y + PIXEL_SNAP_TOLERANCE:
			push_error("Applications glyph must remain inside icon bottom")
			return false
		print("BUFF_APPLICATIONS ", prefix, " rect=", applications.get_global_rect(), " text=", applications.text)
		print("BUFF_ANCHORS ", prefix, " row=", row.global_position, " bar=", bar.global_position, " background=", background.global_position, " pip=", pip.global_position)
		if bar.global_position.distance_to(row.global_position + Vector2(32, 5.5)) > PIXEL_SNAP_TOLERANCE:
			push_error("Buff bar must anchor to icon RIGHT +2 and vertical centre")
			return false
		if background.size.distance_to(Vector2(194, 28)) > 0.01 or background.global_position.distance_to(bar.global_position + Vector2(-2, -2)) > 0.01:
			push_error("Buff background must retain source edge overhang")
			return false
		var name_label := ui.find_child(prefix + "Name", true, false) as Label
		var duration := ui.find_child(prefix + "Timer", true, false) as Label
		if name_label == null or duration == null or name_label.horizontal_alignment != HORIZONTAL_ALIGNMENT_LEFT:
			push_error("Buff name must use source left alignment")
			return false
		if absf(name_label.global_position.x - bar.global_position.x - 5) > PIXEL_SNAP_TOLERANCE:
			push_error("Buff name must retain source LEFT +5 anchor")
			return false
		print("BUFF_GEOMETRY ", prefix, " fraction=", fraction, " fill=", fill.size, " pip=", pip.size, " visible=", pip.visible)
		if fraction == 0:
			if fill.visible or pip.visible:
				push_error("Expired buff must hide fill and pip")
				return false
			continue
		# Clipped labels report a 1px minimum; measure their actual glyph width instead.
		var text_width := duration.get_theme_font("font").get_string_size(duration.text, HORIZONTAL_ALIGNMENT_LEFT, -1, duration.get_theme_font_size("font_size")).x
		print("BUFF_DURATION ", prefix, " rect=", duration.get_global_rect(), " glyph_width=", text_width, " bar_end=", bar.get_global_rect().end)
		if absf(duration.get_global_rect().end.x - (bar.get_global_rect().end.x - 8)) > PIXEL_SNAP_TOLERANCE or absf(duration.size.x - text_width) > PIXEL_SNAP_TOLERANCE:
			push_error("Buff duration must keep natural width at source RIGHT -8")
			return false
		if absf(fill.size.x - 188 * fraction) > 0.01 or pip.size.distance_to(Vector2(10, 46)) > 0.01:
			push_error("Buff remaining fraction or source pip size differs")
			return false
		if pip.global_position.distance_to(bar.global_position + Vector2(188 * fraction - 5, -12.5)) > PIXEL_SNAP_TOLERANCE or not pip.visible:
			push_error("Buff pip must follow fill RIGHT with source CENTER y=-1")
			return false
	return true
