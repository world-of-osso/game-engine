extends SceneTree

## Creature classification on the unit tooltip and TargetFrame against a private server.
## Environment:
##   GODOT_TEST_SERVER       server address (a private test server)
##   UNITRANK_ACCOUNT / UNITRANK_CHARACTER  account (password fbtest) and a character
##                           standing in sight of the creature
##   UNITRANK_CREATURE       creature name (default Timber, world.db 1132, rank 4 rare)
##   UNITRANK_LEVEL_LINE     its expected tooltip level line (default "Level 10 Rare")
##   UNITRANK_SHOTS          screenshot directory
## Hovers the creature in the world (tooltip level line), left-clicks it to target it, checks
## the TargetFrame's rare star, hovers the TargetFrame and captures both.

const PASSWORD := "fbtest"

var client: Node
var shots := "/tmp/claude/unitrank-live/"
var character := ""
var creature := "Timber"
var level_line := "Level 10 Rare"

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("UNITRANK_ACCOUNT")
	character = OS.get_environment("UNITRANK_CHARACTER")
	if server == "" or account == "" or character == "":
		fail("GODOT_TEST_SERVER, UNITRANK_ACCOUNT and UNITRANK_CHARACTER are required")
		return
	for pair in [["UNITRANK_SHOTS", "shots"], ["UNITRANK_CREATURE", "creature"], ["UNITRANK_LEVEL_LINE", "level_line"]]:
		if OS.get_environment(pair[0]) != "":
			set(pair[1], OS.get_environment(pair[0]))
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world():
		return
	if (await find_unit()).is_empty():
		return
	# The creature wanders: follow its current screen point while waiting.
	if not await track_until(func(): return tooltip().title == creature and has_line(tooltip(), level_line), false, "%s tooltip with %s" % [creature, level_line]):
		return
	print("FIXTURE WORLD_TOOLTIP ", tooltip())
	await capture("01-world-tooltip.png")
	if not await track_until(func(): return client.target_state().target_name == creature, true, "%s targeted" % creature):
		return
	var star := control("UnitFramesUI", "TargetBossIcon")
	var dragon := control("UnitFramesUI", "TargetBossPortraitFrameTexture")
	if star == null or dragon == null:
		fail("TargetFrame classification art missing")
		return
	print("FIXTURE TARGET_ART star=%s dragon=%s target=%s" % [star.is_visible_in_tree(), dragon.is_visible_in_tree(), client.target_state()])
	await hover(control("UnitFramesUI", "TargetFrame"))
	if not await wait_until(func(): return tooltip().visible and tooltip().title == creature, 3000, "TargetFrame tooltip"):
		return
	print("FIXTURE TARGET_FRAME_TOOLTIP ", tooltip())
	await capture("02-target-frame-tooltip.png")
	print("FIXTURE UNITRANK_LIVE_DONE")
	client.free()
	quit(0)

## Hover (or left-click) the creature where it is now until `done`, for 10 s.
func track_until(done: Callable, click: bool, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + 10000
	while Time.get_ticks_msec() < deadline:
		var point = unit_point()
		if point != null:
			if click:
				await click_point(point, MOUSE_BUTTON_LEFT)
			else:
				await move_mouse(point)
		else:
			await process_frame
		if done.call():
			return true
	fail("Timed out waiting for %s: tooltip=%s target=%s" % [what, tooltip(), client.target_state()])
	return false

## The creature's pick point on screen when the native ray selects it, else null.
func unit_point():
	var units = client.get_node_or_null("WorldUnits")
	if units == null:
		return null
	for unit in units.get_children():
		if str(unit.name) != creature:
			continue
		var area := unit.find_child("UnitPick", true, false) as Area3D
		if area == null:
			continue
		var world_point := (area.get_child(0) as Node3D).global_position
		if not camera().is_position_in_frustum(world_point):
			continue
		var point := camera().unproject_position(world_point)
		if UnitPicker.pick(camera(), point) == area.get_meta("unit_server_id"):
			return point
	return null

## The creature's pick shape centre on screen, selected by the native ray; turn until seen.
func find_unit() -> Dictionary:
	var deadline := Time.get_ticks_msec() + 60000
	var turned := 0
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var point = unit_point()
		if point != null:
			return {"point": point}
		if turned < 60:
			push_key(KEY_RIGHT, true)
			await wait_frames(3)
			push_key(KEY_RIGHT, false)
			turned += 1
	var names := []
	var units = client.get_node_or_null("WorldUnits")
	if units != null:
		for unit in units.get_children():
			names.append(str(unit.name))
	fail("%s is not visible and unoccluded; units %s" % [creature, names])
	return {}

func tooltip() -> Dictionary:
	return client.tooltip_state()

func has_line(state: Dictionary, text: String) -> bool:
	for line in state.lines:
		if str(line).contains(text):
			return true
	return false

func control(host: String, name: String) -> Control:
	var ui := client.get_node_or_null(host)
	return ui.find_child(name, true, false) as Control if ui != null else null

func camera() -> Camera3D:
	return root.get_viewport().get_camera_3d()

func enter_world() -> bool:
	var deadline := Time.get_ticks_msec() + 60000
	var ui = null
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		ui = client.get_node_or_null("CharacterSelectUI")
		if state.reply_received and state.screen == "CharacterSelect" and state.character_count >= 1 and ui != null:
			break
	if ui == null:
		fail("No character select: " + str(client.account_state()))
		return false
	await click(ui.find_child("CharCard_0", true, false))
	var selected = ui.find_child("CharSelectCharacterName", true, false)
	if selected.text != character:
		fail("Card 0 is %s, not %s" % [selected.text, character])
		return false
	await click(ui.find_child("EnterWorld", true, false))
	deadline = Time.get_ticks_msec() + 120000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "InWorld" and state.local_player_position != null and state.local_server_position != null and state.terrain.pending_count == 0 and not state.terrain.parsed_tiles.is_empty():
			print("FIXTURE IN_WORLD at ", state.local_player_position)
			await wait_frames(60)
			return true
	fail("Timed out entering the world: " + str(client.account_state()))
	return false

func wait_until(predicate: Callable, timeout_ms: int, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call():
			return true
	fail("Timed out waiting for %s: tooltip=%s" % [what, tooltip()])
	return false

func push_key(code: Key, pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.pressed = pressed
	root.push_input(event, true)

func move_mouse(point: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	Input.warp_mouse(point)
	await wait_frames(3)

func hover(target: Control) -> void:
	if target == null:
		fail("Missing control to hover")
		return
	await move_mouse(target.get_global_rect().get_center())

func click(target: Control) -> void:
	await click_point(target.get_global_rect().get_center(), MOUSE_BUTTON_LEFT)

func click_point(point: Vector2, button: MouseButton) -> void:
	await move_mouse(point)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = button
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await wait_frames(3)

func capture(file: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var error := image.save_png(shots + file)
	if error != OK:
		fail("Could not save " + file + ": " + str(error))

func wait_frames(count: int) -> void:
	for frame in range(count):
		await process_frame

func fail(message: String) -> void:
	push_error(message)
	quit(1)
