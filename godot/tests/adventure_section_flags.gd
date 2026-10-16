extends "res://tests/adventure_instance_loot.gd"
# Actual70391 journal sections, physical navigation/hover; no catalog injection.

func reveal_flag(name: String) -> bool:
	for attempt in 40:
		var frame := control(name)
		var list := control("EncounterJournalContent")
		if frame != null and list.get_global_rect().encloses(frame.get_global_rect()):
			return true
		if not await wheel("EncounterJournalContent", MOUSE_BUTTON_WHEEL_DOWN):
			return false
	stop("Source flag not revealed: " + name)
	return false

func check_flag_header(section: int, indices: Array, titles: Array) -> bool:
	var prefix := "JournalSection%s" % section
	if not await reveal_flag(prefix + "Flag%s" % indices[-1]):
		return false
	var header := control(prefix).get_global_rect()
	var scale := header.size.y / 24.0
	var offsets := [27.0, 49.0, 71.0, 84.0]
	for slot in indices.size():
		var name := prefix + "Flag%s" % indices[slot]
		var owner := control(name)
		var art := control(name + "Art")
		if owner == null or art == null:
			stop("Source icon/hover owner missing " + name)
			return false
		var expected := Vector2(header.end.x - offsets[slot]*scale, header.get_center().y - 16.0*scale)
		if owner.get_global_rect().position.distance_to(expected) > 1.0 or owner.size.distance_to(Vector2(32,32)*scale) > 1.0 or art.size.distance_to(Vector2(20,20)*scale) > 1.0:
			stop("Source32px flag anchor/20px atlas mismatch " + name)
			return false
		if not await verify_painted_icon(name + "Art") or not await move_to(name):
			return false
		if client.tooltip_state().get("title", "") != titles[slot]:
			stop("Localized flag tooltip mismatch " + name + " " + str(client.tooltip_state()))
			return false
		print("JOURNAL_FLAG ", name, " title=", titles[slot], " owner=", owner.get_global_rect(), " atlas=", art.get_global_rect())
		if not await shot("section-%s-flag-%s" % [section, indices[slot]]):
			return false
	var last := control(prefix + "Flag%s" % indices[-1]).get_global_rect()
	if absf(control(prefix + "Name").get_global_rect().end.x - last.position.x + 5.0*scale) > 1.0:
		stop("Header title did not reserve source5px before last flag")
		return false
	return true

func run() -> void:
	var server := OS.get_environment("ADVENTURE_GUIDE_SERVER")
	var account := OS.get_environment("ADVENTURE_GUIDE_ACCOUNT")
	shots = OS.get_environment("ADVENTURE_GUIDE_SHOTS")
	if server != "127.0.0.1:5771" or not account.begins_with("fb_") or shots.is_empty():
		stop("Owned realm/account/screenshots required")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var deadline := Time.get_ticks_msec()+180000
	while client.account_state().assets_starting and Time.get_ticks_msec()<deadline:
		await process_frame
	var error: String = client.connect_account(server, account, OS.get_environment("ADVENTURE_GUIDE_PASSWORD"), false)
	if not error.is_empty():
		stop(error)
		return
	if not await enter_world_when_ready():
		return
	if client.account_state().ui_skin != OS.get_environment("ADVENTURE_GUIDE_EXPECTED_SKIN"):
		stop("Actual skin differs from bound profile")
		return
	await journal_key()
	deadline = Time.get_ticks_msec()+90000
	while not client.adventure_guide_state().loaded and Time.get_ticks_msec()<deadline:
		await process_frame
	if not await open_loot_instance("EncounterJournalDungeonTab",73,67):
		return
	if client.adventure_guide_state().difficulty != 2:
		if not await click("JournalDifficultyDropdown") or not await click("JournalDifficulty2"):
			return
	if not await click("JournalBoss112"):
		return
	# Ozruk has no separate Overview: the source's first tab shows his ability details.
	if not await check_flag_header(2410,[3,5,7],["Heroic Difficulty","Important","Magic Effect"]):
		return
	print("JOURNAL_FLAGS_PASS skin=%s actualOzruk2410 sourceart/threeflags/title/ownerhover" % client.account_state().ui_skin)
	quit(0)
