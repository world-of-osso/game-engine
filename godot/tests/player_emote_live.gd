extends SceneTree

# Social emotes on the local player's model against a private server: /sit holds
# SitGround (97) until the player moves, /dance holds EmoteDance (69), /wave plays
# EmoteWave (67) once over the stance (Emotes.db2 rows 3 and 10; Bevy EmoteAnimState).
# GODOT_TEST_SERVER must name a private server; EMOTE_ACCOUNT/EMOTE_CHARACTER the
# account (password fbtest) and its only character; GODOT_TEST_CAPTURE_DIR receives
# screenshots.

const PASSWORD := "fbtest"
const WORLD_WAIT_MS := 300000
const SIT := 97
const DANCE := 69
const WAVE := 67

var client: Node
var shots := ""

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if server == "" or server.ends_with(":5000"):
		fail("GODOT_TEST_SERVER must select a private server, not :5000")
		return
	shots = OS.get_environment("GODOT_TEST_CAPTURE_DIR")
	var account := OS.get_environment("EMOTE_ACCOUNT")
	if shots == "" or account == "":
		fail("GODOT_TEST_CAPTURE_DIR and EMOTE_ACCOUNT are required")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world():
		return
	if not await expect_clip(0, "Stand before any emote"):
		return
	if not await send_line("/sit"):
		return
	if not await expect_clip(SIT, "/sit"):
		return
	await capture("emote-sit.png")
	# Walking stands the player up; standing still again keeps Stand.
	push_key(KEY_W, 0, true)
	for frame in range(30):
		await process_frame
	push_key(KEY_W, 0, false)
	if animation().current_animation_id() == SIT:
		fail("Walking kept SitGround")
		return
	if not await expect_clip(0, "Stand after walking out of /sit"):
		return
	if not await send_line("/dance"):
		return
	if not await expect_clip(DANCE, "/dance"):
		return
	await capture("emote-dance.png")
	if not await send_line("/wave"):
		return
	var deadline := Time.get_ticks_msec() + 5000
	while animation().current_action_id() != WAVE:
		if Time.get_ticks_msec() > deadline:
			fail("/wave played no EmoteWave action (action %d)" % animation().current_action_id())
			return
		await process_frame
	await capture("emote-wave.png")
	print("FIXTURE EMOTES_DONE")
	quit(0)

func animation() -> WowAnimationPlayer:
	var model := client.find_child("PlayerModel", true, false)
	return model.get_node_or_null("M2Animation") as WowAnimationPlayer if model != null else null

func expect_clip(id: int, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + 5000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var player := animation()
		if player != null and player.current_animation_id() == id:
			print("FIXTURE CLIP ", what, ": ", id)
			return true
	var current := animation().current_animation_id() if animation() != null else -1
	fail("%s: clip %d, not %d" % [what, current, id])
	return false

func chat_ui() -> Node:
	return client.get_node_or_null("ChatFrameUI")

func edit_box() -> LineEdit:
	var ui := chat_ui()
	return ui.find_child("ChatFrame1EditBox", true, false) as LineEdit if ui != null else null

# ChatFrame1 500x280 UI units at BOTTOMLEFT (0, 40) (Chattynator Core/Config.lua:28-29), in
# the HUD's effective UI scale (the canvas scale every in-world RegistryUi shares).

func send_line(line: String) -> bool:
	await tap(KEY_ENTER)
	var edit := edit_box()
	if not edit.has_focus():
		fail("Enter did not focus the edit box before '%s'" % line)
		return false
	await type_text(line)
	await tap(KEY_ENTER)
	return true

func enter_world() -> bool:
	var deadline := Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.reply_received and state.screen == "CharacterSelect" and state.character_count >= 1 and client.get_node_or_null("CharacterSelectUI") != null:
			break
	var ui = client.get_node_or_null("CharacterSelectUI")
	if ui == null:
		fail("No character select: " + str(client.account_state()))
		return false
	await click_control(ui.find_child("EnterWorld", true, false))
	deadline = Time.get_ticks_msec() + WORLD_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "InWorld" and state.local_player_position != null:
			print("FIXTURE IN_WORLD ", state.selected_character_name, " at ", state.local_player_position)
			for frame in range(30):
				await process_frame
			return true
	fail("Timed out entering the world: " + str(client.account_state()))
	return false

func click(point: Vector2) -> void:
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
	for frame in range(3):
		await process_frame

func click_control(control: Control) -> void:
	await click(control.get_global_rect().get_center())

func capture(file: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var error := image.save_png(shots.path_join(file))
	if error != OK:
		fail("Could not save " + file + ": " + str(error))
	print("FIXTURE CAPTURE ", shots.path_join(file))

func tap(code: Key) -> void:
	push_key(code, 0, true)
	await process_frame
	push_key(code, 0, false)
	await process_frame
	await process_frame

func type_text(text: String) -> void:
	for character in text:
		push_char(character, true)
		await process_frame
		push_char(character, false)
	await process_frame

func push_char(character: String, pressed: bool) -> void:
	var code := OS.find_keycode_from_string(character.to_upper())
	if character == " ":
		code = KEY_SPACE
	elif character == "/":
		code = KEY_SLASH
	elif character == "'":
		code = KEY_APOSTROPHE
	push_key(code, character.unicode_at(0), pressed)

func push_key(code: Key, unicode: int, pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.unicode = unicode if pressed else 0
	event.pressed = pressed
	root.push_input(event, true)

func fail(message: String) -> void:
	push_error(message)
	print("FIXTURE FAIL ", message)
	quit(1)
