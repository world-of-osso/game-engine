extends SceneTree

const NAME := "Input Fixture"
const WORLD_WAIT_MS := 90000
const MENU_WAIT_MS := 5000
const QUIET_FRAMES := 55

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if not server.begins_with("127.0.0.1:") or server.ends_with(":0"):
		fail("Menu fixture requires its owned loopback endpoint")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_screen(client, "CharacterSelect", 15000):
		return
	var scene := client.get_node_or_null("CharacterSelectScene")
	var ui := client.get_node_or_null("CharacterSelectUI")
	var card = ui.find_child("CharCard_0", true, false) if ui != null else null
	var tab = ui.find_child("CharSelectMenuTab", true, false) if ui != null else null
	if scene == null or not card is Control or not tab is Control:
		fail("Native character-select scene, card or MENU tab missing")
		return
	var charselect_camera_options = load("res://tests/charselect_camera_options_probe.gd").new()
	if not await charselect_camera_options.run(self, client):
		return
	await click(tab)
	if not await wait_menu(client):
		return
	if not menu_authored(client) or not roster_unchanged(client, scene, ui):
		fail("Character-select menu did not retain authored modal and original scene")
		return
	push_key(KEY_DOWN, true)
	await process_frame
	push_key(KEY_DOWN, false)
	await click(card)
	for frame in range(8):
		await process_frame
	if not roster_unchanged(client, scene, ui):
		fail("Character-select roster acted beneath menu")
		return
	push_key(KEY_ESCAPE, true)
	await process_frame
	push_key(KEY_ESCAPE, false)
	if not await wait_menu_closed(client, scene):
		return
	await click(tab)
	if not await wait_menu(client):
		return
	await click_menu_action(client, "MenuBtnResume")
	if not await wait_menu_closed(client, scene):
		return
	if not roster_unchanged(client, scene, ui):
		fail("Return replaced character-select scene or roster")
		return
	# Closing the overlay must preserve the existing repeated roster-key behavior.
	push_key(KEY_DOWN, true, true)
	await process_frame
	push_key(KEY_DOWN, false)
	if ui.find_child("CharSelectCharacterName", true, false).text != "Unequipped Fixture":
		fail("Closed menu suppressed repeated roster key")
		return
	push_key(KEY_UP, true)
	await process_frame
	push_key(KEY_UP, false)
	if not roster_unchanged(client, scene, ui):
		fail("Roster navigation did not return to original selection")
		return
	print("FIXTURE MENU_CHARSELECT_DONE")
	var enter = ui.find_child("EnterWorld", true, false)
	if not enter is Button:
		fail("Native Enter World button missing")
		return
	await click(enter)
	if not await wait_screen(client, "Loading", 15000):
		return
	print("FIXTURE MENU_LOADING")
	if not await wait_world(client):
		return
	var player := client.get_node_or_null("WorldUnits/" + NAME) as Node3D
	var camera := client.get_node_or_null("WorldCamera") as Camera3D
	var animation = load("res://tests/player_locomotion_probe.gd").new()
	if player == null or camera == null:
		fail("Native world player or camera missing")
		return
	var model_deadline := Time.get_ticks_msec() + WORLD_WAIT_MS
	while animation.bind(player) != "":
		if Time.get_ticks_msec() > model_deadline:
			fail("Timed out loading native player animation")
			return
		await process_frame
	if animation.animation.current_animation_id() != 0:
		fail("World player did not start idle Stand 0")
		return
	var marker_options = load("res://tests/target_marker_options_probe.gd").new()
	if not await marker_options.run(self, client, player):
		return
	var camera_options = load("res://tests/camera_options_probe.gd").new()
	if not await camera_options.run(self, client):
		return
	var camera_zoom_options = load("res://tests/camera_zoom_options_probe.gd").new()
	if not await camera_zoom_options.run(self, client, camera_options):
		return
	var target_binding_options = load("res://tests/target_binding_options_probe.gd").new()
	if not await target_binding_options.run(self, client, player):
		return
	push_key(KEY_ESCAPE, true)
	await process_frame
	push_key(KEY_ESCAPE, false)
	if not await wait_menu(client) or not menu_authored(client):
		fail("InWorld Escape did not open authored menu")
		return
	print("FIXTURE MENU_WORLD_OPEN")
	var start := player.position
	var facing := player.rotation.y
	var orbit := camera.global_transform.basis.z
	push_key(KEY_W, true)
	push_key(KEY_SPACE, true)
	var right := InputEventMouseButton.new()
	right.button_index = MOUSE_BUTTON_RIGHT
	right.position = Vector2(640, 360)
	right.pressed = true
	root.push_input(right, true)
	var motion := InputEventMouseMotion.new()
	motion.position = right.position + Vector2(60, 20)
	motion.relative = Vector2(60, 20)
	root.push_input(motion, true)
	for frame in range(25):
		await process_frame
	right.pressed = false
	root.push_input(right, true)
	push_key(KEY_SPACE, false)
	push_key(KEY_W, false)
	print("FIXTURE MENU_BLOCK_QUIET_START")
	for frame in range(QUIET_FRAMES):
		await process_frame
	if player.position.distance_to(start) > 0.05 or absf(player.rotation.y - facing) > 0.01 or camera.global_transform.basis.z.distance_to(orbit) > 0.01 or animation.animation.current_animation_id() != 0:
		fail("Menu allowed movement, jump, facing/camera change or left idle animation")
		return
	if OS.get_environment("GODOT_TEST_VISUAL") == "1":
		await RenderingServer.frame_post_draw
		var image := root.get_texture().get_image()
		var path := "res://../data/diagnostics/godot-conversion/game-menu.png"
		var error := image.save_png(ProjectSettings.globalize_path(path))
		if error != OK:
			fail("Could not capture authored game menu: " + str(error))
			return
	print("FIXTURE MENU_BLOCK_DONE")
	await click_menu_action(client, "MenuBtnResume")
	if not await wait_menu_closed(client, null):
		return
	push_key(KEY_W, true)
	var advanced := false
	for frame in range(60):
		await process_frame
		if frame == 0:
			var input_state: Dictionary = client.account_state()
			print("MENU_INPUT_PROBE focus=", root.gui_get_focus_owner(), " dragging=", root.gui_is_dragging(), " menu=", client.get_node_or_null("GameMenuUI"), " screen=", input_state.screen, " allowed=", input_state.gameplay_input_allowed, " windows=", root.get_embedded_subwindows())
		advanced = advanced or player.position.distance_to(start) > 0.1
	push_key(KEY_W, false)
	print("FIXTURE MENU_W_RELEASED")
	if not advanced:
		fail("W after Return did not move native player")
		return
	for frame in range(QUIET_FRAMES):
		await process_frame
	if animation.animation.current_animation_id() != 0:
		fail("Released W did not restore Stand 0")
		return
	print("FIXTURE MENU_FINAL_QUIET")
	push_key(KEY_ESCAPE, true)
	await process_frame
	push_key(KEY_ESCAPE, false)
	if not await wait_menu(client) or not menu_authored(client):
		fail("Final Escape did not reopen authored menu")
		return
	print("FIXTURE MENU_EXIT_READY")
	await click_menu_action(client, "MenuBtnExit")
	# Exit must terminate the real client; the Rust runner requires this child to exit 0.
	var deadline := Time.get_ticks_msec() + MENU_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
	fail("Exit button did not terminate native client")

func roster_unchanged(client: Node, scene: Node, ui: Node) -> bool:
	var name = ui.find_child("CharSelectCharacterName", true, false)
	return client.account_state().screen == "CharacterSelect" and client.get_node_or_null("CharacterSelectScene") == scene and name is Label and name.text == NAME

func menu_authored(client: Node) -> bool:
	var menu := client.get_node_or_null("GameMenuUI")
	return menu != null and menu.visible and menu.find_child("GameMenuRoot", true, false) != null and menu.find_child("MenuBtnResume", true, false) != null and menu.find_child("MenuBtnExit", true, false) != null

func wait_menu(client: Node) -> bool:
	var deadline := Time.get_ticks_msec() + MENU_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if client.get_node_or_null("GameMenuUI") != null:
			return true
	fail("Timed out opening native GameMenuUI")
	return false

func wait_menu_closed(client: Node, original_scene: Node) -> bool:
	var deadline := Time.get_ticks_msec() + MENU_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if client.get_node_or_null("GameMenuUI") == null:
			if original_scene != null and client.get_node_or_null("CharacterSelectScene") != original_scene:
				fail("Closing menu replaced original character-select scene")
				return false
			return true
	fail("Timed out closing GameMenuUI")
	return false

func click_menu_action(client: Node, name: String) -> void:
	var menu := client.get_node_or_null("GameMenuUI")
	var action = menu.find_child(name, true, false) if menu != null else null
	if not action is Control:
		fail("Authored menu action missing: " + name)
		return
	await click(action)

func wait_screen(client: Node, wanted: String, timeout_ms: int) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == wanted:
			if wanted == "CharacterSelect" and (not state.reply_received or state.character_count != 3):
				fail("Fixture authentication did not populate roster: " + str(state))
				return false
			return true
	fail("Timed out waiting for " + wanted + ": " + str(client.account_state()))
	return false

func wait_world(client: Node) -> bool:
	var deadline := Time.get_ticks_msec() + WORLD_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "InWorld" and state.selected_character_name == NAME and state.unit_count == 2:
			var terrain: Dictionary = state.terrain
			if terrain.map == "azeroth" and terrain.pending_count == 0 and terrain.failures.is_empty() and not terrain.parsed_tiles.is_empty() and client.get_node_or_null("WorldUnits/" + NAME) != null:
				return true
	fail("Timed out waiting for native world and terrain: " + str(client.account_state()))
	return false

func push_key(code: Key, pressed: bool, echo: bool = false) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.pressed = pressed
	event.echo = echo
	root.push_input(event, true)

func click(control: Control) -> void:
	var point := control.get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = point
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
	await process_frame

func fail(message: String) -> void:
	push_error(message)
	quit(1)
