extends "res://tests/adventure_instance_loot.gd"
# Exact source glyph/header geometry on genuine journal difficulties, no lockout injection.

func select_title_difficulty(id: int, glyph: bool) -> bool:
	if not await click("JournalDifficultyDropdown") or not await click("JournalDifficulty%s" % id):
		return false
	var icon := control("EncounterJournalDifficultyIcon")
	var title := control("EncounterJournalLeftTitle") as Label
	var bounds := control("EncounterJournalLeftTitleBounds").get_global_rect()
	var journal := control("EncounterJournal").get_global_rect()
	if title == null or title.text != "Ny'alotha, the Waking City" or title.get_theme_font_size("font_size") != 16 or title.get_line_count() != 1:
		stop("Source raid title text/font mismatch")
		return false
	if glyph:
		if icon == null or icon.get_global_rect().size.distance_to(Vector2(20,20))>1.0:
			stop("Source published20px difficulty glyph missing")
			return false
		var rectangle := icon.get_global_rect()
		if rectangle.position.distance_to(journal.position + Vector2(67,80))>1.0 or absf(title.get_global_rect().position.x-rectangle.end.x-3.0)>1.0 or absf(title.get_global_rect().get_center().y-rectangle.get_center().y)>1.0:
			stop("Source raid title glyph/right3/centerY anchor mismatch")
			return false
		if not await verify_painted_icon("EncounterJournalDifficultyIcon"):
			return false
	else:
		if icon != null or bounds.position.distance_to(journal.position+Vector2(72,84))>1.0:
			stop("Plain raid title did not restore source65,20 anchor without glyph")
			return false
	print("RAID_TITLE difficulty=%s glyph=%s title=%s bounds=%s" % [id,glyph,title.get_global_rect(),bounds])
	return await move_to("JournalDifficultyDropdown")

func run() -> void:
	var server := OS.get_environment("ADVENTURE_GUIDE_SERVER")
	var account := OS.get_environment("ADVENTURE_GUIDE_ACCOUNT")
	shots = OS.get_environment("ADVENTURE_GUIDE_SHOTS")
	if server != "127.0.0.1:5771" or not account.begins_with("fb_adventure4") or shots.is_empty():
		stop("Owned private realm/account and screenshots required")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var deadline := Time.get_ticks_msec()+180000
	while client.account_state().assets_starting and Time.get_ticks_msec()<deadline:
		await process_frame
	var error: String = client.connect_account(server,account,OS.get_environment("ADVENTURE_GUIDE_PASSWORD"),false)
	if not error.is_empty():
		stop(error)
		return
	if not await enter_world_when_ready():
		return
	if client.account_state().ui_skin != OS.get_environment("ADVENTURE_GUIDE_EXPECTED_SKIN"):
		stop("Actual skin does not match capture receipt")
		return
	await journal_key()
	deadline = Time.get_ticks_msec()+90000
	while not client.adventure_guide_state().loaded and Time.get_ticks_msec()<deadline:
		await process_frame
	if not await open_loot_instance("EncounterJournalRaidTab",396,1180):
		return
	for id in [15,16,14]:
		if not await select_title_difficulty(id,id!=14) or not await shot("raid-title-%s" % id):
			return
	if not await select_title_difficulty(17,false):
		return
	print("RAID_TITLE_PASS skin=%s actualHeroic/Mythicglyph/plainNormal/RaidFinder/lefttitle16/right3/centerY" % client.account_state().ui_skin)
	quit(0)
