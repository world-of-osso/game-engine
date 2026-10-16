extends "res://tests/adventure_instance_loot.gd"
# Real catalog + physical text/key/pointer events, no search-result injection.

func _initialize() -> void:
	call_deferred("watchdog")
	call_deferred("run")

func watchdog() -> void:
	await create_timer(600.0).timeout
	stop("Journal search exceeded600s")

func press_key(code: Key) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = code
		event.physical_keycode = code
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await settle()

func type_word(value: String, codes: Array) -> bool:
	if not await click("EncounterJournalSearchBox"):
		return false
	for index in codes.size():
		for pressed in [true, false]:
			var event := InputEventKey.new()
			event.keycode = codes[index]
			event.physical_keycode = codes[index]
			event.unicode = value.unicode_at(index) if pressed else 0
			event.pressed = pressed
			root.push_input(event, true)
			await process_frame
		await settle()
	if label_text("EncounterJournalSearchBox") != value:
		stop("Physical search text differs: " + label_text("EncounterJournalSearchBox"))
		return false
	return true

func shown(name: String) -> bool:
	var node := control(name)
	return node != null and node.is_visible_in_tree()

func verify_preview() -> bool:
	if not shown("EncounterJournalSearchPreview"):
		stop("Source preview missing after physical text; old full overlay=" + str(shown("EncounterJournalSearchResults")))
		return false
	var search := control("EncounterJournalSearchBox").get_global_rect()
	var preview := control("EncounterJournalSearchPreview").get_global_rect()
	if preview.position.distance_to(Vector2(search.position.x, search.end.y)) > 1.0 or absf(preview.size.x-search.size.x) > 1.0:
		stop("Source preview must be anchored below the210px search field")
		return false
	for index in 5:
		var row := control("JournalSearchPreview%s" % index)
		var height := 24.0 if index == 0 else 27.0
		if row == null or not row.is_visible_in_tree() or row.size.distance_to(Vector2(210,height)) > 1.0:
			stop("Source first24/subsequent27 preview row missing " + str(index))
			return false
	if shown("JournalSearchPreview5") or not shown("JournalSearchAll"):
		stop("Source five-row limit/show-all branch incorrect")
		return false
	return await shot("search-preview-five")

func verify_full_results() -> bool:
	await press_key(KEY_DOWN)
	if not shown("JournalSearchPreview1Selected"):
		stop("Arrow down did not select second preview row")
		return false
	await press_key(KEY_UP)
	await press_key(KEY_UP)
	if not shown("JournalSearchAllSelected"):
		stop("Arrow up did not wrap first preview to Show All")
		return false
	await press_key(KEY_ENTER)
	if shown("EncounterJournalSearchPreview") or not shown("EncounterJournalSearchResults"):
		stop("Enter on Show All did not replace preview with full results")
		return false
	var popup := control("EncounterJournalSearchResults").get_global_rect()
	var journal := control("EncounterJournal").get_global_rect()
	if popup.size.distance_to(Vector2(600,400)) > 1.0 or popup.position.distance_to(journal.position+Vector2(100,89)) > 1.0:
		stop("Source BottomPopup600x400/bottom+7 anchors changed")
		return false
	var row := control("JournalSearchResult0")
	if row == null or row.size.distance_to(Vector2(575,49)) > 1.0 or label_text("JournalSearchResult0Name").is_empty() or label_text("JournalSearchResult0Type").is_empty():
		stop("Source full49px name/type/path row missing")
		return false
	# EJ_GetSearchDisplay Lua2457–2471 leaves instance path nil, unlike encounter path.
	if label_text("JournalSearchResult0Name") != "Darkflame Cleft" or not label_text("JournalSearchResult0Path").is_empty() or label_text("JournalSearchResult2Name") != "Ambassador Flamelash" or label_text("JournalSearchResult2Path") != "Blackrock Depths":
		stop("Actual instance-empty/encounter breadcrumb contract differs")
		return false
	if not await verify_painted_icon("JournalSearchResult0Icon") or not await shot("search-full-results"):
		return false
	var before := row.get_global_rect().position.y
	if not await wheel("EncounterJournalSearchList", MOUSE_BUTTON_WHEEL_DOWN):
		return false
	if row.get_global_rect().position.y >= before or not await shot("search-full-scrolled"):
		stop("Full search wheel did not move actual source rows")
		return false
	if not await click("EncounterJournalSearchResultsClose"):
		return false
	if shown("EncounterJournalSearchResults") or shown("EncounterJournalSearchPreview") or label_text("EncounterJournalSearchBox") != "flame":
		stop("Closing full search lost query or retained popup")
		return false
	return true

func verify_section_selection() -> bool:
	if not await click("EncounterJournalSearchBox"):
		return false
	if not shown("EncounterJournalSearchPreview"):
		stop("Refocus did not restore source preview")
		return false
	await press_key(KEY_ESCAPE)
	if shown("EncounterJournalSearchPreview") or label_text("EncounterJournalSearchBox") != "flame":
		stop("Physical blur did not hide preview and retain query")
		return false
	if not await click("EncounterJournalSearchBox") or not await click("EncounterJournalSearchBoxClearButton"):
		return false
	if not await type_word("Paralyze", [KEY_P,KEY_A,KEY_R,KEY_A,KEY_L,KEY_Y,KEY_Z,KEY_E]):
		return false
	if label_text("JournalSearchPreview0Name") != "Paralyze" or not await click("JournalSearchPreview0"):
		stop("Actual first Paralyze section result absent")
		return false
	var state: Dictionary = client.adventure_guide_state()
	var header := control("JournalSection2410")
	var viewport := control("EncounterJournalContent")
	if state.encounter != 112 or state.difficulty != 2 or header == null or not viewport.get_global_rect().encloses(header.get_global_rect()):
		stop("Real section search did not select Heroic Ozruk and reveal2410: " + str(state))
		return false
	if label_text("EncounterJournalSearchBox") != "Paralyze":
		stop("Search selection incorrectly cleared retained source query")
		return false
	if shown("EncounterJournalSearchPreview") or shown("EncounterJournalSearchResults"):
		stop("Search selection retained a results overlay")
		return false
	return await shot("search-section-selected")

func run() -> void:
	var server := OS.get_environment("ADVENTURE_GUIDE_SERVER")
	var account := OS.get_environment("ADVENTURE_GUIDE_ACCOUNT")
	shots = OS.get_environment("ADVENTURE_GUIDE_SHOTS")
	if server != "127.0.0.1:5771" or not account.begins_with("fb_") or shots.is_empty():
		stop("Owned private realm/account/shots required")
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
		stop("Actual skin differs from bound search profile")
		return
	await journal_key()
	deadline = Time.get_ticks_msec()+90000
	while not client.adventure_guide_state().loaded and Time.get_ticks_msec()<deadline:
		await process_frame
	if client.adventure_guide_state().tab != "Dungeons" and not await click("EncounterJournalDungeonTab"):
		return
	if not await type_word("fl", [KEY_F,KEY_L]):
		return
	if shown("EncounterJournalSearchPreview") or shown("EncounterJournalSearchResults"):
		stop("Two-character physical query incorrectly shows results")
		return
	await press_key(KEY_ENTER)
	if not control("EncounterJournalSearchBox").has_focus():
		stop("Enter on invalid short query incorrectly released source focus")
		return
	if not await click("EncounterJournalSearchBoxClearButton"):
		return
	if not await type_word("flame", [KEY_F,KEY_L,KEY_A,KEY_M,KEY_E]) or not await verify_preview() or not await verify_full_results() or not await verify_section_selection():
		return
	print("JOURNAL_SEARCH_PASS skin=%s actualcatalog preview/arrowwrap/Enter/fullpopup/close/sectionfocus" % client.account_state().ui_skin)
	quit(0)
