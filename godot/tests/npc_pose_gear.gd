extends SceneTree

## Live dev-server proof that Stockade creatures render their replicated pose and gear
## (map 34, character placed between the riflemen and the guard line, facing +x):
## - a Stockade Guard with emote state 333 plays Ready1H 26, wears its display's gloves,
##   boots and tabard (body geosets 402/502/1202 shown, 401/501/1201 hidden) and holds
##   its drawn sword on attachment 1 and shield on attachment 0;
## - a Stockade Rifleman plays ReadyRifle 48 with its rifle on attachment 1;
## - Petty Criminals play Sleep 100 and Sit 97.
## Screenshots of the guards and criminals go to `SHOT_DIR`. Account and character (card 0)
## come from NPC_POSE_ACCOUNT, NPC_POSE_PASSWORD, NPC_POSE_CHARACTER; place the character
## while offline, e.g. `game-server-admin set-position <name> 70 0 -25.3` inside the Stockade.

const SHOT_DIR := "res://../data/diagnostics/npcposes"
const STOCKADE := "stormwindjail"
const WAIT_UNITS_MS := 30000
const YAW_TOLERANCE := 0.08

var NAME := OS.get_environment("NPC_POSE_CHARACTER")
var turn_sign := 0.0
var failures: Array[String] = []

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if server != "127.0.0.1:5000":
		fail("GODOT_TEST_SERVER must explicitly select 127.0.0.1:5000")
		return
	var account := OS.get_environment("NPC_POSE_ACCOUNT")
	var password := OS.get_environment("NPC_POSE_PASSWORD")
	if account == "" or password == "" or NAME == "":
		fail("NPC_POSE_ACCOUNT, NPC_POSE_PASSWORD and NPC_POSE_CHARACTER are required")
		return
	DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path(SHOT_DIR))
	var client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, password, false)
	if error != "":
		fail("connect: " + error)
		return
	if not await enter_world(client):
		return
	var units := client.get_node("WorldUnits") as Node3D
	var player := units.get_node(NAME) as Node3D
	if not await wait_for_units(units):
		return
	check_guard(units)
	check_rifleman(units)
	check_criminals(units)
	await capture(player, units, "Stockade Guard", 26, "guards")
	await capture(player, units, "Petty Criminal", 100, "criminals")
	if failures.is_empty():
		print("PASS: Stockade guards, riflemen and criminals show their pose and gear")
		quit(0)
	else:
		for failure in failures:
			push_error(failure)
		quit(1)

func enter_world(client: Node) -> bool:
	if not await wait_until(client, func(state): return state.screen == "CharacterSelect" and state.reply_received, 20000, "CharacterSelect"):
		return false
	var ui = client.get_node("CharacterSelectUI")
	await click_control(ui.find_child("CharCard_0", true, false))
	await process_frame
	var selected = ui.find_child("CharSelectCharacterName", true, false)
	if selected.text != NAME:
		fail("Card 0 is %s, not %s" % [selected.text, NAME])
		return false
	await click_control(ui.find_child("EnterWorld", true, false))
	var ready := func(state):
		return state.screen == "InWorld" and state.selected_character_name == NAME \
			and state.terrain.map == STOCKADE and state.local_player_position != null \
			and state.terrain.pending_count == 0
	if not await wait_until(client, ready, 120000, "InWorld in the Stockade"):
		return false
	for _frame in 90:
		await process_frame
	print("TRACE entered: ", client.account_state().local_player_position)
	return true

## Units named `name` whose creature model has loaded.
func models_named(units: Node3D, name: String) -> Array[Node3D]:
	var found: Array[Node3D] = []
	for child in units.get_children():
		# Godot renames duplicate siblings; the unit keeps its replicated name as metadata.
		if child.get_meta("unit_name", "") != name:
			continue
		var model := child.find_child("NpcModel", true, false) as Node3D
		if model != null:
			found.append(model)
	return found

func animation_id(model: Node3D) -> int:
	var animation := model.get_node_or_null("M2Animation") as WowAnimationPlayer
	return animation.current_animation_id() if animation != null else -1

func wait_for_units(units: Node3D) -> bool:
	var deadline := Time.get_ticks_msec() + WAIT_UNITS_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if models_named(units, "Stockade Guard").size() >= 4 and models_named(units, "Petty Criminal").size() >= 4 \
				and models_named(units, "Stockade Rifleman").size() >= 2:
			for _frame in 30:
				await process_frame
			return true
	var names := []
	for child in units.get_children():
		names.append([child.get_meta("unit_name", ""), child.find_child("NpcModel", true, false) != null])
	fail("Stockade guards, riflemen and criminals did not load within %d ms: %s" % [WAIT_UNITS_MS, names])
	return false

## Visible state of the body batches of each mesh part, from the batch metadata.
func body_parts(model: Node3D) -> Dictionary:
	var parts := {}
	for child in model.get_children():
		var mesh := child as MeshInstance3D
		if mesh == null or not mesh.has_meta("m2_mesh_part"):
			continue
		var part: int = mesh.get_meta("m2_mesh_part")
		parts[part] = parts.get(part, false) or mesh.visible
	return parts

func unit_label(model: Node3D) -> String:
	var unit := model.get_parent().get_parent()
	return "%s %s" % [unit.get_meta("unit_name", ""), unit.global_position]

func item_attachment(model: Node3D, slot: String) -> String:
	var item := model.find_child("Equipment" + slot, true, false) as Node3D
	if item == null:
		return "none"
	if not item.visible:
		return "hidden"
	return String(item.get_parent().name)

func check_guard(units: Node3D) -> void:
	var ready: Array[Node3D] = []
	var ids := []
	for model in models_named(units, "Stockade Guard"):
		ids.append(animation_id(model))
		if animation_id(model) == 26:
			ready.append(model)
	print("TRACE guard animations ", ids)
	if ready.is_empty():
		failures.append("No Stockade Guard plays Ready1H 26: %s" % [ids])
		return
	for model in ready:
		var parts := body_parts(model)
		var main := item_attachment(model, "MainHand")
		var off := item_attachment(model, "OffHand")
		print("TRACE guard %s parts 401=%s 402=%s 501=%s 502=%s 1201=%s 1202=%s main=%s off=%s" % [
			unit_label(model), parts.get(401), parts.get(402), parts.get(501),
			parts.get(502), parts.get(1201), parts.get(1202), main, off])
		for part in [402, 502, 1202]:
			if parts.get(part) != true:
				failures.append("Guard %s armor geoset %d not shown: %s" % [unit_label(model), part, parts])
		for part in [401, 501, 1201]:
			if parts.get(part) == true:
				failures.append("Guard %s bare geoset %d still shown" % [unit_label(model), part])
		if main != "Attachment1" or off != "Attachment0":
			failures.append("Guard %s sword on %s, shield on %s; want Attachment1 / Attachment0" % [unit_label(model), main, off])

func check_rifleman(units: Node3D) -> void:
	var ids := []
	var placed := []
	for model in models_named(units, "Stockade Rifleman"):
		ids.append(animation_id(model))
		placed.append([item_attachment(model, "MainHand"), item_attachment(model, "Ranged")])
	print("TRACE rifleman animations ", ids, " rifle ", placed)
	if not ids.has(48):
		failures.append("No Stockade Rifleman plays ReadyRifle 48: %s" % [ids])
	if not placed.has(["Attachment1", "hidden"]):
		failures.append("No rifleman holds its rifle on Attachment1 with the ranged copy hidden: %s" % [placed])

func check_criminals(units: Node3D) -> void:
	var ids := []
	for model in models_named(units, "Petty Criminal"):
		ids.append(animation_id(model))
	print("TRACE criminal animations ", ids)
	for pose in [100, 97]:
		if not ids.has(pose):
			failures.append("No Petty Criminal plays %d: %s" % [pose, ids])

## Photograph the `name` unit playing `anim` nearest the character from four sides, with a
## test camera 4 yd away (the follow camera stays with the character, out of aggro range).
func capture(player: Node3D, units: Node3D, name: String, anim: int, label: String) -> void:
	var target: Node3D = null
	var best := INF
	for model in models_named(units, name):
		var distance := model.global_position.distance_to(player.global_position)
		if animation_id(model) == anim and distance < best:
			target = model
			best = distance
	if target == null:
		failures.append("No %s playing %d to photograph" % [name, anim])
		return
	# The client makes its follow camera current every frame; a sub-viewport over the same
	# world has a camera of its own.
	var view := SubViewport.new()
	view.size = Vector2i(1280, 720)
	view.world_3d = root.world_3d
	view.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	var camera := Camera3D.new()
	view.add_child(camera)
	root.add_child(view)
	camera.make_current()
	var focus := target.global_position + Vector3(0.0, 0.8, 0.0)
	for yaw in [0, 90, 180, 270]:
		var direction := Vector3(sin(deg_to_rad(yaw)), 0.0, cos(deg_to_rad(yaw)))
		camera.global_position = focus + direction * 4.0 + Vector3(0.0, 1.2, 0.0)
		camera.look_at(focus)
		for _frame in 20:
			await process_frame
		await RenderingServer.frame_post_draw
		var path := ProjectSettings.globalize_path("%s/npc-pose-gear-%s-yaw%d.png" % [SHOT_DIR, label, yaw])
		view.get_texture().get_image().save_png(path)
		print("TRACE screenshot %s: %s %s playing %d" % [path, name, unit_label(target), animation_id(target)])
	view.queue_free()

## Turn the character toward `target` with the arrow keys; whether it already faces it.
func face(player: Node3D, target: Node3D) -> bool:
	await process_frame
	var d := target.global_position - player.global_position
	var error := angle_difference(player.rotation.y + PI / 2.0, atan2(d.x, d.z))
	if turn_sign == 0.0:
		var before := player.rotation.y
		push_key(KEY_RIGHT, true)
		for _frame in 10:
			await process_frame
		push_key(KEY_RIGHT, false)
		await process_frame
		turn_sign = 1.0 if angle_difference(before, player.rotation.y) >= 0.0 else -1.0
		return false
	push_key(KEY_RIGHT, abs(error) > YAW_TOLERANCE and sign(error) == turn_sign)
	push_key(KEY_LEFT, abs(error) > YAW_TOLERANCE and sign(error) == -turn_sign)
	return abs(error) <= YAW_TOLERANCE

func release_turn() -> void:
	push_key(KEY_RIGHT, false)
	push_key(KEY_LEFT, false)

func wait_until(client: Node, predicate: Callable, timeout_ms: int, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call(client.account_state()):
			return true
	fail("Timed out waiting for %s: %s" % [what, client.account_state()])
	return false

func push_key(keycode: Key, pressed: bool) -> void:
	var key := InputEventKey.new()
	key.physical_keycode = keycode
	key.keycode = keycode
	key.pressed = pressed
	root.push_input(key, true)

func click_control(control: Control) -> void:
	var point := control.get_global_rect().get_center()
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await process_frame

func fail(message: String) -> void:
	push_error(message)
	quit(1)
