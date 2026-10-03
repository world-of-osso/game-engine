extends "res://tests/world_merchant_flow.gd"

# One live Enter World on a private server, reporting the loading screen's status every
# 10 s until the world shows (FIXTURE ENTRY_DONE <seconds>) or LOOP_WORLD_S passes
# (FIXTURE FAIL). Run it repeatedly to catch a Loading screen that never finishes.
# Environment:
#   GODOT_TEST_SERVER                 private server address (never 127.0.0.1:5000)
#   LOOP_ACCOUNT / LOOP_CHARACTER     account (password fbtest) and character
#   LOOP_WORLD_S                      seconds to wait for the world (default 420)

func run_test() -> void:
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("LOOP_ACCOUNT")
	var character := OS.get_environment("LOOP_CHARACTER")
	if server.is_empty() or server == "127.0.0.1:5000" or account.is_empty() or character.is_empty():
		fail("Needs a private GODOT_TEST_SERVER and LOOP_ACCOUNT/LOOP_CHARACTER")
		return
	var wait_s := int(OS.get_environment("LOOP_WORLD_S")) if OS.has_environment("LOOP_WORLD_S") else 420
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Live connection: " + error)
		return
	var deadline := Time.get_ticks_msec() + 30000
	var ui: Node = null
	while Time.get_ticks_msec() < deadline and ui == null:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.reply_received and state.screen == "CharacterSelect" and state.character_count >= 1:
			ui = client.get_node_or_null("CharacterSelectUI")
	if ui == null:
		fail("No character select: " + str(client.account_state()))
		return
	await frames(10)
	var card: Control = null
	for candidate in ui.find_children("CharCard_*", "", true, false):
		for label in candidate.find_children("*", "Label", true, false):
			if label.text == character:
				card = candidate
	if card == null:
		fail("Roster has no " + character)
		return
	await click_control(card, MOUSE_BUTTON_LEFT)
	await click_control(ui.find_child("EnterWorld", true, false), MOUSE_BUTTON_LEFT)
	var started := Time.get_ticks_msec()
	var next_report := started
	var frame := 0
	while Time.get_ticks_msec() < started + wait_s * 1000:
		await process_frame
		frame += 1
		var state: Dictionary = client.account_state()
		if state.screen == "InWorld" and state.local_player_position != null:
			print("FIXTURE ENTRY_DONE %.1f frames=%d" % [(Time.get_ticks_msec() - started) / 1000.0, frame])
			client.free()
			quit(0)
			return
		if Time.get_ticks_msec() >= next_report:
			next_report += 10000
			print("FIXTURE LOADING t=%.0f frames=%d status=%s objects=%s units_pending=%s" % [
				(Time.get_ticks_msec() - started) / 1000.0, frame, loading_status(),
				_objects(state),
				state.get("unit_visuals_pending")])
	fail("Timed out entering the world: " + loading_status() + " " + str(client.account_state()))

func _objects(state: Dictionary) -> String:
	var objects: Dictionary = state.get("world_objects", {})
	return "spawned=%s pending=%s failures=%s" % [objects.get("spawned"), objects.get("pending"), objects.get("failures")]

func loading_status() -> String:
	var texts: Array[String] = []
	for label in client.find_children("*", "Label", true, false):
		if label.is_visible_in_tree() and (label.text.contains("Loading") or label.text.contains("...")):
			texts.append(label.text)
	return "|".join(texts)
