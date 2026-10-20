extends SceneTree

var client: Node

# Private-server native-input proof driver. No fabricated queue or match state.
var directory := ""
func _initialize() -> void:
	Engine.max_fps = 20
	call_deferred("live_run")
func live_run() -> void:
	var endpoint := OS.get_environment("ARENA_ENDPOINT")
	if not endpoint.begins_with("127.0.0.1:") or endpoint.get_slice(":", 1) != "5941":
		fail("Arena proof requires private UDP5941")
		return
	directory = OS.get_environment("ARENA_DIRECTORY")
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
				result.target = client.target_state()
				result.controls = []
				for node in root.find_children("*", "Control", true, false):
					if node.is_visible_in_tree():
						var rect: Rect2 = node.get_global_rect()
						result.controls.append({"name": str(node.name), "rect": [rect.position.x, rect.position.y, rect.size.x, rect.size.y], "text": str(node.text) if node is Label or node is Button else ""})
			"objects":
				result.replication = client.mail_state().get("replicated_objects", [])
				result.objects = []
				for node in root.find_children("Mailbox_*", "Node3D", true, false):
					if not node.has_meta("game_object_entry"):
						continue
					var model = node.get_node_or_null("GameObjectModel")
					var collision = node.get_node_or_null("GameObjectModel/M2Collision") as StaticBody3D
					result.objects.append({"entry": node.get_meta("game_object_entry"), "server_id": node.get_meta("game_object_server_id"), "position": str(node.global_position), "model_fdid": model.get_meta("model_file_data_id") if model else null, "collision_layer": collision.collision_layer if collision else null})
			"audio":
				result.played_kits = client.merchant_state().get("ui_sounds", [])
				result.players = []
				var sound_root := root.find_child("UiSoundKits", true, false)
				if sound_root != null:
					for node in sound_root.get_children():
						if node is AudioStreamPlayer:
							result.players.append({"playing": node.playing, "position": node.get_playback_position(), "gain": node.volume_linear, "length": node.stream.get_length()})
			"benchmark_collision":
				result.measurement = client.benchmark_arena_collision()
			"capture":
				await frames(3)
				await RenderingServer.frame_post_draw
				result.error = root.get_texture().get_image().save_png(command.path)
			"click":
				var target = root.find_child(command.name, true, false) as Control
				if target == null or not target.is_visible_in_tree():
					result.error = "Missing visible native control: " + str(command.name)
				else:
					await pointer(target.get_global_rect().get_center(), MOUSE_BUTTON_LEFT)
			"scroll":
				var target = root.find_child(command.name, true, false) as Control
				if target == null or not target.is_visible_in_tree():
					result.error = "Missing visible native control: " + str(command.name)
				else:
					var button := MOUSE_BUTTON_WHEEL_DOWN if command.steps > 0 else MOUSE_BUTTON_WHEEL_UP
					for _step in range(abs(int(command.steps))):
						await pointer(target.get_global_rect().get_center(), button)
			"key":
				await tap(int(command.key))
			"key_down", "key_up":
				push_key(int(command.key), command.action == "key_down")
			"quit":
				client.free()
				quit(0)
		await frames(3)
		var response := FileAccess.open(directory.path_join("response.json"), FileAccess.WRITE)
		response.store_string(JSON.stringify(result))
		response.close()

func pointer(point: Vector2, button: MouseButton) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await process_frame
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = button
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await frames(3)

func push_key(key: Key, pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = key
	event.physical_keycode = key
	event.pressed = pressed
	root.push_input(event, true)

func tap(key: Key) -> void:
	for pressed in [true, false]:
		push_key(key, pressed)
		await process_frame
	await frames(3)

func frames(count: int) -> void:
	for _frame in range(count):
		await process_frame

func fail(message: String) -> void:
	push_error("FIXTURE NATIVE_ARENA: " + message)
	quit(1)
