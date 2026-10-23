extends "res://tests/adventure_pages_live.gd"
# Private harness supplies actual source-backed deaths, ports and combat events; no client injection.
var evidence := ""

func phase(name: String) -> void:
	var file := FileAccess.open(evidence.path_join("phase.json"), FileAccess.WRITE)
	file.store_string(JSON.stringify({"phase": name, "account": client.account_state(), "journal": client.adventure_guide_state()}))
	file.close()

func wait_signal(name: String) -> bool:
	var deadline := Time.get_ticks_msec() + 180000
	while not FileAccess.file_exists(evidence.path_join(name + ".json")) and Time.get_ticks_msec() < deadline:
		await process_frame
	if not FileAccess.file_exists(evidence.path_join(name + ".json")):
		stop("Private realm did not finish " + name)
		return false
	return true

func wait_map(map_id: int) -> bool:
	var deadline := Time.get_ticks_msec() + 180000
	while Time.get_ticks_msec() < deadline:
		var account: Dictionary = client.account_state()
		if account.world_map_id == map_id and account.pending_world_port == "None":
			await settle()
			return true
		await process_frame
	stop("Private map transfer did not finish: " + str(map_id))
	return false

func attack_named_boss(name: String) -> bool:
	var deadline := Time.get_ticks_msec() + 60000
	while client.target_state().get("target_name", "") != name and Time.get_ticks_msec() < deadline:
		await press_key(KEY_TAB)
	if client.target_state().get("target_name", "") != name:
		stop("Physical Tab did not select real boss " + name)
		return false
	# Shared default-bar Attack action, also used by melee_sounds_live.gd.
	var attack: int = client.spells_state().bar.find(88163)
	var keys := [KEY_1, KEY_2, KEY_3, KEY_4, KEY_5, KEY_6, KEY_7, KEY_8, KEY_9, KEY_0, KEY_MINUS, KEY_EQUAL]
	if attack < 0 or attack >= keys.size():
		stop("Actual Attack action unavailable: " + str(client.spells_state().bar))
		return false
	var target = client.target_state().target
	await press_key(keys[attack])
	deadline = Time.get_ticks_msec() + 15000
	while client.target_state().auto_attack != target and Time.get_ticks_msec() < deadline:
		await process_frame
	if client.target_state().auto_attack != target:
		stop("Physical Attack did not start on actual boss " + name)
		return false
	print("AG_PHYSICAL_BOSS_ATTACK ", client.target_state())
	return true

func boss_frame_right_click() -> bool:
	var deadline := Time.get_ticks_msec() + 60000
	while not shown("Boss1TargetFrame") and Time.get_ticks_msec() < deadline:
		await process_frame
	if not shown("Boss1TargetFrame"):
		stop("Real private encounter did not present its boss frame")
		return false
	if client.adventure_guide_state().visible or client.adventure_guide_state().loaded:
		stop("Received boss frame auto-opened the previously unopened Guide")
		return false
	var point := control("Boss1TargetFrame").get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await process_frame
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_RIGHT
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await settle()
	return true

func verify_received_save_and_key() -> bool:
	if not await select_named_boss("Lucifron", [KEY_L,KEY_U,KEY_C,KEY_I,KEY_F,KEY_R,KEY_O,KEY_N], 1519):
		return false
	if client.adventure_guide_state().difficulty != 9 or not shown("JournalBoss1519DefeatedOverlay"):
		stop("Real saved Molten Core9 Lucifron marker missing")
		return false
	if not await move_to("JournalBoss1519DefeatedOverlay") or not client.tooltip_state().get("visible", false) or not await shot("integration-lucifron-saved"):
		return false
	if not await click("EncounterJournalSearchBoxClearButton") or not await type_word("The Azure Vault", [KEY_T,KEY_H,KEY_E,KEY_SPACE,KEY_A,KEY_Z,KEY_U,KEY_R,KEY_E,KEY_SPACE,KEY_V,KEY_A,KEY_U,KEY_L,KEY_T]) or not await click("JournalSearchPreview0"):
		return false
	if not await click("EncounterJournalPage0"):
		return false
	var state: Dictionary = client.adventure_guide_state()
	var key := label_text("EncounterJournalKeystone")
	if state.instance != 1203 or state.encounter != -1 or not key.contains("The Azure Vault +8") or not key.contains("37:30"):
		stop("Authoritative owned key401 did not join authentic Azure dungeon/timer: " + key)
		return false
	return await shot("integration-owned-azure-key")

func run() -> void:
	evidence = OS.get_environment("ADVENTURE_GUIDE_CONTROL")
	if not evidence.begins_with("/"):
		stop("Explicit private control directory required")
		return
	if not await connect_owned_realm():
		return
	phase("closed-ready")
	if not await wait_signal("lucifron-ported") or not await wait_map(409):
		return
	phase("lucifron-entered")
	if not await wait_signal("lucifron-killed"):
		return
	if client.adventure_guide_state().visible or client.adventure_guide_state().loaded:
		stop("Closed Guide auto-opened on the real Lucifron event")
		return
	phase("cobrahn-ready")
	if not await wait_signal("cobrahn-ported") or not await wait_map(43):
		return
	phase("cobrahn-entered")
	if not await wait_signal("cobrahn-started"):
		return
	if client.adventure_guide_state().visible or client.adventure_guide_state().loaded:
		stop("Closed Guide auto-opened on the real Cobrahn event")
		return
	if not await attack_named_boss("Lord Cobrahn") or not await boss_frame_right_click() or not await wait_journal(func(state): return state.loaded and state.encounter == 475 and state.lockout_count > 0 and state.owned_key_count > 0, "First-open boss link/snapshot replies incomplete"):
		return
	if client.adventure_guide_state().difficulty != 1 or not await shot("integration-first-boss-link"):
		stop("First boss link did not use real Normal difficulty")
		return
	phase("open-guide-ready")
	if not await wait_signal("pythas-started") or not await wait_journal(func(state): return state.encounter == 476 and state.difficulty == 1, "Open Guide did not follow actual Pythas ENCOUNTER_START"):
		return
	if not await shot("integration-open-guide-encounter-start"):
		return
	phase("pythas-observed")
	if not await wait_signal("safe-origin") or not await wait_map(0) or not await verify_received_save_and_key():
		return
	phase("passed")
	print("AG_PAGES_INTEGRATION_PASS skin=%s closedStart/firstBossRightClick/nonemptySave/openStart/realLucifronDeath/sourceMapAzureKey" % client.account_state().ui_skin)
	quit(0)
