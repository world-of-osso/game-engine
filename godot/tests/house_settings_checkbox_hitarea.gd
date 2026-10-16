extends SceneTree
# Supplied draft: physical source hit targets and production staging, not server persistence.
var output: String
var verify: bool
var actions: Array[String] = []

func _initialize() -> void:
	call_deferred("run_checkboxes")

func settle() -> void:
	for frame in range(8):
		await process_frame
		await RenderingServer.frame_post_draw

func drain(ui: Node) -> void:
	for action in ui.house_access_preview_actions():
		actions.append(action)

func move(ui: Node, point: Vector2) -> void:
	var event := InputEventMouseMotion.new()
	event.position = point
	event.global_position = point
	Input.parse_input_event(event)
	await process_frame
	drain(ui)
	await settle()

func press(ui: Node, point: Vector2, down: bool) -> void:
	var event := InputEventMouseButton.new()
	event.position = point
	event.global_position = point
	event.button_index = MOUSE_BUTTON_LEFT
	event.pressed = down
	Input.parse_input_event(event)
	await process_frame
	drain(ui)
	await settle()

func control(ui: Node, name: String) -> Control:
	var found := ui.find_child(name, true, false) as Control
	assert(found != null, name)
	return found

func capture(ui: Node, label: String) -> void:
	var image := root.get_texture().get_image()
	assert(image.save_png(output.path_join(label + "-context.png")) == OK)
	var panel := control(ui, "HousingHouseSettingsFrame")
	var crop := image.get_region(Rect2i(panel.global_position + Vector2(230, 280), Vector2(180, 180)))
	assert(crop.save_png(output.path_join(label + "-crop.png")) == OK)

func run_checkboxes() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	output = OS.get_environment("HOUSE_CHECKBOX_CAPTURE_PATH")
	verify = OS.get_environment("HOUSE_CHECKBOX_VERIFY") == "1"
	assert(not output.is_empty())
	for forever in [false, true]:
		var skin := "forever" if forever else "modern"
		for scenario in [["limited", false], ["anyone", false], ["none", false], ["none", true]]:
			var label: String = skin + "-" + scenario[0] + ("-pending" if scenario[1] else "-current")
			var enabled: bool = scenario[0] == "limited" and not scenario[1]
			var ui = ClassDB.instantiate("RegistryUi")
			root.add_child(ui)
			assert(ui.show_house_access_preview(forever, scenario[1]).is_empty())
			if not scenario[1]:
				assert(ui.apply_house_access_preview_action("housing:settings:" + scenario[0]).is_empty())
			await settle()
			await move(ui, Vector2(150, 500))
			capture(ui, label + "-initial")
			for key in ["neighbors", "guild", "friends", "party"]:
				var name: String = "HousingSettingsFlag" + key
				var action: String = "housing:settings:toggle:" + key
				var checkbox := control(ui, name)
				var box := control(ui, name + "Box")
				var caption := control(ui, name + "Label")
				if verify:
					assert(checkbox.size == Vector2(30, 29))
					var row := control(ui, name + "Row")
					assert(row.size == Vector2(100, 30))
					# Native scale-1 Taffy layout rounds the source half-pixel anchor.
					var expected := (row.global_position + Vector2(0, 0.5)).round()
					assert(checkbox.global_position == expected, str(checkbox.get_global_rect()) + " source rounded=" + str(expected))
				var label_point := caption.global_position + Vector2(5, caption.size.y / 2.0)
				await move(ui, label_point)
				await press(ui, label_point, true)
				await press(ui, label_point, false)
				if verify or not enabled:
					assert(actions.is_empty(), "Label is not a checkbox hit target")
				else:
					assert(actions == [action], "Baseline reproduces incorrect label dispatch")
					print("BASELINE_LABEL_DISPATCH ", label, " ", key)
				actions.clear()
				var point := box.get_global_rect().get_center()
				await move(ui, point)
				await press(ui, point, true)
				await move(ui, Vector2(150, 500))
				await press(ui, Vector2(150, 500), false)
				assert(actions.is_empty(), "Outside release cancels")
				await move(ui, point)
				await press(ui, point, true)
				await press(ui, point, false)
				if enabled:
					assert(actions == [action], "Exact checkbox action retained")
					assert(ui.apply_house_access_preview_action(action).is_empty())
					await settle()
					assert(ui.find_child(name + "Tick", true, false) != null, "Real draft reducer updates checked art")
				else:
					assert(actions.is_empty(), "Mode/pending checkbox dispatch refused")
				actions.clear()
				await move(ui, Vector2(150, 500))
				capture(ui, label + "-" + key)
				print("HOUSE_CHECKBOX ", label, " ", key, " hit/cancel/staging PASS")
			ui.queue_free()
			await process_frame
	print("PASS house-checkbox both-skin hit/label/cancel/staging; verified=", verify)
	quit(0)
