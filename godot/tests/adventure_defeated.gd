extends "res://tests/adventure_instance_loot.gd"
# Real private-server boss death and RequestRaidInfo; never inject client lock state.
var evidence := ""
const MARKER := "JournalBoss2617DefeatedOverlay"

func phase(name: String) -> void:
	var file := FileAccess.open(evidence.path_join("phase.json"), FileAccess.WRITE)
	file.store_string(JSON.stringify({"phase":name,"account":client.account_state(),"journal":client.adventure_guide_state()}))
	file.close()

func wait_signal(name: String) -> bool:
	var deadline := Time.get_ticks_msec() + 180000
	while not FileAccess.file_exists(evidence.path_join(name + ".json")) and Time.get_ticks_msec() < deadline:
		await process_frame
	if not FileAccess.file_exists(evidence.path_join(name + ".json")):
		stop("Private realm proof signal missing: " + name)
		return false
	return true

func expect_marker(shown: bool) -> bool:
	var deadline := Time.get_ticks_msec() + 45000
	while Time.get_ticks_msec() < deadline:
		var marker := control(MARKER)
		if (marker != null and marker.is_visible_in_tree()) == shown:
			return true
		await process_frame
	stop("Saved Umbriss marker expected=" + str(shown))
	return false

func refresh_received_locks() -> void:
	await journal_key()
	await journal_key()
	await settle()

func choose_difficulty(id: int) -> bool:
	return await click("JournalDifficultyDropdown") and await click("JournalDifficulty%s" % id)

func run() -> void:
	var server := OS.get_environment("ADVENTURE_GUIDE_SERVER")
	var account := OS.get_environment("ADVENTURE_GUIDE_ACCOUNT")
	shots = OS.get_environment("ADVENTURE_GUIDE_SHOTS")
	evidence = OS.get_environment("ADVENTURE_GUIDE_CONTROL")
	if server != "127.0.0.1:5771" or not account.begins_with("fb_adventure4d") or shots.is_empty() or evidence.is_empty():
		stop("Explicit owned realm/account/screenshots/control required")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var deadline := Time.get_ticks_msec() + 180000
	while client.account_state().assets_starting and Time.get_ticks_msec() < deadline:
		await process_frame
	var error: String = client.connect_account(server, account, OS.get_environment("ADVENTURE_GUIDE_PASSWORD"), false)
	if not error.is_empty():
		stop(error)
		return
	if not await enter_world_when_ready():
		return
	error = client.set_dungeon_difficulty(2)
	if not error.is_empty():
		stop(error)
		return
	deadline = Time.get_ticks_msec() + 45000
	while client.entrance_bar_state().dungeon_difficulty != 2 and Time.get_ticks_msec() < deadline:
		await process_frame
	if client.entrance_bar_state().dungeon_difficulty != 2:
		stop("Realm did not acknowledge Heroic dungeon difficulty")
		return
	phase("difficulty-ready")
	if not await wait_signal("teleported"):
		return
	deadline = Time.get_ticks_msec() + 180000
	while (client.account_state().world_map_id != 670 or client.account_state().pending_world_port != "None") and Time.get_ticks_msec() < deadline:
		await process_frame
	if client.account_state().world_map_id != 670:
		stop("Realm map transfer to actual Grim Batol failed")
		return
	await journal_key()
	deadline = Time.get_ticks_msec() + 90000
	while not client.adventure_guide_state().loaded and Time.get_ticks_msec() < deadline:
		await process_frame
	if not await open_loot_instance("EncounterJournalDungeonTab", 73, 71) or not await choose_difficulty(2):
		return
	if not await expect_marker(false) or not await shot("defeated-before-kill"):
		return
	phase("before-kill")
	if not await wait_signal("killed"):
		return
	await refresh_received_locks()
	if not await expect_marker(true):
		return
	if not await verify_painted_icon("JournalBoss2617DefeatedIcon"):
		return
	if client.raid_difficulty_state().saved_count < 1 or not await move_to(MARKER):
		stop("No received save or physical defeated-icon hover")
		return
	var tooltip: Dictionary = client.tooltip_state()
	var hovered := root.gui_get_hovered_control()
	print("JOURNAL_DEFEATED_TOOLTIP actual=%s native_hover=%s marker=%s" % [JSON.stringify(tooltip), str(hovered.get_path()) if hovered != null else "none", control(MARKER).get_global_rect()])
	if not tooltip.get("visible", false) or not str(tooltip.get("lines", "")).contains("You've already defeated this boss on Heroic difficulty this week."):
		await RenderingServer.frame_post_draw
		root.get_texture().get_image().save_png(evidence.path_join("tooltip-failure.png"))
		stop("Source saved-boss difficulty tooltip missing")
		return
	if not await shot("defeated-after-kill-tooltip"):
		return
	if not await choose_difficulty(1) or not await expect_marker(false) or not await shot("defeated-other-difficulty"):
		return
	if not await choose_difficulty(2) or not await expect_marker(true):
		return
	await refresh_received_locks()
	if not await expect_marker(true) or not await shot("defeated-reopened"):
		return
	phase("expire-save")
	if not await wait_signal("expired"):
		return
	await refresh_received_locks()
	if not await expect_marker(false) or not await shot("defeated-expired"):
		return
	phase("passed")
	print("JOURNAL_DEFEATED_PASS skin=%s realUmbrissDeath/sourceBit3/HeroicOnly/tooltip/reopen/expiry" % client.account_state().ui_skin)
	quit(0)
