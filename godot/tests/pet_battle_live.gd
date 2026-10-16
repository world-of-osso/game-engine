extends SceneTree

const POSED_TRIANGLES := preload("res://tests/posed_triangle_points.gd")
var client: Node

## Owned private-server driver. Every battle decision uses actual native pointer input.
## PBWILD_DIR holds command/response JSON; PBWILD_ENDPOINT must be loopback, not5000.
## Startup -- --screen inworld --server <endpoint> --char <fb_ character>, with an
## isolated XDG_CONFIG_HOME containing fb_pbwild/fbtest credentials and chosen skin.
var directory := ""

func _initialize() -> void:
	Engine.max_fps = 30
	call_deferred("live_run")

func live_run() -> void:
	var endpoint := OS.get_environment("PBWILD_ENDPOINT")
	if not endpoint.begins_with("127.0.0.1:") or endpoint.ends_with(":5000"):
		fail("Wild battle proof requires its owned private UDP endpoint")
		return
	root.size = Vector2i(1920, 1080)
	directory = OS.get_environment("PBWILD_DIR")
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
				result.input_enabled = client.is_processing_input()
				result.unhandled_enabled = client.is_processing_unhandled_input()
				result.inventory = client.merchant_state()
				result.controls = []
				for node in root.find_children("*", "Control", true, false):
					if node.is_visible_in_tree():
						var rect = node.get_global_rect()
						result.controls.append({"name": str(node.name), "rect": [rect.position.x, rect.position.y, rect.size.x, rect.size.y], "text": str(node.text) if node is Label or node is Button or node is LineEdit else ""})
				result.units = []
				var units = client.get_node_or_null("WorldUnits")
				if units != null:
					for unit in units.get_children():
						result.units.append({"name": str(unit.name), "position": [unit.global_position.x, unit.global_position.y, unit.global_position.z], "meshes": unit.find_children("*", "MeshInstance3D", true, false).size()})
				var scene = root.find_child("PetBattleActiveModels", true, false)
				result.battle_meshes = scene.find_children("*", "MeshInstance3D", true, false).size() if scene != null else 0
				result.battle_models = []
				if scene != null:
					for model in scene.get_children():
						var bounds := AABB()
						var visible_meshes := 0
						var framed := true
						var model_viewport = scene.get_viewport()
						var model_camera = model_viewport.get_camera_3d()
						var view_rect: Rect2 = model_viewport.get_parent().get_global_rect()
						var projected := Rect2()
						var posed_projected := Rect2()
						for mesh in model.find_children("*", "MeshInstance3D", true, false):
							if mesh.is_visible_in_tree():
								var transformed: AABB = mesh.global_transform * mesh.get_aabb().merge(mesh.get_custom_aabb())
								for corner in range(8):
									var world_corner = transformed.get_endpoint(corner)
									var point = model_camera.unproject_position(world_corner)
									projected = Rect2(point, Vector2.ZERO) if visible_meshes == 0 and corner == 0 else projected.expand(point)
								var posed_mesh = mesh.bake_mesh_from_current_skeleton_pose() if mesh.get_skin() != null else mesh.mesh
								var posed: AABB = mesh.global_transform * posed_mesh.get_aabb()
								for corner in range(8):
									var posed_corner = posed.get_endpoint(corner)
									var point = model_camera.unproject_position(posed_corner)
									framed = framed and not model_camera.is_position_behind(posed_corner) and model_viewport.get_visible_rect().has_point(point)
									posed_projected = Rect2(point, Vector2.ZERO) if visible_meshes == 0 and corner == 0 else posed_projected.expand(point)
								bounds = transformed if visible_meshes == 0 else bounds.merge(transformed)
								visible_meshes += 1
						var origin = view_rect.position + model_camera.unproject_position(model.global_position)
						var visual = model.get_child(0)
						var instance_scale = visual.get_meta("pet_instance_scale")
						var display_id = visual.get_meta("creature_display_id")
						result.battle_models.append({"name": str(model.name), "visible_meshes": visible_meshes, "framed": framed, "bounds": [bounds.size.x, bounds.size.y, bounds.size.z], "origin_px": [origin.x, origin.y], "projected_px": [projected.position.x + view_rect.position.x, projected.position.y + view_rect.position.y, projected.size.x, projected.size.y], "posed_projected_px": [posed_projected.position.x + view_rect.position.x, posed_projected.position.y + view_rect.position.y, posed_projected.size.x, posed_projected.size.y], "position": [model.position.x, model.position.y, model.position.z], "scale": [model.scale.x, model.scale.y, model.scale.z], "display_id": display_id, "pet_instance_scale": instance_scale, "visual_scale": [visual.scale.x, visual.scale.y, visual.scale.z], "visual_position": [visual.position.x, visual.position.y, visual.position.z]})
			"capture":
				await frames(3)
				await RenderingServer.frame_post_draw
				var image = root.get_texture().get_image()
				result.size = [image.get_width(), image.get_height()]
				result.error = image.save_png(command.path)
			"camera":
				client.set_camera_orbit(float(command.yaw), float(command.pitch), float(command.distance))
			"action":
				var target: Control
				for host in client.get_children():
					if host.has_method("control_for_action"):
						target = host.control_for_action(command.name) as Control
						if target != null and target.is_visible_in_tree():
							break
				if target != null and target.is_visible_in_tree():
					await pointer(target.get_global_rect().get_center(), MOUSE_BUTTON_LEFT)
				else:
					result.error = "Missing visible action: " + str(command.name)
			"click", "right_click":
				var target = root.find_child(command.name, true, false) as Control
				if target == null or not target.is_visible_in_tree():
					result.error = "Missing visible control: " + str(command.name)
				else:
					await pointer(target.get_global_rect().get_center(), MOUSE_BUTTON_RIGHT if command.action == "right_click" else MOUSE_BUTTON_LEFT)
			"npc":
				var clicked := false
				var units = client.get_node_or_null("WorldUnits")
				var camera = root.get_camera_3d()
				if units != null and camera != null:
					for unit in units.get_children():
						if str(unit.name) != command.name:
							continue
						var area = unit.find_child("UnitPick", true, false) as Area3D
						if area == null:
							continue
						var picked = pet_surface_point(unit, area, camera)
						if not picked.is_empty():
							result.point = [picked.point.x, picked.point.y]
							result.picked_unit = UnitPicker.pick(camera, picked.point)
							await pointer(picked.point, MOUSE_BUTTON_RIGHT)
							var hovered = root.gui_get_hovered_control()
							result.hovered = str(hovered.get_path()) if hovered != null else ""
							clicked = true
							break
				if not clicked:
					result.error = "No pickable on-screen NPC: " + str(command.name)
			"key":
				await tap(int(command.key))
			"quit":
				client.free()
				quit(0)
		await frames(3)
		var response = FileAccess.open(directory.path_join("response.json"), FileAccess.WRITE)
		response.store_string(JSON.stringify(result))
		response.close()

func pet_surface_point(unit: Node, area: Area3D, camera: Camera3D) -> Dictionary:
	var model := unit.get_node_or_null("NpcVisualRoot/NpcModel")
	if model == null:
		return {}
	for node in model.find_children("*", "MeshInstance3D", true, false):
		var mesh := node as MeshInstance3D
		if mesh.mesh == null or not mesh.is_visible_in_tree():
			continue
		for surface in range(mesh.mesh.get_surface_count()):
			for center in POSED_TRIANGLES.centroids(mesh, surface):
				var point := camera.unproject_position(center)
				if camera.is_position_in_frustum(center) and root.get_visible_rect().has_point(point) and UnitPicker.pick(camera, point) == area.get_meta("unit_server_id"):
					return {"point": point}
	return {}

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
	push_error("FIXTURE PET_BATTLE: " + message)
	quit(1)
