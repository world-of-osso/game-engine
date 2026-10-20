extends "res://tests/world_nameplate_options_flow.gd"

# RED/GREEN uses the actual client headlessly. Rendered runs follow the gate.
func run_test() -> void:
	Engine.max_fps = 60
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if not server.begins_with("127.0.0.1:") or server.ends_with(":0") or not prepare_assets():
		fail("Nameplate click fixture needs its isolated assets and loopback server")
		return
	var client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error: String = client.connect_account(server, "fixture", "fixture", false)
	if error != "" or not await wait_screen(client, "CharacterSelect", STARTUP_WAIT_MS):
		fail("Click fixture authentication: " + error)
		return
	var ui = client.get_node("CharacterSelectUI")
	var card := ui.find_child("CharCard_0", true, false) as Control
	var enter := ui.find_child("EnterWorld", true, false) as Button
	for frame in range(5):
		await process_frame
	print("FIXTURE NAMEPLATE_ROSTER card=", card.get_global_rect(), " enter=", enter.get_global_rect(), " disabled=", enter.disabled)
	await click_point(card.get_global_rect().get_center())
	await click_point(enter.get_global_rect().get_center())
	if not await wait_screen(client, "InWorld", WORLD_LOAD_WAIT_MS):
		return
	if not await wait_visual(client, 1.5, WORLD_LOAD_WAIT_MS):
		return
	var npc := client.get_node("WorldUnits/" + NPC) as Node3D
	var pick := npc.find_child("UnitPick", true, false) as Area3D
	if pick == null:
		fail("Fixture NPC has no native pick ID")
		return
	var id: int = pick.get_meta("unit_server_id")
	await tap(KEY_F1)
	var self_id = client.target_state().target
	if self_id == null or self_id == id or not await selection(client, self_id, PLAYER, "initial self"):
		fail("RED needs a different existing target, not the desired NPC")
		return
	if not await turn_until_plate(client, id):
		return
	var rules: Dictionary = client.nameplate_rules(id)
	if rules.targeted or not rules.in_combat_with_player or not rules.shown:
		fail("Untargeted combat-visible prerequisite: " + str(rules))
		return
	if client.nameplate_state().has(self_id):
		fail("Local player unexpectedly has a projected plate")
		return
	var camera := root.get_camera_3d()
	var qualified := plate_mesh_miss(client, camera, id)
	if qualified.is_empty():
		fail("No shown NPC plate point separate from its actual mesh")
		return
	print("FIXTURE NAMEPLATE_CLICK_QUALIFIED owner=", id, " current=", self_id,
		" point=", qualified.point, " mesh=", qualified.mesh, " rules=", rules)
	await click_point(qualified.point)
	if not await selection(client, id, NPC, "visible untargeted plate with mesh miss"):
		return
	await capture_click("owner-selected")
	print("FIXTURE NAMEPLATE_CLICK_DONE")
	client.free()
	quit(0)

func plate_mesh_miss(client: Node, camera: Camera3D, id: int) -> Dictionary:
	var plate: Dictionary = client.nameplate_state()[id]
	var rect: Rect2 = plate.frame_rect
	for x in [0.5, 0.25, 0.75]:
		for y in [0.5, 0.25, 0.75]:
			var point := rect.position + rect.size * Vector2(x, y)
			if not root.get_visible_rect().has_point(point):
				continue
			var mesh = UnitPicker.pick(camera, point)
			if mesh != id:
				return {"point": point, "mesh": mesh}
	return {}

func click_point(point: Vector2) -> void:
	# Like battleground_live.gd: headless has no OS pointer-enter notification.
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
		event.button_index = MOUSE_BUTTON_LEFT
		event.button_mask = MOUSE_BUTTON_MASK_LEFT if pressed else 0
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	for frame in range(3):
		await process_frame

func selection(client: Node, id, name: String, phase: String) -> bool:
	var deadline := Time.get_ticks_msec() + 5000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.target_state()
		if state.target != id or state.server_target != id or state.circle_on != id:
			continue
		var ui := client.get_node_or_null("UnitFramesUI")
		var label = ui.find_child("TargetName", true, false) if ui != null else null
		if state.target_name == name and label is Label and label.text == name and label.is_visible_in_tree():
			print("FIXTURE NAMEPLATE_CLICK_SELECTION ", phase, " ", state)
			return true
	fail("%s: expected owner%s/%s and actual wire echo, got %s" % [phase, id, name, client.target_state()])
	return false

func capture_click(phase: String) -> void:
	if DisplayServer.get_name() == "headless":
		return
	await RenderingServer.frame_post_draw
	var output := OS.get_environment("GODOT_NAMEPLATE_CLICK_PROOF")
	var skin := OS.get_environment("GODOT_NAMEPLATE_CLICK_SKIN")
	var path := output.path_join("%s-%s.png" % [skin.to_lower(), phase])
	var error := root.get_texture().get_image().save_png(path)
	if error != OK:
		fail("Nameplate click capture failed: " + str(error))
