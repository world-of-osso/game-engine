extends SceneTree

# Private-server native-input proof driver. No fabricated queue or match state.
var directory := ""
var client: Node
func _initialize() -> void:
	Engine.max_fps = 20
	call_deferred("live_run")
func live_run() -> void:
	var endpoint := OS.get_environment("BG_ENDPOINT")
	if not endpoint.begins_with("127.0.0.1:") or not endpoint.get_slice(":", 1) in ["5710", "5711", "5712"]:
		fail("Battleground proof requires private UDP5710–5712")
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
		var finish := false
		match command.action:
			"snapshot":
				var hovered := root.gui_get_hovered_control()
				result.pointer_hover = str(hovered.get_path()) if hovered != null else ""
				result.client_processing = client.is_processing()
				result.gui_input_disabled = root.gui_disable_input
				result.mouse_mode = Input.mouse_mode
				result.window_visible = root.visible
				result.startup_args = OS.get_cmdline_user_args()
				result.account = client.account_state()
				result.controls = []
				for node in root.find_children("*", "Control", true, false):
					if node.is_visible_in_tree():
						var rect: Rect2 = node.get_global_rect()
						result.controls.append({"name": str(node.name), "rect": [rect.position.x, rect.position.y, rect.size.x, rect.size.y], "text": str(node.text) if node is Label or node is Button else ""})
			"visuals":
				result.visuals = read_local_player_visuals()
			"units":
				result.units = []
				var units := root.find_child("WorldUnits", true, false)
				if units != null:
					for unit in units.get_children():
						if unit is Node3D and unit.has_meta("unit_name"):
							var position: Vector3 = unit.global_position
							var model := unit.find_child("NpcModel", true, false) as Node3D
							var model_path := str(model.get_meta("m2_source_path", "")) if model != null else ""
							var model_visible := model != null and model.is_visible_in_tree()
							result.units.append({"name": str(unit.get_meta("unit_name")), "path": str(unit.get_path()), "position": [position.x, position.y, position.z], "model_path": model_path, "model_visible": model_visible})
			"objects":
				result.objects = []
				for object in root.find_children("Mailbox_*", "Node3D", true, false):
					if object is Node3D and object.has_meta("game_object_entry"):
						var position: Vector3 = object.global_position
						var model := object.find_child("GameObjectModel", true, false) as Node3D
						var model_path := str(model.get_meta("m2_source_path", "")) if model != null else ""
						var model_visible := model != null and model.is_visible_in_tree()
						result.objects.append({"name": str(object.get_meta("game_object_name")), "entry": int(object.get_meta("game_object_entry")), "server_id": int(object.get_meta("game_object_server_id")), "position": [position.x, position.y, position.z], "model_path": model_path, "model_visible": model_visible})
			"transport-floor":
				result.ships = []
				for object in root.find_children("Mailbox_*", "Node3D", true, false):
					if not object.has_meta("game_object_entry"):
						continue
					var entry := int(object.get_meta("game_object_entry"))
					if not entry in [278407, 279254]:
						continue
					var model := object.get_node_or_null("GameObjectModel") as Node3D
					var body := object.get_node_or_null("GameObjectModel/M2Collision") as StaticBody3D
					var local := Vector3(-29.8623, 20.5534, -0.520232) if entry == 278407 else Vector3(0.875362, 34.1691, 8.07249)
					var feet: Vector3 = object.global_position + Basis(Vector3.UP, object.rotation.y - PI / 2.0) * local
					var query := PhysicsRayQueryParameters3D.create(feet + Vector3.UP * 1.6, feet - Vector3.UP * 50.0, 1 << 5)
					var hit: Dictionary = object.get_world_3d().direct_space_state.intersect_ray(query)
					result.ships.append({"entry": entry, "position": str(object.global_position), "yaw": object.rotation.y, "model_transform": str(model.global_transform) if model != null else "absent", "collision_transform": str(body.global_transform) if body != null else "absent", "layer": body.collision_layer if body != null else -1, "probe_feet": str(feet), "hit_position": str(hit.get("position", "absent")), "hit_normal": str(hit.get("normal", "absent")), "collider": str(hit.collider.get_path()) if hit.has("collider") else "absent"})
			"capture":
				await frames(3)
				await RenderingServer.frame_post_draw
				result.error = root.get_texture().get_image().save_png(command.path)
			"click", "right-click":
				var target = root.find_child(command.name, true, false) as Control
				if target == null or not target.is_visible_in_tree():
					result.error = "Missing visible native control: " + str(command.name)
				else:
					result.target = str(target.get_path())
					result.mouse_filter = target.mouse_filter
					result.target_viewport = str(target.get_viewport().get_path())
					var button := MOUSE_BUTTON_RIGHT if command.action == "right-click" else MOUSE_BUTTON_LEFT
					await pointer(target.get_global_rect().get_center(), button)
					result.input_handled = root.is_input_handled()
			"key":
				await tap(int(command.key))
			"quit":
				client.free()
				finish = true
		await frames(3)
		var response := FileAccess.open(directory.path_join("response.json"), FileAccess.WRITE)
		response.store_string(JSON.stringify(result))
		response.close()
		if finish:
			quit(0)
			return

func read_local_player_visuals() -> Dictionary:
	var camera := root.get_camera_3d()
	var result := {"models": []}
	if camera != null:
		result.camera_position = str(camera.global_position)
		result.camera_transform = str(camera.global_transform)
	var player := root.find_child("Bgcovera", true, false) as Node3D
	if player == null:
		result.error = "Local character Bgcovera node absent"
		return result
	result.player_position = str(player.global_position)
	for node in player.find_children("*", "MeshInstance3D", true, false):
		var mesh := node as MeshInstance3D
		result.models.append({"path": str(mesh.get_path()),
			"position": str(mesh.global_position), "scale": str(mesh.global_basis.get_scale()),
			"world_bounds": str(mesh.global_transform * mesh.get_aabb()),
			"visible": mesh.is_visible_in_tree()})
	return result

func pointer(point: Vector2, button: MouseButton) -> void:
	# A private headless compositor has no physical pointer-enter event.
	# Viewport requires this notification before synthetic mouse input reaches GUI controls.
	root.notify_mouse_entered()
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
		event.button_mask = (1 << (button - 1)) if pressed else 0
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await frames(3)

func tap(key: Key) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = key
		event.physical_keycode = key
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await frames(3)

func frames(count: int) -> void:
	for _frame in range(count):
		await process_frame

func fail(message: String) -> void:
	push_error("FIXTURE BATTLEGROUND: " + message)
	quit(1)
