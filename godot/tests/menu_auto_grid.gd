extends SceneTree

var completion = preload("res://tests/completion.gd").new(self, 4)

func _initialize() -> void:
	call_deferred("run_completion")

func run_completion() -> void:
	completion.finish(await check_grid())

func check_grid() -> bool:
	var skin := OS.get_environment("GODOT_MENU_GRID_SKIN")
	var directory := OS.get_environment("GODOT_MENU_GRID_CAPTURES")
	if not completion.require(skin in ["modern", "forever"] and not directory.is_empty(), "explicit skin and capture directory required"):
		return false
	root.size = Vector2i(1920, 1080)
	for fixture in [[7, 1, 7], [11, 2, 6], [25, 3, 9], [37, 4, 10]]:
		var count: int = fixture[0]
		var columns: int = fixture[1]
		var rows: int = fixture[2]
		var ui: Node = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		var error: String = ui.call("show_menu_auto_grid_preview", count, skin == "forever")
		if not completion.require(error.is_empty(), error):
			return false
		for _frame in range(60):
			await process_frame
			await RenderingServer.frame_post_draw
		var menu := ui.find_child("AutoGridFixture", true, false) as Control
		var first: Control = ui.call("control_for_action", "preview:grid:0")
		if not completion.require(menu != null and first != null, "actual projected menu and first cell"):
			return false
		var image := root.get_texture().get_image()
		if not completion.require(image != null and not image.is_empty() and image.get_size() == Vector2i(1920, 1080), "rendered1920x1080 viewport"):
			return false
		var saved := image.save_png(directory.path_join("%s-count%d.png" % [skin, count]))
		if not completion.require(saved == OK, "save original grid PNG"):
			return false
		var menu_rect := menu.get_global_rect()
		var cell_width := first.size.x
		print("MENU_AUTO_GRID skin=", skin, " count=", count, " rect=", menu_rect, " cell_width=", cell_width, " expected_columns=", columns, " expected_rows=", rows)
		if not completion.require(is_equal_approx(menu.size.x, columns * cell_width + 16.0), "source auto columns " + skin + " count" + str(count)):
			return false
		if not completion.require(is_equal_approx(menu.size.y, rows * 20.0 + 23.0), "source grid row extent"):
			return false
		for index in range(count):
			var cell: Control = ui.call("control_for_action", "preview:grid:" + str(index))
			if not completion.require(cell != null and cell.is_visible_in_tree(), "projected grid cell " + str(index)):
				return false
			var column := floori(float(index) / float(rows))
			var row := index % rows
			var expected := menu_rect.position + Vector2(8.0 + column * cell_width, 8.0 + row * 20.0)
			if not completion.require(cell.get_global_rect().position.is_equal_approx(expected) and cell.size.is_equal_approx(Vector2(cell_width, 20.0)), "source vertical grid cell " + str(index)):
				return false
		var last: Control = ui.call("control_for_action", "preview:grid:" + str(count - 1))
		await click_point(last.get_global_rect().get_center())
		if not completion.require(str(ui.call("pop_action")) == "preview:grid:" + str(count - 1), "native last-column click action"):
			return false
		if not completion.require(str(ui.call("pop_action")).is_empty(), "one action per click"):
			return false
		if not completion.case_done():
			return false
		ui.free()
	return true

func click_point(point: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion)
	await process_frame
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = pressed
		event.button_mask = MOUSE_BUTTON_MASK_LEFT if pressed else 0
		root.push_input(event)
		await process_frame
