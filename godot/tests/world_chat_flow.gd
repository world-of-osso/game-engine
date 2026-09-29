extends SceneTree

# In-world chat frame against a real server (docs/specs/chat-frame.md, parity matrix Chat
# rows): ChatFrame1 sits at BOTTOMLEFT (0, 40), 500x280; the server's MOTD shows as a
# yellow system line; Enter opens the edit box and W then types instead of moving; a /say
# is echoed by the server as "[Name] says:"; /y and /e are echoed; a whisper to an offline
# name gets the server's system error; / opens prefilled; Up recalls the last line;
# Escape closes the box without opening the game menu; the Combat Log tab selects.
# GODOT_TEST_SERVER must name a private server; GODOT_TEST_CAPTURE_DIR receives screenshots.

const ACCOUNT := "fb_chat"
const PASSWORD := "fbtest"
const CHARACTER := "Fbchat"
const MOTD := "Welcome to the chat test realm."
const WORLD_WAIT_MS := 300000
const ECHO_WAIT_MS := 8000
const YELLOW := Color(1, 1, 0, 1)
const WHITE := Color(1, 1, 1, 1)

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
	if shots == "":
		fail("GODOT_TEST_CAPTURE_DIR is required")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, ACCOUNT, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world():
		return
	if not expect_frame_geometry():
		return
	if not await expect_line(MOTD, YELLOW, "the MOTD"):
		return
	await capture("chat-motd.png")

	# Enter opens the edit box; W types instead of moving the character.
	var start = client.account_state().local_player_position
	await tap(KEY_ENTER)
	var edit := edit_box()
	if edit == null or not edit.is_visible_in_tree() or not edit.has_focus():
		fail("Enter did not open and focus ChatFrame1EditBox")
		return
	var header := chat_ui().find_child("ChatFrame1EditBoxHeader", true, false) as Label
	if header == null or not header.is_visible_in_tree() or header.text != "Say: ":
		fail("The open edit box has no 'Say: ' header")
		return
	if not edit.get_theme_color("font_color").is_equal_approx(WHITE):
		fail("Edit box text is %s, expected the Say colour" % edit.get_theme_color("font_color"))
		return
	push_char("w", true)
	for frame in range(30):
		await process_frame
	push_char("w", false)
	await process_frame
	var moved = client.account_state().local_player_position
	if moved != start:
		fail("W moved the character while typing: %s -> %s" % [start, moved])
		return
	if edit.text != "w":
		fail("W did not type into the edit box: '%s'" % edit.text)
		return
	await type_text("ave hello chat")
	await capture("chat-typing.png")
	await tap(KEY_ENTER)
	if edit.is_visible_in_tree():
		fail("Enter did not close the edit box")
		return
	if not await expect_line("[%s] says: wave hello chat" % CHARACTER, WHITE, "the /say echo"):
		return

	if not await send_line("/y loud noises"):
		return
	if not await expect_line("[%s] yells: loud noises" % CHARACTER, Color(1, 0.25, 0.25, 1), "the /yell echo"):
		return
	if not await send_line("/e checks the chat"):
		return
	if not await expect_line("%s checks the chat" % CHARACTER, Color(1, 0.5, 0.25, 1), "the /emote echo"):
		return
	if not await send_line("/w Nobodyhere hi"):
		return
	if not await expect_line("No player named 'Nobodyhere' is currently playing.", YELLOW, "the whisper error"):
		return
	if not await send_line("/join Trade"):
		return
	if not await expect_line("Type '/help' for a listing of a few commands.", YELLOW, "the unknown command"):
		return
	if not await send_line("/help"):
		return
	if not await expect_line("Up/Down recall sent lines; Escape closes the chat box", YELLOW, "the /help listing"):
		return
	await capture("chat-lines.png")

	# The wheel over the messages scrolls the chat, not the camera; Scroll to bottom returns.
	var newest: String = chat_rows().back().text
	var zoom = client.account_state().camera_distance
	var messages := chat_ui().find_child("ChatFrame1Messages", true, false) as Control
	for notch in range(3):
		await wheel(messages.get_global_rect().get_center(), MOUSE_BUTTON_WHEEL_UP)
	var bottom := chat_ui().find_child("ChatFrame1ScrollToBottomButton", true, false) as Control
	if not bottom.is_visible_in_tree() or chat_rows().back().text == newest:
		fail("Wheel up did not scroll the chat: bottom row '%s'" % chat_rows().back().text)
		return
	if client.account_state().camera_distance != zoom:
		fail("Wheel over the chat zoomed the camera")
		return
	await capture("chat-scrolled-up.png")
	await click(bottom.get_global_rect().get_center())
	if bottom.is_visible_in_tree() or chat_rows().back().text != newest:
		fail("Scroll to bottom did not return to the newest line")
		return

	# / opens prefilled; Up recalls the newest sent line; Escape closes without sending.
	await tap_char("/")
	if not edit.is_visible_in_tree() or edit.text != "/":
		fail("/ did not open the edit box prefilled: '%s'" % edit.text)
		return
	await tap(KEY_UP)
	if edit.text != "/help":
		fail("Up recalled '%s', expected '/help'" % edit.text)
		return
	await tap(KEY_ESCAPE)
	if edit.is_visible_in_tree():
		fail("Escape did not close the edit box")
		return
	if client.get_node_or_null("GameMenuUI") != null:
		fail("Escape closing chat also opened the game menu")
		return

	# The Combat Log tab selects and hides chat lines.
	var tab := chat_ui().find_child("ChatFrame1TabsTab1", true, false) as Control
	await click(tab.get_global_rect().get_center())
	if tab.modulate.a != 1.0:
		fail("Combat Log tab not selected: alpha %s" % tab.modulate.a)
		return
	if not chat_rows().filter(func(row): return MOTD in row.text).is_empty():
		fail("Combat Log tab still lists chat lines")
		return
	await capture("chat-combat-log-tab.png")
	var general := chat_ui().find_child("ChatFrame1TabsTab0", true, false) as Control
	await click(general.get_global_rect().get_center())
	await capture("chat-general.png")
	print("FIXTURE WORLD_CHAT_DONE")
	client.free()
	quit(0)

func chat_ui() -> Node:
	return client.get_node_or_null("ChatFrameUI")

func edit_box() -> LineEdit:
	var ui := chat_ui()
	return ui.find_child("ChatFrame1EditBox", true, false) as LineEdit if ui != null else null

# ChatFrame1 500x280 UI units at BOTTOMLEFT (0, 40) (Chattynator Core/Config.lua:28-29), in
# the HUD's effective UI scale (the canvas scale every in-world RegistryUi shares).
func expect_frame_geometry() -> bool:
	var frame := chat_ui().find_child("ChatFrame1", true, false) as Control
	if frame == null or not frame.is_visible_in_tree():
		fail("No visible ChatFrame1")
		return false
	var size := Vector2(root.size)
	var scale: float = (chat_ui().get_node("RegistryCanvas") as Control).scale.x
	print("FIXTURE UI_SCALE ", scale)
	var expected := Rect2(0, size.y - 320.0 * scale, 500.0 * scale, 280.0 * scale)
	var actual := frame.get_global_rect()
	if actual.position.distance_to(expected.position) > 0.5 or actual.size.distance_to(expected.size) > 0.5:
		fail("ChatFrame1 is %s, expected %s" % [actual, expected])
		return false
	print("FIXTURE CHAT_FRAME ", actual)
	return true

# Shown message rows, top to bottom: {text, color} with the row's runs joined.
func chat_rows() -> Array:
	var rows := {}
	for label in chat_ui().find_children("ChatFrame1*", "Label", true, false):
		var name := str(label.name)
		if not label.is_visible_in_tree():
			continue
		var row := ""
		if name.begins_with("ChatFrame1MessagesRow"):
			row = name.substr(0, name.find("Run"))
		elif name.begins_with("ChatFrame1Link"):
			row = "ChatFrame1MessagesRow" + name.substr(14).split("_")[0]
		else:
			continue
		if not rows.has(row):
			rows[row] = {"text": "", "color": label.get_theme_color("font_color"), "y": label.get_global_rect().position.y}
		rows[row].text += label.text
	var out := rows.values()
	out.sort_custom(func(a, b): return a.y < b.y)
	return out

func expect_line(text: String, color: Color, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + ECHO_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		for row in chat_rows():
			if row.text.strip_edges() == text:
				if not row.color.is_equal_approx(color):
					fail("%s shows %s, expected %s" % [what, row.color, color])
					return false
				print("FIXTURE LINE ", what, ": ", row.text)
				return true
	fail("No %s line '%s' in %s" % [what, text, chat_rows().map(func(row): return row.text)])
	return false

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

func wheel(point: Vector2, button: MouseButton) -> void:
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = button
		event.pressed = pressed
		event.factor = 1.0
		root.push_input(event, true)
	await process_frame
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

func tap_char(character: String) -> void:
	push_char(character, true)
	await process_frame
	push_char(character, false)
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
	quit(1)
