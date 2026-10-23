extends "res://tests/adventure_pages_integration_live.gd"
# Real source Font + authenticated production challenge requests; never install a local run.

func snapshot_challenge(name: String) -> void:
	var file := FileAccess.open(evidence.path_join(name + ".json"), FileAccess.WRITE)
	file.store_string(client.challenge_state())
	file.close()

func wait_challenge_phase(expected: String) -> bool:
	var deadline := Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		var snapshot: Dictionary = JSON.parse_string(client.challenge_state())
		if snapshot.get("error") != null:
			stop("Real challenge request refused: " + str(snapshot.error))
			return false
		var run = snapshot.get("run")
		if run != null and run.get("phase", "") == expected:
			await settle()
			return true
		await process_frame
	stop("Realm challenge phase absent: " + expected)
	return false

func wait_owned_key() -> bool:
	var deadline := Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		var snapshot: Dictionary = JSON.parse_string(client.challenge_state())
		for key in snapshot.get("keys", []):
			if int(key.challenge_id) == 401:
				return true
		await process_frame
	stop("Realm did not return the owned Azure key")
	return false

func verify_running_journal() -> bool:
	if not await click("EJMicroButton") or not await wait_journal(func(state): return state.loaded and state.visible, "Active-run Guide did not load"):
		return false
	if not await enter_query("The Azure Vault", [KEY_T,KEY_H,KEY_E,KEY_SPACE,KEY_A,KEY_Z,KEY_U,KEY_R,KEY_E,KEY_SPACE,KEY_V,KEY_A,KEY_U,KEY_L,KEY_T]) or not await click("JournalSearchPreview0"):
		return false
	if not await click("EncounterJournalPage0") or not await click("JournalDifficultyDropdown") or not await click("JournalDifficulty8"):
		return false
	var state: Dictionary = client.adventure_guide_state()
	var snapshot: Dictionary = JSON.parse_string(client.challenge_state())
	var run: Dictionary = snapshot.run
	var summary := label_text("EncounterJournalKeystone")
	if state.instance != 1203 or state.encounter != -1 or state.difficulty != 8 or int(run.map_id) != 2515 or int(run.time_limit_ms) != 2250000:
		stop("Journal active-run map/difficulty/source timer differs")
		return false
	var title := "The Azure Vault +%s" % int(run.key.level)
	var forces := "Enemy Forces: %s / %s" % [int(run.forces), int(run.forces_required)]
	if not summary.contains(title) or not summary.contains("Time Remaining:") or not summary.contains(forces):
		stop("Journal did not display the actual running authority: " + summary)
		return false
	if run.get("rating") == null and summary.contains("Rating:"):
		stop("Unknown rating was invented")
		return false
	for boss in run.bosses:
		if not summary.contains(boss.name):
			stop("Authoritative challenge boss missing: " + boss.name)
			return false
	snapshot_challenge("challenge-running")
	if not await shot("challenge-running-overview"):
		return false
	var before := summary
	await create_timer(2.2).timeout
	if label_text("EncounterJournalKeystone") == before:
		stop("Running journal timer did not advance with realm snapshots")
		return false
	return await shot("challenge-running-clock-advanced")

func run() -> void:
	evidence = OS.get_environment("ADVENTURE_GUIDE_CONTROL")
	if not evidence.begins_with("/"):
		stop("Private control directory required")
		return
	if not await connect_owned_realm():
		return
	phase("challenge-outside-ready")
	if not await wait_signal("challenge-outside") or not await wait_map(1):
		return
	var error: String = client.choose_mythic_difficulty()
	if not error.is_empty():
		stop(error)
		return
	var deadline := Time.get_ticks_msec() + 30000
	while client.entrance_bar_state().dungeon_difficulty != 23 and Time.get_ticks_msec() < deadline:
		await process_frame
	if client.entrance_bar_state().dungeon_difficulty != 23:
		stop("Realm Mythic preference ACK absent")
		return
	phase("challenge-ready")
	if not await wait_signal("challenge-ported") or not await wait_map(2515) or not await wait_screen("InWorld") or not await wait_owned_key():
		return
	# Preserve u64 GUID/entity decimal lexemes: Python consumes this raw authority JSON.
	snapshot_challenge("challenge-owned-key")
	phase("challenge-entered")
	if not await wait_signal("font-bound"):
		return
	var request := FileAccess.get_file_as_string(evidence.path_join("challenge-slot-request.json"))
	error = client.challenge_request(request)
	if not error.is_empty():
		stop(error)
		return
	if not await wait_challenge_phase("Slotted"):
		return
	error = client.challenge_request(JSON.stringify("Start"))
	if not error.is_empty():
		stop(error)
		return
	if not await wait_challenge_phase("Running") or not await verify_running_journal():
		return
	error = client.challenge_request(JSON.stringify("Abandon"))
	if not error.is_empty():
		stop(error)
		return
	if not await wait_challenge_phase("Abandoned"):
		return
	snapshot_challenge("challenge-abandoned")
	if not label_text("EncounterJournalKeystone").contains("Run abandoned") or not await shot("challenge-abandoned-overview"):
		stop("Journal did not follow the actual abandoned-run snapshot")
		return
	phase("passed")
	print("AG_CHALLENGE_PASS skin=%s sourceFont246779/authenticatedSlotStart/realRunningClock/bosses/Abandon" % client.account_state().ui_skin)
	quit(0)
