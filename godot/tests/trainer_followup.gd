extends SceneTree

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var directory := OS.get_environment("GODOT_TRAINER_EVIDENCE")
	if directory.is_empty():
		push_error("GODOT_TRAINER_EVIDENCE is required")
		quit(1)
		return
	for skin in ["modern", "forever"]:
		for mode in ["default", "hovered", "selected"]:
			OS.set_environment("GODOT_TRAINER_SELECTED", "3908" if mode == "default" else "2963")
			OS.set_environment("GODOT_TRAINER_THREE_COINS", "1")
			var ui = ClassDB.instantiate("RegistryUi")
			root.add_child(ui)
			var host = ClassDB.instantiate("RegistryUi")
			host.layer = 8
			root.add_child(host)
			var method := "show_forever_trainer_preview" if skin == "forever" else "show_trainer_preview"
			var error = ui.call(method)
			if error != "":
				push_error(error)
				quit(1)
				return
			for frame in range(240):
				await process_frame
				ui.find_child("TrainerPreviewPortrait", true, false).call("tick")
				await RenderingServer.frame_post_draw
			if not check_prices(ui):
				quit(1)
				return
			var pointer := Vector2(120, 228) if mode == "hovered" else Vector2(700, 700)
			var motion := InputEventMouseMotion.new()
			motion.position = pointer
			motion.global_position = pointer
			root.push_input(motion)
			for frame in range(6):
				await process_frame
				error = ui.call("update_trainer_preview_tooltip", host, pointer)
				if error != "":
					push_error(error)
					quit(1)
					return
				await RenderingServer.frame_post_draw
			if mode == "hovered":
				var title := host.find_child("TooltipTitle", true, false) as Label
				if title == null or not title.is_visible_in_tree() or title.text != "Bolt of Linen Cloth":
					push_error("Hovered row lacks shared spell tooltip")
					quit(1)
					return
				var has_id := false
				for child in host.find_children("TooltipLine*", "Label", true, false):
					if child.text == "Spell ID: 2963":
						has_id = true
				if not has_id:
					push_error("Trainer tooltip lost requested Spell ID line")
					quit(1)
					return
			var image = root.get_texture().get_image()
			var path = directory.path_join(skin + "-" + mode + ".png")
			if image == null or image.is_empty() or image.save_png(path) != OK:
				push_error("Rendered capture failed: " + path)
				quit(1)
				return
			print("PASS: ", skin, " ", mode, " prices/capture ", path)
			if mode == "hovered":
				error = ui.call("update_trainer_preview_tooltip", host, Vector2(700, 700))
				await process_frame
				var tooltip := host.find_child("TooltipFrame", true, false) as Control
				if error != "" or (tooltip != null and tooltip.is_visible_in_tree()):
					push_error("Trainer tooltip must hide on leave")
					quit(1)
					return
			host.queue_free()
			ui.queue_free()
			await process_frame
	print("PASS: six offline trainer captures, both skins, hover/leave and selected/unselected three coins")
	quit(0)

func check_prices(ui: Node) -> bool:
	var list := ui.find_child("ClassTrainerScrollBox", true, false) as Control
	for spell in [2963, 2964]:
		var row := ui.find_child("ClassTrainerService%d" % spell, true, false) as Control
		for denomination in range(3):
			var coin := ui.find_child("ClassTrainerService%dCostCoin%d" % [spell, denomination], true, false) as Control
			if coin == null or coin.size != Vector2(13, 13):
				push_error("Three denomination price lacks full-size coin")
				return false
			var bounds := coin.get_global_rect()
			if bounds.end.x > row.get_global_rect().end.x - 8.0 or not list.get_global_rect().encloses(bounds):
				push_error("Coin crosses row border/scroll clip: " + str(bounds))
				return false
	return true
