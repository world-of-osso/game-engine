extends SceneTree

# Live Forever input proof: a player-frame click updates the selected unit,
# TargetFrame and selection circle in the real client.
const CHARACTER_WAIT_MS := 15000
const WORLD_WAIT_MS := 120000
const TARGET_WAIT_MS := 5000

var client: Node
var character := ""
var screenshot_path := ""

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("GODOT_TEST_ACCOUNT")
	var password := OS.get_environment("GODOT_TEST_PASSWORD")
	character = OS.get_environment("GODOT_TEST_CHARACTER")
	screenshot_path = OS.get_environment("GODOT_UNITFRAME_SCREENSHOT")
	if server.is_empty() or account.is_empty() or password.is_empty() or character.is_empty() or screenshot_path.is_empty():
		quit(1)
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error: String = client.connect_account(server, account, password, false)
	var passed := error.is_empty()
	if passed:
		passed = await enter_world()
	if passed:
		passed = await click_player_frame()
	if is_instance_valid(client):
		client.free()
	quit(0 if passed else 1)

func enter_world() -> bool:
	var deadline := Time.get_ticks_msec() + CHARACTER_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.reply_received and state.screen == "CharacterSelect" and state.character_count > 0:
			break
	var ui := client.get_node_or_null("CharacterSelectUI")
	if ui == null:
		return reject("No character select: " + str(client.account_state()))
	var card := roster_card(ui)
	if card == null:
		return reject("Roster has no " + character)
	await click_control(card)
	await click_control(ui.find_child("EnterWorld", true, false) as Control)
	deadline = Time.get_ticks_msec() + WORLD_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "InWorld" and state.local_player_position != null:
			if state.ui_skin != "Forever":
				return reject("Expected Forever skin, got " + str(state.ui_skin))
			return true
	return reject("Timed out entering the world: " + str(client.account_state()))

func roster_card(ui: Node) -> Control:
	var index := 0
	while true:
		var card := ui.find_child("CharCard_%d" % index, true, false) as Control
		if card == null:
			return null
		for label in card.find_children("*", "Label", true, false):
			if label.text == character:
				return card
		index += 1

func click_player_frame() -> bool:
	var ui := client.get_node_or_null("UnitFramesUI")
	var frame := ui.find_child("PlayerFrame", true, false) as Control if ui != null else null
	if frame == null or not frame.is_visible_in_tree():
		return reject("Forever PlayerFrame is absent or hidden")
	await click_control(frame)
	var deadline := Time.get_ticks_msec() + TARGET_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.target_state()
		var target_frame := ui.find_child("TargetFrame", true, false) as Control
		var target_name := ui.find_child("TargetName", true, false) as Label
		if state.target != null and state.target_name == character and state.circle_on == state.target:
			if target_frame == null or not target_frame.is_visible_in_tree() or target_name == null or target_name.text != character:
				return reject("TargetFrame did not show the selected player")
			if not await capture_player_target():
				return false
			print("UNITFRAME_WAITING_FOR_PARTY_FRAME")
			return await click_party_frame()
	return reject("PlayerFrame click did not target the local player: " + str(client.target_state()))

func click_party_frame() -> bool:
	var deadline := Time.get_ticks_msec() + WORLD_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var party := first_non_local_party_frame()
		if party.is_empty():
			continue
		await click_control(party.frame)
		var state: Dictionary = client.target_state()
		if state.target == null or state.target_name != party.name or state.circle_on != state.target:
			return reject("Party-frame click did not target " + party.name + ": " + str(state))
		var ui := client.get_node_or_null("UnitFramesUI")
		var target_name := ui.find_child("TargetName", true, false) as Label if ui != null else null
		if target_name == null or target_name.text != party.name:
			return reject("TargetFrame did not show party member " + party.name)
		print("UNITFRAME_PARTY_CLICK_PASS member=" + party.name + " id=" + str(state.target))
		return true
	return reject("No non-local party frame appeared before timeout")

func first_non_local_party_frame() -> Dictionary:
	var ui := client.get_node_or_null("GroupFramesUI")
	if ui == null:
		return {}
	var candidates: Array[String] = []
	for group in range(1, 9):
		for member in range(1, 6):
			candidates.append("CompactRaidGroup%dMember%d" % [group, member])
	for member in range(1, 6):
		candidates.append("CompactPartyFrameMember%d" % member)
	for member in range(1, 5):
		candidates.append("PartyMemberFrame%d" % member)
	for frame_name in candidates:
		var frame := ui.find_child(frame_name, true, false) as Control
		var label := ui.find_child(frame_name + "Name", true, false) as Label
		if frame != null and frame.is_visible_in_tree() and label != null and label.is_visible_in_tree() and label.text != character and not label.text.is_empty():
			return {"frame": frame, "name": label.text}
	return {}

func capture_player_target() -> bool:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	DirAccess.make_dir_recursive_absolute(screenshot_path.get_base_dir())
	var error := image.save_png(screenshot_path)
	if error != OK:
		return reject("Could not save screenshot: " + screenshot_path)
	print("UNITFRAME_CLICK_PASS Forever PlayerFrame -> TargetFrame and selection circle; screenshot=" + screenshot_path)
	return true

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

func reject(message: String) -> bool:
	push_error(message)
	return false
