extends SceneTree

# In-world unit selection against the dev server (docs/specs/godot-parity-matrix.md,
# Targeting row): a real left-click on a visible NPC selects it, the TargetFrame shows its
# name, the ring sits under it and the server echoes the target; a click on empty ground
# keeps the target; Escape clears it before a second Escape opens the game menu; Tab
# selects the nearest NPC. Account fb_worldmap / Fbworldmap stands in Northshire.

const ACCOUNT := "fb_worldmap"
const PASSWORD := "fbtest"
const CHARACTER := "Fbworldmap"
const WORLD_WAIT_MS := 120000
const NPC_WAIT_MS := 60000
const ECHO_WAIT_MS := 5000
const SHOTS := "/tmp/claude/"

var client: Node

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if server == "":
		fail("GODOT_TEST_SERVER must select the server")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, ACCOUNT, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world():
		return
	var npc: Dictionary = await find_visible_npc()
	if npc.is_empty():
		return
	print("FIXTURE NPC ", npc.name, " id ", npc.id, " at ", npc.point)

	await click(npc.point)
	if not await expect_target(npc.id, npc.name, "clicking the NPC"):
		return
	await capture("target-selected.png")

	var ground := ground_point()
	if ground == Vector2(-1, -1):
		fail("No empty-ground point on screen")
		return
	await click(ground)
	if not await expect_target(npc.id, npc.name, "clicking empty ground at %s" % ground):
		return

	await tap(KEY_ESCAPE)
	if not await expect_target(null, "", "Escape"):
		return
	if client.get_node_or_null("GameMenuUI") != null:
		fail("Escape clearing the target also opened the game menu")
		return
	await capture("target-cleared.png")
	await tap(KEY_ESCAPE)
	if client.get_node_or_null("GameMenuUI") == null:
		fail("Escape without a target did not open the game menu")
		return
	await tap(KEY_ESCAPE)

	await tap(KEY_TAB)
	var tabbed: Dictionary = client.target_state()
	if tabbed.target == null:
		fail("Tab selected no NPC: " + str(tabbed))
		return
	print("FIXTURE TAB ", tabbed.target_name, " id ", tabbed.target)
	tabbed = await tab_to_dummy(tabbed)
	if not await expect_target(tabbed.target, tabbed.target_name, "Tab"):
		return
	for size in [Vector2i(1280, 720), Vector2i(1920, 1080)]:
		if not await expect_target_frame_at_preset(size):
			return
		await capture("target-tab-%dx%d.png" % [size.x, size.y])
	print("FIXTURE WORLD_TARGET_DONE")
	client.free()
	quit(0)

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

# The character-select card whose name label reads CHARACTER.
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

# A replicated NPC whose pick shape centre is on screen and selected by the native ray.
func find_visible_npc() -> Dictionary:
	var deadline := Time.get_ticks_msec() + NPC_WAIT_MS
	var turned := 0
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var units := client.get_node_or_null("WorldUnits")
		if units == null or camera() == null:
			continue
		var best := {}
		for unit in units.get_children():
			var area := unit.find_child("UnitPick", true, false) as Area3D
			if area == null or unit.find_child("NpcModel", true, false) == null:
				continue
			# Godot suffixes duplicate sibling names; the frame shows the replicated name.
			if str(unit.name).right(1).is_valid_int():
				continue
			var shape := area.get_child(0) as Node3D
			var world_point := shape.global_position
			if not camera().is_position_in_frustum(world_point):
				continue
			var point := camera().unproject_position(world_point)
			var id = area.get_meta("unit_server_id")
			var distance := camera().global_position.distance_to(world_point)
			if UnitPicker.pick(camera(), point) == id and (best.is_empty() or distance < best.distance):
				best = {"id": id, "name": str(unit.name), "point": point, "distance": distance}
		if not best.is_empty():
			return best
		# Turn the character until an NPC is in view.
		if turned < 20:
			push_key(KEY_D, true)
			for frame in range(10):
				await process_frame
			push_key(KEY_D, false)
			turned += 1
	fail("No visible, unoccluded NPC in view")
	return {}

# A screen point below the player whose ray hits terrain and no unit.
func ground_point() -> Vector2:
	var space := camera().get_world_3d().direct_space_state
	for y in [680, 650, 620, 590]:
		for x in [200, 1080, 400, 880]:
			var point := Vector2(x, y)
			if UnitPicker.pick(camera(), point) != null:
				continue
			var origin := camera().project_ray_origin(point)
			var query := PhysicsRayQueryParameters3D.create(origin, origin + camera().project_ray_normal(point) * 200.0)
			var hit := space.intersect_ray(query)
			if not hit.is_empty():
				return point
	return Vector2(-1, -1)

func target_frame_name() -> String:
	var ui = client.get_node_or_null("UnitFramesUI")
	var label = ui.find_child("TargetName", true, false) if ui != null else null
	if not label is Label or not label.is_visible_in_tree():
		return ""
	return label.text

# Tab cycles NPCs nearest first; keep cycling to a training dummy when one is replicated.
func tab_to_dummy(state: Dictionary) -> Dictionary:
	var seen := {}
	while not "Dummy" in str(state.target_name) and not seen.has(state.target):
		seen[state.target] = state.target_name
		await tap(KEY_TAB)
		state = client.target_state()
	if not "Dummy" in str(state.target_name):
		print("FIXTURE NO_DUMMY among ", seen.values())
	print("FIXTURE TAB_TARGET ", state.target_name, " id ", state.target)
	return state

# Retail Modern preset: TargetFrame BOTTOMLEFT at UIParent BOTTOM (300, 250), in UI units of
# a 768-unit-tall screen; the 133x51 portrait-off art sits 19 right and 35 up of that point
# so its health slot is where the 232x100 Retail frame draws its health bar.
func expect_target_frame_at_preset(size: Vector2i) -> bool:
	# The headless output is 1280x720; larger viewports render at their content size.
	root.content_scale_mode = Window.CONTENT_SCALE_MODE_VIEWPORT
	root.content_scale_size = size
	for frame in range(5):
		await process_frame
	var ui = client.get_node_or_null("UnitFramesUI")
	var frame = ui.find_child("TargetFrame", true, false) if ui != null else null
	if not frame is Control or not frame.is_visible_in_tree():
		fail("No visible TargetFrame at %s" % size)
		return false
	var scale := size.y / 768.0
	var expected := Rect2(size.x / 2.0 + 319.0 * scale, size.y - (285.0 + 51.0) * scale, 133.0 * scale, 51.0 * scale)
	var actual: Rect2 = frame.get_global_rect()
	if actual.position.distance_to(expected.position) > 0.5 or actual.size.distance_to(expected.size) > 0.5:
		fail("TargetFrame at %s is %s, expected %s" % [size, actual, expected])
		return false
	print("FIXTURE TARGET_FRAME ", size, " ", actual)
	return true

func expect_target(id, name: String, action: String) -> bool:
	var state: Dictionary = client.target_state()
	if state.target != id or state.target_name != name:
		fail("%s: target %s, expected %s %s" % [action, state, id, name])
		return false
	if target_frame_name() != name:
		fail("%s: TargetFrame shows '%s', expected '%s'" % [action, target_frame_name(), name])
		return false
	if state.circle_on != id:
		fail("%s: target ring on %s, expected %s" % [action, state.circle_on, id])
		return false
	var deadline := Time.get_ticks_msec() + ECHO_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		state = client.target_state()
		if state.sent == id and state.server_target == id:
			print("FIXTURE ", action, " -> ", state)
			return true
		await process_frame
	fail("%s: server did not echo target %s: %s" % [action, id, state])
	return false

func click(point: Vector2) -> void:
	await hover_point(point)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	for frame in range(3):
		await process_frame

func hover_point(point: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await process_frame

func click_control(control: Control) -> void:
	await click(control.get_global_rect().get_center())

func capture(file: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var error := image.save_png(SHOTS + file)
	if error != OK:
		fail("Could not save " + file + ": " + str(error))

func tap(code: Key) -> void:
	push_key(code, true)
	await process_frame
	push_key(code, false)
	await process_frame
	await process_frame

func push_key(code: Key, pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.pressed = pressed
	root.push_input(event, true)

func fail(message: String) -> void:
	push_error(message)
	quit(1)
