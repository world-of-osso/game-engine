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
	enter.pressed.connect(func(): print("FIXTURE NAMEPLATE_ENTER_PRESSED"))
	var roster_deadline := Time.get_ticks_msec() + STARTUP_WAIT_MS
	while not enter.is_visible_in_tree() and Time.get_ticks_msec() < roster_deadline:
		await process_frame
	if not enter.is_visible_in_tree():
		fail("Character scene never revealed its roster controls")
		return
	print("FIXTURE NAMEPLATE_GUI disabled=", root.gui_disable_input, " viewport=", root.get_visible_rect(), " button_visible=", enter.is_visible_in_tree(), " filter=", enter.mouse_filter)
	print("FIXTURE NAMEPLATE_ROSTER card=", card.get_global_rect(), " enter=", enter.get_global_rect(), " disabled=", enter.disabled)
	await click_point(card.get_global_rect().get_center())
	await click_point(enter.get_global_rect().get_center())
	if not await wait_screen(client, "InWorld", WORLD_LOAD_WAIT_MS):
		return
	if not await wait_click_npc(client):
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
	if not await verify_controls(client, camera, id, self_id, qualified.point):
		return
	await capture_click("controls-owner-selected")
	print("FIXTURE NAMEPLATE_CLICK_DONE")
	client.free()
	quit(0)

func wait_click_npc(client: Node) -> bool:
	var deadline := Time.get_ticks_msec() + WORLD_LOAD_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var model := client.get_node_or_null("WorldUnits/" + NPC + "/NpcVisualRoot/NpcModel")
		if model == null:
			continue
		for node in model.find_children("*", "MeshInstance3D", true, false):
			var mesh := node as MeshInstance3D
			if mesh.mesh == null or not mesh.is_visible_in_tree():
				continue
			var material := mesh.get_active_material(0) as ShaderMaterial
			if material != null and material.get_shader_parameter("base_texture") is Texture2D:
				print("FIXTURE NAMEPLATE_AUTHORED_NPC_READY display=2079 mesh=", mesh.get_path())
				return true
	fail("Authored nameplate NPC never acquired a visible textured mesh: " + str(client.account_state()))
	return false

func verify_controls(client: Node, camera: Camera3D, id: int, self_id: int, plate_point: Vector2) -> bool:
	await tap(KEY_F1)
	if not await selection(client, self_id, PLAYER, "controls self"):
		return false
	print("FIXTURE NAMEPLATE_CLICK_HIDE")
	if not await wait_plate_visibility(client, id, false):
		return false
	if UnitPicker.pick(camera, plate_point) != null:
		fail("Hidden-plate control point no longer misses actual meshes")
		return false
	await click_point(plate_point)
	if not await selection(client, self_id, PLAYER, "hidden plate cannot target"):
		return false
	var mesh_point := pick_mesh_point(client, camera, id)
	if mesh_point.is_empty():
		fail("No qualified NPC mesh point while its plate is hidden")
		return false
	await click_point(mesh_point.point)
	if not await selection(client, id, NPC, "unchanged mesh routing"):
		return false
	await tap(KEY_F1)
	if not await selection(client, self_id, PLAYER, "not-selectable self"):
		return false
	print("FIXTURE NAMEPLATE_CLICK_NOT_SELECTABLE")
	if not await wait_plate_visibility(client, id, false, true):
		return false
	if UnitPicker.pick(camera, mesh_point.point) == id:
		fail("Not-selectable owner still has a native mesh hit")
		return false
	await click_point(plate_point)
	if not await selection(client, self_id, PLAYER, "not-selectable plate cannot target"):
		return false
	await click_point(mesh_point.point)
	if not await selection(client, self_id, PLAYER, "not-selectable mesh cannot target"):
		return false
	print("FIXTURE NAMEPLATE_CLICK_SHOW")
	if not await wait_plate_visibility(client, id, true):
		return false
	if not await verify_modal_guard(client, plate_point):
		return false
	await tap(KEY_F1)
	if not await selection(client, self_id, PLAYER, "overlap self"):
		return false
	if not await verify_overlap(client, camera, id):
		return false
	return await verify_empty_ground(client, camera, id)

func wait_plate_visibility(client: Node, id: int, shown: bool, not_selectable := false) -> bool:
	var deadline := Time.get_ticks_msec() + 5000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var rules: Dictionary = client.nameplate_rules(id)
		if client.nameplate_state().has(id) == shown and (not not_selectable or not rules.selectable):
			return true
	fail("Replicated plate visibility did not become %s: %s" % [shown, client.nameplate_rules(id)])
	return false

func pick_mesh_point(client: Node, camera: Camera3D, id: int) -> Dictionary:
	var npc := client.get_node("WorldUnits/" + NPC) as Node3D
	for height in [1.0, 0.5, 1.5, 2.0]:
		var point := camera.unproject_position(npc.global_position + Vector3(0.0, height, 0.0))
		if UnitPicker.pick(camera, point) == id:
			print("FIXTURE NAMEPLATE_MESH_CONTROL point=", point, " owner=", id)
			return {"point": point}
	return {}

func verify_modal_guard(client: Node, plate_point: Vector2) -> bool:
	await tap(KEY_ESCAPE)
	await tap(KEY_ESCAPE)
	var deadline := Time.get_ticks_msec() + 5000
	while client.get_node_or_null("GameMenuUI") == null and Time.get_ticks_msec() < deadline:
		await process_frame
	var menu := client.get_node_or_null("GameMenuUI")
	if not await cleared_selection(client):
		return false
	await move_pointer(plate_point)
	var hovered := root.gui_get_hovered_control()
	if menu == null or hovered == null or not menu.is_ancestor_of(hovered):
		fail("Modal control did not cover the qualified plate point with real registry UI")
		return false
	# Exercise the targeting press boundary without activating an unknown menu button.
	await push_button(plate_point, true)
	var state: Dictionary = client.target_state()
	if state.target != null or state.sent != null or state.server_target != null or state.circle_on != null:
		fail("Registry/modal click changed target: " + str(state))
		return false
	await move_pointer(Vector2(2.0, 2.0))
	await push_button(Vector2(2.0, 2.0), false)
	await tap(KEY_ESCAPE)
	print("FIXTURE NAMEPLATE_MODAL_CONTROL registry=", hovered.name, " target unchanged")
	return true

func cleared_selection(client: Node) -> bool:
	var deadline := Time.get_ticks_msec() + 5000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.target_state()
		if state.target == null and state.sent == null and state.server_target == null and state.circle_on == null:
			return true
	fail("Escape did not clear client, actual server echo and ring: " + str(client.target_state()))
	return false

func verify_overlap(client: Node, camera: Camera3D, id: int) -> bool:
	var npc := client.get_node("WorldUnits/" + NPC) as Node3D
	var far_position := npc.global_position + (npc.global_position - camera.global_position).normalized() * 0.1
	print("FIXTURE NAMEPLATE_CLICK_OVERLAP ", JSON.stringify([far_position.x, far_position.y, far_position.z]))
	var deadline := Time.get_ticks_msec() + 10000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var far := client.get_node_or_null("WorldUnits/Fixture Far Plate") as Node3D
		if far == null:
			continue
		var area := far.find_child("UnitPick", true, false) as Area3D
		if area == null:
			continue
		var far_id: int = area.get_meta("unit_server_id")
		var plates: Dictionary = client.nameplate_state()
		if not plates.has(id) or not plates.has(far_id):
			continue
		var near_distance := npc.global_position.distance_to(camera.global_position)
		var far_distance := far.global_position.distance_to(camera.global_position)
		if far_distance <= near_distance:
			continue
		var overlap: Rect2 = (plates[id].frame_rect as Rect2).intersection(plates[far_id].frame_rect)
		var qualified := overlap_mesh_miss(camera, overlap)
		if qualified.is_empty():
			continue
		print("FIXTURE NAMEPLATE_OVERLAP_QUALIFIED near=", id, " far=", far_id, " distances=", [near_distance, far_distance], " point=", qualified.point)
		await click_point(qualified.point)
		return await selection(client, id, NPC, "nearest overlapping plate owner")
	fail("No genuine overlapping shown plates with distinct camera distances and mesh miss")
	return false

func overlap_mesh_miss(camera: Camera3D, rect: Rect2) -> Dictionary:
	if not rect.has_area():
		return {}
	for x in [0.5, 0.25, 0.75]:
		for y in [0.5, 0.25, 0.75]:
			var point := rect.position + rect.size * Vector2(x, y)
			if root.get_visible_rect().has_point(point) and UnitPicker.pick(camera, point) == null:
				return {"point": point}
	return {}

func verify_empty_ground(client: Node, camera: Camera3D, id: int) -> bool:
	for x in [0.35, 0.5, 0.65]:
		for y in [0.7, 0.8, 0.6]:
			var point := Vector2(root.size) * Vector2(x, y)
			if UnitPicker.pick(camera, point) != null:
				continue
			var over_plate := false
			for plate in client.nameplate_state().values():
				if (plate.frame_rect as Rect2).has_point(point):
					over_plate = true
			if over_plate:
				continue
			await move_pointer(point)
			if root.gui_get_hovered_control() != null:
				continue
			print("FIXTURE NAMEPLATE_EMPTY_GROUND_QUALIFIED point=", point)
			await click_point(point)
			return await selection(client, id, NPC, "empty ground retains sticky target")
	fail("No empty world point outside all plate rectangles and registry UI")
	return false

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

func move_pointer(point: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await process_frame

func push_button(point: Vector2, pressed: bool) -> void:
	var event := InputEventMouseButton.new()
	event.position = point
	event.global_position = point
	event.button_index = MOUSE_BUTTON_LEFT
	event.button_mask = MOUSE_BUTTON_MASK_LEFT if pressed else 0
	event.pressed = pressed
	root.push_input(event, true)
	await process_frame

func click_point(point: Vector2) -> void:
	await move_pointer(point)
	print("FIXTURE NAMEPLATE_POINTER point=", point, " hovered=", root.gui_get_hovered_control(), " gui_disabled=", root.gui_disable_input)
	for pressed in [true, false]:
		await push_button(point, pressed)
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
