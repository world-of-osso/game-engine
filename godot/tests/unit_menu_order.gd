extends SceneTree

var completion = preload("res://tests/completion.gd").new(self, 4)

func _initialize() -> void:
	call_deferred("run_completion")

func run_completion() -> void:
	completion.finish(await check_submenus())

func check_submenus() -> bool:
	root.size = Vector2i(1920, 1080)
	for forever in [true]:
		for raid in [false, true]:
			var ui: Node = ClassDB.instantiate("RegistryUi")
			root.add_child(ui)
			var error: String = ui.call("show_projection_unit_menu_preview", forever, raid)
			if not completion.require(error == "", error):
				return false
			for frame in range(4):
				await process_frame
			var prefix := "UnitFrameRaidDifficultyMenu" if raid else "UnitFrameDifficultyMenu"
			var submenu := ui.find_child(prefix + "Background", true, false) as Control
			var parent := ui.find_child("UnitFrameContextMenuBackground", true, false) as Control
			if not completion.require(submenu != null and parent != null, "actual parent/submenu backgrounds"):
				return false
			print("UNIT_MENU_ORDER forever=", forever, " raid=", raid, " submenu_z=", submenu.z_index, " parent_z=", parent.z_index)
			if not completion.require(submenu.is_visible_in_tree() and parent.is_visible_in_tree() and submenu.z_index > parent.z_index, "submenu must draw over parent " + prefix):
				return false
			if not completion.case_done():
				return false
			ui.free()
	return true
