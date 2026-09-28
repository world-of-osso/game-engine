extends SceneTree

# Retail nameplate visibility in world against the dev server (docs/specs/nameplate-style.md):
# with the default CVars no plate shows for untargeted units out of combat with the player;
# Tab-targeting nearby NPCs shows a plate for an enemy target and only for it, never for a
# friendly one; a targeted enemy whose pick-box centre is hidden from the camera by terrain or
# WMO collision (independent GDScript ray, layers 1-2) has alpha 0.4, a clear one 1.0; Escape
# clears the target and its plate. The fixture turns the character in place (Right arrow) to
# bring units on screen and to find a unit behind cover. Account fb_worldmap / Fbworldmap
# stands in Northshire. Combat cannot be started from this client (no attack or spell input,
# nearby creatures are neutral), so combat-driven plates are covered by unit tests.

const ACCOUNT := "fb_worldmap"
const PASSWORD := "fbtest"
const CHARACTER := "Fbworldmap"
const WORLD_WAIT_MS := 120000
const SETTLE_MS := 8000
const MAX_TABS := 24
const SHOTS := "/tmp/claude/nameplates/"

var client: Node

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	DirAccess.make_dir_recursive_absolute(SHOTS)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if server != "127.0.0.1:5000":
		fail("GODOT_TEST_SERVER must explicitly select 127.0.0.1:5000")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, ACCOUNT, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world():
		return
	await wait_ms(SETTLE_MS)

	var untargeted: Dictionary = client.nameplate_state()
	print("FIXTURE UNTARGETED PLATES ", untargeted)
	if not untargeted.is_empty():
		fail("Plates without a target or combat: " + str(untargeted))
		return
	await capture("untargeted.png")

	var seen := {}
	var enemy_plates := 0
	var occluded_plates := 0
	var friendly_targets := 0
	for tab in range(MAX_TABS):
		await tap(KEY_TAB)
		await wait_frames(3)
		var target = client.target_state().target
		if target == null or seen.has(target):
			break
		seen[target] = true
		var name: String = client.target_state().target_name
		var rules: Dictionary = client.nameplate_rules(target)
		print("FIXTURE RULES %s %s" % [name, rules])
		# Default CVars: a living, selectable target within 60 yd has a plate exactly when
		# the player can attack it; friendly units never do.
		var expect_shown: bool = rules.enemy and rules.alive and rules.selectable and rules.distance <= 60.0
		if rules.shown != expect_shown or (rules.reaction == "Friendly" and rules.enemy):
			fail("%s: rules %s" % [name, rules])
			return
		if not rules.shown:
			friendly_targets += 1
			if not await expect_only_plate(null, name):
				return
			continue
		if not await turn_until_plate(target):
			fail("%s: shown but no plate after a full turn: %s" % [name, client.nameplate_state()])
			return
		var result: int = await check_target_plate(target, name)
		if result < 0:
			return
		enemy_plates += 1
		if result == 0 and enemy_plates == 1:
			await capture("target-plate.png")
			print("FIXTURE PLATE RECTS ", client.nameplate_state()[target])
		occluded_plates += result
	print("FIXTURE SUMMARY targets %d enemy_plates %d occluded %d friendly_or_none %d" % [seen.size(), enemy_plates, occluded_plates, friendly_targets])
	if enemy_plates == 0:
		fail("No Tab target showed a plate")
		return
	# A unit behind terrain or a WMO wall: turn until one is on screen, then target it.
	# Units move, so a unit found behind cover may leave it before its plate is read.
	var attempts := 0
	while occluded_plates == 0 and attempts < 4:
		attempts += 1
		var hidden = await find_occluded_enemy()
		if hidden == null:
			continue
		if not await tab_to(hidden):
			return
		var hidden_name: String = client.target_state().target_name
		var result: int = await check_target_plate(hidden, hidden_name)
		if result < 0:
			return
		occluded_plates += result
	if occluded_plates == 0:
		fail("No targeted enemy stayed behind terrain or WMO collision in %d searches" % attempts)
		return
	await tap(KEY_ESCAPE)
	await wait_frames(3)
	if not client.nameplate_state().is_empty():
		fail("Plate remains after clearing the target: " + str(client.nameplate_state()))
		return
	print("FIXTURE WORLD_NAMEPLATE_DONE enemy_plates=%d occluded=%d" % [enemy_plates, occluded_plates])
	client.free()
	quit(0)

# The target's plate against an independent occlusion ray: -1 failed, 0 clear, 1 occluded.
func check_target_plate(target, name: String) -> int:
	if not await expect_only_plate(target, name):
		return -1
	var plate: Dictionary = client.nameplate_state()[target]
	var blocked := independently_occluded(target)
	print("FIXTURE TARGET %s (%s) plate alpha %s occluded %s ray %s" % [name, target, plate.alpha, plate.occluded, blocked])
	var expected := 0.4 if blocked else 1.0
	if not is_equal_approx(plate.alpha, expected) or plate.occluded != blocked:
		fail("%s: alpha %s occluded %s, independent ray says %s" % [name, plate.alpha, plate.occluded, blocked])
		return -1
	if plate.name != name:
		fail("Plate name '%s' for target '%s'" % [plate.name, name])
		return -1
	if blocked:
		await capture("occluded-target.png")
		print("FIXTURE OCCLUDED PLATE ", plate)
	return 1 if blocked else 0

# An enemy within 60 yd whose pick-box centre is in view but behind world collision.
func find_occluded_enemy():
	for heading in range(36):
		for unit in client.get_node("WorldUnits").get_children():
			for area in unit.find_children("UnitPick", "Area3D", true, false):
				var id = area.get_meta("unit_server_id")
				var rules: Dictionary = client.nameplate_rules(id)
				if rules.is_empty() or not rules.enemy or not rules.alive or rules.distance > 60.0:
					continue
				var center: Vector3 = (area.get_child(0) as Node3D).global_position
				if camera().is_position_in_frustum(center) and independently_occluded(id):
					print("FIXTURE HIDDEN ENEMY ", id, " ", rules)
					return id
		push_key(KEY_RIGHT, true)
		await wait_frames(6)
		push_key(KEY_RIGHT, false)
		await wait_frames(2)
	return null

# Tab from the nearest NPC outwards until `id` is the target.
func tab_to(id) -> bool:
	for tab in range(120):
		if client.target_state().target == id:
			return true
		await tap(KEY_TAB)
		await wait_frames(2)
	fail("Tab never reached unit %s" % id)
	return false

# No plate other than `target`'s (none when null).
func expect_only_plate(target, name: String) -> bool:
	var plates: Dictionary = client.nameplate_state()
	for id in plates:
		if id != target:
			fail("Target %s: plate on another unit %s: %s" % [name, id, plates])
			return false
	return true

# Turn the character in place until the target's plate is on screen.
func turn_until_plate(target) -> bool:
	for step in range(40):
		if client.nameplate_state().has(target):
			return true
		push_key(KEY_RIGHT, true)
		await wait_frames(6)
		push_key(KEY_RIGHT, false)
		await wait_frames(2)
	return client.nameplate_state().has(target)

func push_key(code: Key, pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.pressed = pressed
	root.push_input(event, true)

# The unit's pick-box centre hidden from the camera by terrain or WMO collision.
func independently_occluded(id) -> bool:
	for unit in client.get_node("WorldUnits").get_children():
		for area in unit.find_children("UnitPick", "Area3D", true, false):
			if area.get_meta("unit_server_id") != id:
				continue
			var center: Vector3 = (area.get_child(0) as Node3D).global_position
			var query := PhysicsRayQueryParameters3D.create(camera().global_position, center, 3)
			return not camera().get_world_3d().direct_space_state.intersect_ray(query).is_empty()
	fail("No pick shape for %s" % id)
	return false

func enter_world() -> bool:
	var deadline := Time.get_ticks_msec() + 15000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.reply_received and state.screen == "CharacterSelect" and state.character_count >= 1:
			break
	var ui = client.get_node_or_null("CharacterSelectUI")
	if ui == null:
		fail("No character select: " + str(client.account_state()))
		return false
	var card := roster_card(ui)
	if card == null:
		fail("Roster has no " + CHARACTER)
		return false
	await click_control(card)
	await click_control(ui.find_child("EnterWorld", true, false))
	deadline = Time.get_ticks_msec() + WORLD_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "InWorld" and state.local_player_position != null:
			print("FIXTURE IN_WORLD ", state.selected_character_name, " at ", state.local_player_position)
			return true
	fail("Timed out entering the world: " + str(client.account_state()))
	return false

func roster_card(ui: Node) -> Control:
	var index := 0
	while true:
		var card = ui.find_child("CharCard_%d" % index, true, false)
		if not card is Control:
			return null
		for label in card.find_children("*", "Label", true, false):
			if label.text == CHARACTER:
				return card
		index += 1
	return null

func camera() -> Camera3D:
	return root.get_viewport().get_camera_3d()

func click_control(control: Control) -> void:
	var point := control.get_global_rect().get_center()
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
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await wait_frames(3)

func capture(file: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var error := image.save_png(SHOTS + file)
	if error != OK:
		fail("Could not save " + file + ": " + str(error))

func tap(code: Key) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = code
		event.physical_keycode = code
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame

func wait_frames(count: int) -> void:
	for frame in range(count):
		await process_frame

func wait_ms(duration: int) -> void:
	var deadline := Time.get_ticks_msec() + duration
	while Time.get_ticks_msec() < deadline:
		await process_frame

func fail(message: String) -> void:
	push_error(message)
	quit(1)
