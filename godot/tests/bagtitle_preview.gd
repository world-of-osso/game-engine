extends SceneTree

# Production native Label + portrait chrome, no account/server. Raising only the title
# must not reveal previously occluded glyph pixels. Includes Retail's narrow long-name
# case: right ellipsis is intentional, missing leading glyphs are not.
func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	var output := OS.get_environment("BAGTITLE_SHOTS")
	DirAccess.make_dir_recursive_absolute(output)
	var failures := []
	for forever in [false, true]:
		var skin := "forever" if forever else "modern"
		var ui = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		var error: String = ui.show_bagtitle_preview(forever)
		if not error.is_empty():
			fail(error)
			return
		await settle()
		# Separate windows for a readable capture without modifying authored title layout.
		for index in range(3):
			var container := ui.find_child("ContainerFrame" + str(index), true, false) as Control
			container.position = Vector2(150 + index * 260, 180)
		await settle()
		var original := root.get_texture().get_image()
		original.save_png(output.path_join(skin + "-before.png"))
		for index in range(3):
			var title := ui.find_child("ContainerFrame" + str(index) + "TitleText", true, false) as Label
			var font := title.get_theme_font("font")
			var measured := font.get_string_size(title.text, HORIZONTAL_ALIGNMENT_LEFT, -1, title.get_theme_font_size("font_size")).x
			if index < 2 and measured > title.size.x:
				fail(skin + " full title exceeds its authored rect: " + title.text)
				return
			var previous_z := title.z_index
			title.z_index = 4095
			await settle()
			var unobstructed := root.get_texture().get_image()
			var bounds := Rect2i(title.get_global_rect())
			var changed := 0
			for y in range(bounds.position.y, bounds.end.y):
				for x in range(bounds.position.x, bounds.end.x):
					if original.get_pixel(x, y) != unobstructed.get_pixel(x, y):
						changed += 1
			title.z_index = previous_z
			await settle()
			if changed != 0:
				failures.append("%s %s: %d title pixels hidden by container chrome" % [skin, title.text, changed])
			print("TITLE ", skin, " ", title.text, " rect=", bounds, " measured=", measured, " occluded_pixels=", changed)
		if original.save_png(output.path_join(skin + ".png")) != OK:
			fail("save screenshot")
			return
		ui.free()
		await process_frame
	if not failures.is_empty():
		fail("; ".join(failures))
		return
	print("PASS bagtitle both skins")
	quit(0)

func settle() -> void:
	for frame in range(120):
		await process_frame
		await RenderingServer.frame_post_draw

func fail(message: String) -> void:
	push_error(message)
	quit(1)
