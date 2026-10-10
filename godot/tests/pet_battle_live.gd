extends "res://tests/world_auction_flow.gd"

## Owned private-server driver. Every battle decision uses actual native pointer input.
## PBWILD_DIR holds command/response JSON; PBWILD_ENDPOINT must be loopback, not5000.
## Startup -- --screen inworld --server <endpoint> --char <fb_ character>, with an
## isolated XDG_CONFIG_HOME containing fb_pbwild/fbtest credentials and chosen skin.
var directory := ""

class InputProbe extends Node:
	func _input(event: InputEvent) -> void:
		if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_RIGHT:
			print("PBWILD input pressed=", event.pressed, " point=", event.position)

	func _unhandled_input(event: InputEvent) -> void:
		if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_RIGHT:
			print("PBWILD unhandled pressed=", event.pressed, " point=", event.position)

func _initialize() -> void:
	Engine.max_fps = 30
	call_deferred("live_run")

func live_run() -> void:
	var endpoint := OS.get_environment("PBWILD_ENDPOINT")
	if not endpoint.begins_with("127.0.0.1:") or endpoint.ends_with(":5000"):
		fail("Wild battle proof requires its owned private UDP endpoint")
		return
	root.size = Vector2i(1280, 720)
	directory = OS.get_environment("PBWILD_DIR")
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	root.add_child(InputProbe.new())
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
						var picked = auctioneer_surface_point(unit, area, camera)
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
