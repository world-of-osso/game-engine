extends SceneTree

# Offline production guild-bank projection. Set GUILD_BANK_TABS_OUTPUT to an existing directory.
func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	var output := OS.get_environment("GUILD_BANK_TABS_OUTPUT")
	if output.is_empty():
		push_error("GUILD_BANK_TABS_OUTPUT is required")
		quit(1)
		return
	for forever in [true]:
		var ui = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		var error: String = ui.call("show_buttonfit_service_preview", "guildbank", forever)
		if not error.is_empty():
			push_error(error)
			quit(1)
			return
		for frame in range(5):
			await process_frame
			await RenderingServer.frame_post_draw
		var skin := "forever" if forever else "modern"
		var path := output.path_join(skin + ".png")
		if root.get_texture().get_image().save_png(path) != OK:
			push_error("Could not save " + path)
			quit(1)
			return
		var passed := check_tabs(ui, skin)
		ui.free()
		if not passed:
			quit(1)
			return
	quit(0)

func check_tabs(ui: Node, skin: String) -> bool:
	var previous: Control = null
	for index in range(1, 5):
		var tab := ui.find_child("GuildBankFrameTab%d" % index, true, false) as Control
		var text := ui.find_child("GuildBankFrameTab%dText" % index, true, false) as Label
		if tab == null or text == null:
			push_error("Missing bottom tab %d" % index)
			return false
		var width := text.get_theme_font("font").get_string_size(text.text, HORIZONTAL_ALIGNMENT_LEFT, -1, text.get_theme_font_size("font_size")).x
		print(skin, " tab=", index, " rect=", tab.get_global_rect(), " text=", width)
		if tab.size.x + 0.01 < width + 20.0:
			push_error("%s tab %d lacks Retail text padding" % [skin, index])
			return false
		if previous != null and absf(tab.position.x - (previous.position.x + previous.size.x + 3.0)) > 0.01:
			push_error("%s tab %d does not have the Retail 3px gap" % [skin, index])
			return false
		previous = tab
	for index in range(1, 4):
		var side := ui.find_child("GuildBankTab%d" % index, true, false) as Control
		if side == null or not side.is_visible_in_tree():
			push_error("%s side tab %d missing" % [skin, index])
			return false
	return true
