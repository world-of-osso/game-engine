extends "res://tests/adventure_guide_scroll.gd"
# External private harness stages the saved raid and engages the real dungeon boss.
func run() -> void:
	var server := OS.get_environment("ADVENTURE_GUIDE_SERVER")
	var account := OS.get_environment("ADVENTURE_GUIDE_ACCOUNT")
	var encounter_text := OS.get_environment("ADVENTURE_GUIDE_EXPECT_ENCOUNTER")
	var locks_text := OS.get_environment("ADVENTURE_GUIDE_EXPECT_LOCKOUTS")
	if not server.begins_with("127.0.0.1:") or server.ends_with(":5000") or not account.begins_with("fb_"):
		stop("Explicit private endpoint and fb_* account required")
		return
	if not encounter_text.is_valid_int() or not locks_text.is_valid_int() or int(locks_text) <= 0 or int(encounter_text) <= 0:
		stop("Explicit authored encounter ID and nonempty expected saved locks required")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await settle()
	var error: String = client.connect_account(server, account, OS.get_environment("ADVENTURE_GUIDE_PASSWORD"), false)
	if not error.is_empty():
		stop(error)
		return
	if not await wait_screen("CharacterSelect"): return
	await settle()
	await click("EnterWorld")
	if not await wait_screen("InWorld"): return
	print("ADVENTURE_GUIDE_BOSS_WAITING character=" + str(client.account_state().selected_character_name))
	var deadline := Time.get_ticks_msec() + 120000
	var boss: Control
	while Time.get_ticks_msec() < deadline:
		boss = client.find_child("Boss1TargetFrame", true, false) as Control
		if boss != null and boss.is_visible_in_tree(): break
		await process_frame
	if boss == null or not boss.is_visible_in_tree():
		stop("Real server boss frame did not engage")
		return
	if client.adventure_guide_state().visible or client.adventure_guide_state().loaded:
		stop("Guide must remain unopened before the first real boss link")
		return
	await pointer(boss.get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
	deadline = Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		var state = client.adventure_guide_state()
		if state.loaded and state.lockout_count == int(locks_text): break
		await process_frame
	var state = client.adventure_guide_state()
	if not state.visible or state.encounter != int(encounter_text) or state.lockout_count != int(locks_text):
		stop("Boss-first journal/authoritative save mismatch: " + str(state))
		return
	print("ADVENTURE_GUIDE_BOSS_LINK_PASS skin=%s encounter=%s lockouts=%s first_open=true" % [client.account_state().ui_skin, state.encounter, state.lockout_count])
	quit(0)
