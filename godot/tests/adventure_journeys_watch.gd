extends "res://tests/adventure_journeys_layout.gd"
# Physical input only: genuine ungated Covenant55 -> Faction2836 ACK/HUD/reopen/clear.

func await_watch(expected: int) -> bool:
	var deadline := Time.get_ticks_msec() + 15000
	while Time.get_ticks_msec() < deadline:
		if client.adventure_guide_state().watched_faction == expected:
			await settle()
			return true
		await process_frame
	stop("Watch acknowledgement absent; expected=" + str(expected))
	return false

func reveal_card() -> bool:
	var card := control("JourneyCard55")
	var list := control("EncounterJournalJourneysList")
	if card == null or list == null:
		stop("Published faction card/list missing")
		return false
	if not list.get_global_rect().encloses(card.get_global_rect()):
		var thumb := control("EncounterJournalJourneysListScrollThumb")
		var track := control("EncounterJournalJourneysListScrollTrack")
		if thumb == null or track == null:
			stop("Journeys list native scroll thumb/track absent")
			return false
		await move_to("EncounterJournalJourneysListScrollThumb")
		await pointer_button(thumb.get_global_rect().get_center(), true)
		var bottom := track.get_global_rect().get_center()
		bottom.y = track.get_global_rect().end.y - 1.0
		var motion := InputEventMouseMotion.new()
		motion.position = bottom
		motion.global_position = bottom
		root.push_input(motion, true)
		await settle()
		await pointer_button(bottom, false)
		await settle()
	if not list.get_global_rect().encloses(control("JourneyCard55").get_global_rect()):
		stop("Physical thumb drag did not reveal ungated card55")
		return false
	return await move_to("JourneyCard55")

func enter_world_when_ready() -> bool:
	if not await wait_screen("CharacterSelect"):
		return false
	var deadline := Time.get_ticks_msec() + 180000
	while Time.get_ticks_msec() < deadline:
		var enter := control("EnterWorld")
		if enter != null and enter.is_visible_in_tree():
			return await click("EnterWorld") and await wait_screen("InWorld")
		await process_frame
	stop("Character preview did not reveal EnterWorld")
	return false

func run() -> void:
	var server := OS.get_environment("ADVENTURE_GUIDE_SERVER")
	var account := OS.get_environment("ADVENTURE_GUIDE_ACCOUNT")
	var expected_count := int(OS.get_environment("ADVENTURE_GUIDE_FACTION_COUNT"))
	shots = OS.get_environment("ADVENTURE_GUIDE_SHOTS")
	if server != "127.0.0.1:5771" or not account.begins_with("fb_") or shots.is_empty() or expected_count <= 0:
		stop("Explicit private5771/fb_* account, published count and shots required")
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
	await journal_key()
	deadline = Time.get_ticks_msec() + 90000
	while (not client.adventure_guide_state().loaded or client.adventure_guide_state().renown_faction_count < 0) and Time.get_ticks_msec() < deadline:
		await process_frame
	if client.adventure_guide_state().renown_faction_count != expected_count:
		stop("Published faction catalog not ready")
		return
	if not await click("EncounterJournalJourneysTab"):
		return
	if client.adventure_guide_state().journey_expansion != 11:
		if not await click("JournalJourneysExpansion") or not await click("JournalJourneysExpansion11"):
			return
	if not await reveal_card():
		return
	if client.adventure_guide_state().watched_faction == 2836:
		if not await click("JourneyCard55WatchCheckbox") or not await await_watch(-1):
			return
		if not await move_to("JourneyCard55"):
			return
	if not await click("JourneyCard55WatchCheckbox") or not await await_watch(2836):
		return
	var background := control("ReputationStatusBarBackground")
	if background == null or not background.is_visible_in_tree():
		stop("ACK did not paint normalized reputation HUD")
		return
	if not await move_to("ReputationStatusBarBackground"):
		return
	if label_text("ReputationStatusBarText") != "The Labyrinth of Kindo'jan 0 / 2100":
		stop("Authoritative normalized HUD text missing")
		return
	if not await shot("journeys-watch-ack-hud"):
		return
	await journal_key()
	await journal_key()
	if not await await_watch(2836) or not await reveal_card():
		return
	if not await shot("journeys-watch-reopened"):
		return
	if not await click("JourneyCard55WatchCheckbox") or not await await_watch(-1):
		return
	var bar := control("ReputationStatusBar")
	if bar != null and bar.is_visible_in_tree():
		stop("Clear ACK retained watched reputation HUD")
		return
	if not await shot("journeys-watch-cleared"):
		return
	print("JOURNEYS_WATCH_PASS skin=%s Covenant55/Faction2836 physicalACK/reopen/bar0of2100/clear" % client.account_state().ui_skin)
	quit(0)
