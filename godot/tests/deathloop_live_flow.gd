extends SceneTree

var client: Node
var artifacts := OS.get_environment("DEATHLOOP_ARTIFACTS")
var number := OS.get_environment("DEATHLOOP_ACCOUNT")
var command_path := artifacts.path_join("command" + number + ".json")

func _initialize() -> void:
	call_deferred("start")

func start() -> void:
	root.size = Vector2i(1920, 1080)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var deadline := Time.get_ticks_msec() + 180000
	while client.account_state().get("assets_starting", true) and Time.get_ticks_msec() < deadline:
		await process_frame
	print("DEATHLOOP CONNECT ", client.connect_account(OS.get_environment("DEATHLOOP_SERVER"), OS.get_environment("DEATHLOOP_USERNAME"), OS.get_environment("DEATHLOOP_PASSWORD"), false))
	deadline = Time.get_ticks_msec() + 90000
	while client.account_state().screen != "CharacterSelect" and Time.get_ticks_msec() < deadline:
		await process_frame
	var ui := client.get_node_or_null("CharacterSelectUI")
	if ui == null:
		print("DEATHLOOP BLOCKED ", client.account_state())
		return
	await click(ui.find_child("CharCard_0", true, false))
	await click(ui.find_child("EnterWorld", true, false))
	print("DEATHLOOP ENTER SUBMITTED")
	while true:
		await process_frame
		if not FileAccess.file_exists(command_path):
			continue
		var command: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(command_path))
		DirAccess.remove_absolute(command_path)
		print("DEATHLOOP COMMAND ", command)
		match command.get("action", ""):
			"click":
				await click(client.find_child(command.name, true, false))
			"key":
				var event := InputEventKey.new()
				event.keycode = int(command.code)
				event.physical_keycode = int(command.code)
				event.pressed = true
				Input.parse_input_event(event)
				await process_frame
				event.pressed = false
				Input.parse_input_event(event)
			"mouse":
				var event := InputEventMouseButton.new()
				event.position = Vector2(command.x, command.y)
				event.button_index = int(command.get("button", 2))
				var motion := InputEventMouseMotion.new()
				motion.position = event.position
				Input.parse_input_event(motion)
				await process_frame
				event.pressed = true
				Input.parse_input_event(event)
				await process_frame
				event.pressed = false
				Input.parse_input_event(event)
			"call":
				var object: Node = client
				if command.has("node"):
					object = client.get_node(command.node)
				print("DEATHLOOP CALL ", object.callv(command.method, command.get("args", [])))
			"capture":
				await RenderingServer.frame_post_draw
				root.get_texture().get_image().save_png(artifacts.path_join(command.stem + ".png"))
				var environment: WorldEnvironment = client.find_child("Environment", true, false)
				var saturation := 1.0
				if environment != null and environment.environment.adjustment_enabled:
					saturation = environment.environment.adjustment_saturation
				var state := {"saturation": saturation, "group": client.group_state(), "account": client.account_state(), "minimap": client.minimap_state(), "world_map": client.world_map_state(), "auras": client.aura_state(), "character": client.character_frame_state(), "target": client.target_state(), "controls": [], "meshes": []}
				for node in client.find_children("*", "Control", true, false):
					if node.is_visible_in_tree():
						var item := {"path": str(node.get_path()), "rect": str(node.get_global_rect())}
						if node is Label or node is RichTextLabel:
							item.text = node.text
						state.controls.append(item)
				for node in client.find_children("*", "MeshInstance3D", true, false):
					if node.transparency > 0.0:
						state.meshes.append({"path": str(node.get_path()), "transparency": node.transparency})
				FileAccess.open(artifacts.path_join(command.stem + "-state.json"), FileAccess.WRITE).store_string(JSON.stringify(state, "\t"))
				print("DEATHLOOP CAPTURE ", command.stem)
				if command.has("expect"):
					verify(state, command.expect, command.stem)

func click(control: Control) -> void:
	if control == null:
		print("DEATHLOOP missing control")
		return
	var point := control.get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = point
	Input.parse_input_event(motion)
	await process_frame
	var event := InputEventMouseButton.new()
	event.position = point
	event.button_index = MOUSE_BUTTON_LEFT
	event.pressed = true
	Input.parse_input_event(event)
	await process_frame
	event.pressed = false
	Input.parse_input_event(event)
	await process_frame

# Native behavioral checks run against authoritative private-server transitions,
# actual Environment, rendered map pins, and released world/transport resources.
func verify(state: Dictionary, expected: Dictionary, stem: String) -> void:
	var checks: Array = []
	if expected.has("saturation"):
		checks.append({"name": "ghost grading", "pass": is_equal_approx(state.saturation, float(expected.saturation)), "actual": state.saturation})
	if expected.has("corpse_pin"):
		var present := false
		for pin in state.world_map.get("pins", []):
			present = present or pin.type == "Corpse"
		checks.append({"name": "world-map corpse pin", "pass": present == bool(expected.corpse_pin), "actual": present})
	if expected.has("world_attached"):
		checks.append({"name": "world-exit release", "pass": state.account.world_attached == bool(expected.world_attached), "actual": state.account.world_attached})
	if expected.has("connected"):
		checks.append({"name": "logout transport", "pass": state.account.connected == bool(expected.connected), "actual": state.account.connected})
	if expected.has("ghost_mesh"):
		checks.append({"name": "ghost model", "pass": (not state.meshes.is_empty()) == bool(expected.ghost_mesh), "actual": not state.meshes.is_empty()})
	FileAccess.open(artifacts.path_join(stem + "-checks.json"), FileAccess.WRITE).store_string(JSON.stringify(checks, "\t"))
	for check in checks:
		print("DEATHLOOP CHECK ", JSON.stringify(check))
