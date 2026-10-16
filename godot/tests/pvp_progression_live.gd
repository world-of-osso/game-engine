extends SceneTree

# Private real-server proof driver; never injects progression or widget state.
var client: Node
var directory: String

func _initialize() -> void:
	Engine.max_fps = 20
	call_deferred("run_private")

func run_private() -> void:
	if OS.get_environment("BG_ENDPOINT") != "127.0.0.1:5941":
		push_error("PvP progression requires private UDP5941")
		quit(1)
		return
	directory = OS.get_environment("BG_DIRECTORY")
	root.size = Vector2i(1920, 1080)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var last := ""
	while true:
		await process_frame
		var path := directory.path_join("command.json")
		if not FileAccess.file_exists(path):
			continue
		var input := FileAccess.get_file_as_string(path)
		if input == last:
			continue
		last = input
		var command: Dictionary = JSON.parse_string(input)
		var result: Dictionary = {"id": command.id}
		match command.action:
			"snapshot":
				result.account = client.account_state()
				result.tooltip = client.tooltip_state()
				result.controls = []
				for node in root.find_children("*", "Control", true, false):
					if node.is_visible_in_tree():
						var rect: Rect2 = node.get_global_rect()
						result.controls.append({"name": str(node.name), "rect": [rect.position.x, rect.position.y, rect.size.x, rect.size.y], "text": str(node.text) if node is Label or node is Button else ""})
			"capture":
				await RenderingServer.frame_post_draw
				result.error = root.get_texture().get_image().save_png(command.path)
			"click", "hover":
				var target := root.find_child(command.name, true, false) as Control
				if target == null or not target.is_visible_in_tree():
					result.error = "Missing visible control: " + str(command.name)
				else:
					var point := target.get_global_rect().get_center()
					var motion := InputEventMouseMotion.new()
					motion.position = point
					motion.global_position = point
					Input.parse_input_event(motion)
					await process_frame
					if command.action == "click":
						for down in [true, false]:
							var mouse := InputEventMouseButton.new()
							mouse.position = point
							mouse.global_position = point
							mouse.button_index = MOUSE_BUTTON_LEFT
							mouse.pressed = down
							Input.parse_input_event(mouse)
							await process_frame
			"key":
				for down in [true, false]:
					var key := InputEventKey.new()
					key.keycode = int(command.key)
					key.physical_keycode = int(command.key)
					key.pressed = down
					Input.parse_input_event(key)
					await process_frame
			"quit":
				client.free()
				quit(0)
				return
		for frame in range(3):
			await process_frame
		var response := FileAccess.open(directory.path_join("response.json"), FileAccess.WRITE)
		response.store_string(JSON.stringify(result))
		response.close()
