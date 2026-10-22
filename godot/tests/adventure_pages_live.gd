extends "res://tests/adventure_search.gd"
# Existing authored source catalog + native input; private5996 only, no UI/catalog injection.

func wait_journal(predicate: Callable, reason: String) -> bool:
	var deadline := Time.get_ticks_msec() + 90000
	while Time.get_ticks_msec() < deadline:
		if predicate.call(client.adventure_guide_state()):
			await settle()
			return true
		await process_frame
	stop(reason + ": " + str(client.adventure_guide_state()))
	return false

func select_named_boss(value: String, codes: Array, encounter: int) -> bool:
	if not label_text("EncounterJournalSearchBox").is_empty() and not await click("EncounterJournalSearchBoxClearButton"):
		return false
	if not await type_word(value, codes) or not await click("JournalSearchPreview0"):
		return false
	return await wait_journal(func(state): return state.encounter == encounter, "Authored boss search did not select " + value)

func choose_spec(id: int) -> bool:
	var error: String = client.set_specialization(id)
	if not error.is_empty():
		stop(error)
		return false
	var deadline := Time.get_ticks_msec() + 30000
	while client.spells_state().spec != id and Time.get_ticks_msec() < deadline:
		await process_frame
	if client.spells_state().spec != id:
		stop("Private realm specialization ACK absent: " + str(id))
		return false
	await settle()
	return true

func verify_role_refresh() -> bool:
	if not await choose_spec(70) or not await select_named_boss("Kurog", [KEY_K,KEY_U,KEY_R,KEY_O,KEY_G], 2491):
		return false
	if not await click("EncounterJournalPage0"):
		return false
	if label_text("JournalSection25033ExpandedIcon") != "−" or label_text("JournalSection25035ExpandedIcon") != "+":
		stop("Authentic Kurog Damage/Tank overview defaults differ")
		return false
	if not await shot("pages-kurog-damage-overview"):
		return false
	await journal_key()
	if not await choose_spec(66):
		return false
	await journal_key()
	if not await wait_journal(func(state): return state.role == 1, "New Tank role not observed"):
		return false
	if label_text("JournalSection25035ExpandedIcon") != "−" or label_text("JournalSection25033ExpandedIcon") != "+":
		stop("Reopened Guide sorted Tank first but retained Damage disclosure")
		return false
	if not await shot("pages-kurog-tank-overview"):
		return false
	if not await click("EncounterJournalPage1"):
		return false
	if not shown("JournalSection25036") or not await shot("pages-kurog-abilities"):
		stop("Authentic Kurog ability tree missing")
		return false
	if not await click("JournalSection25036") or not await click("JournalSection25036"):
		return false
	print("AG_PAGES_ROLE_PASS real70-to66-ACK/reopen/role-headers/ability-disclosure")
	return true

func verify_link_menus_and_loot() -> bool:
	if not await select_named_boss("Tindral", [KEY_T,KEY_I,KEY_N,KEY_D,KEY_R,KEY_A,KEY_L], 2565):
		return false
	if not await click("EncounterJournalPage2") or not await click("EncounterJournalNavBarOverflowButton"):
		return false
	if not shown("EncounterJournalNavBarOverflowMenu") or not await shot("pages-authored-overflow-open"):
		stop("Authentic longest breadcrumb did not open shared overflow")
		return false
	if not await click("JournalClassFilter") or not await click("JournalClassMenu"):
		return false
	if not shown("EncounterJournalClassMenu") or not await shot("pages-class-submenu-open"):
		return false
	if not await select_named_boss("Paralyze", [KEY_P,KEY_A,KEY_R,KEY_A,KEY_L,KEY_Y,KEY_Z,KEY_E], 112):
		return false
	if shown("EncounterJournalNavBarOverflowMenu") or shown("EncounterJournalClassMenu"):
		stop("Encounter link retained old navigation overlay")
		return false
	if client.adventure_guide_state().difficulty != 2 or not await click("EncounterJournalPage2"):
		stop("Paralyze search did not retain its authored Heroic mode")
		return false
	if client.adventure_guide_state().loot_count != 6 or label_text("EncounterJournalSearchBox") != "Paralyze":
		stop("Retained section search incorrectly hides Heroic Ozruk's six drops")
		return false
	if not await click("JournalClassFilter"):
		return false
	if shown("EncounterJournalClassMenu"):
		stop("New boss class filter reopened old submenu")
		return false
	if not await click("JournalClassMenu") or not await click("JournalClass2") or not await click("JournalClassFilter") or not await click("JournalSpec70"):
		return false
	if client.adventure_guide_state().loot_count != 3:
		stop("Authored Heroic Ozruk Paladin/Retribution drops must be three")
		return false
	if not await choose_slot(2) or client.adventure_guide_state().loot_count != 1 or not await shot("pages-heroic-ret-neck"):
		return false
	if not await choose_slot(0) or not await click("JournalDifficultyDropdown") or not await click("JournalDifficulty24"):
		return false
	if client.adventure_guide_state().difficulty != 24 or client.adventure_guide_state().loot_count <= 0 or not await shot("pages-timewalking-ret-loot"):
		stop("Authored Timewalking loot missing")
		return false
	if not await click("EncounterJournalPage3") or not await expect_model(382, 36475, "Ozruk"):
		return false
	if not await verify_rendered_portrait("JournalCreature382Portrait", 36475) or not await shot("pages-ozruk-model"):
		return false
	print("AG_PAGES_LINK_LOOT_PASS authoredTindralOverflow/classSubmenu/Paralyze-retained/role-class-slot/difficulty/model")
	return true

func verify_search_surface() -> bool:
	if not await click("EncounterJournalDungeonTab"):
		return false
	if not await type_word("fl", [KEY_F,KEY_L]):
		return false
	await press_key(KEY_ENTER)
	if not control("EncounterJournalSearchBox").has_focus() or shown("EncounterJournalSearchPreview"):
		stop("Short query must retain edit focus without preview")
		return false
	if not await click("EncounterJournalSearchBoxClearButton"):
		return false
	return await type_word("flame", [KEY_F,KEY_L,KEY_A,KEY_M,KEY_E]) and await verify_preview() and await verify_full_results() and await verify_section_selection()

func connect_owned_realm() -> bool:
	var server := OS.get_environment("ADVENTURE_GUIDE_SERVER")
	var account := OS.get_environment("ADVENTURE_GUIDE_ACCOUNT")
	shots = OS.get_environment("ADVENTURE_GUIDE_SHOTS")
	if server != "127.0.0.1:5996" or not account.begins_with("fb_agpages") or not shots.begins_with("/"):
		stop("Owned private5996/fb_agpages account and absolute screenshot directory required")
		return false
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var deadline := Time.get_ticks_msec() + 180000
	while client.account_state().assets_starting and Time.get_ticks_msec() < deadline:
		await process_frame
	var error: String = client.connect_account(server, account, OS.get_environment("ADVENTURE_GUIDE_PASSWORD"), false)
	if not error.is_empty():
		stop(error)
		return false
	if not await enter_world_when_ready():
		return false
	if client.account_state().ui_skin != OS.get_environment("ADVENTURE_GUIDE_EXPECTED_SKIN"):
		stop("Authenticated character skin differs from receipt")
		return false
	return true

func run() -> void:
	if not await connect_owned_realm():
		return
	if not await click("EJMicroButton") or not await wait_journal(func(state): return state.loaded and state.visible, "Micro menu did not open Guide"):
		return
	await journal_key()
	if client.adventure_guide_state().visible:
		stop("J did not close micro-opened Guide")
		return
	await journal_key()
	if not await wait_journal(func(state): return state.visible, "J did not reopen Guide"):
		return
	if OS.get_environment("ADVENTURE_GUIDE_PHASE") == "search":
		if not await verify_search_surface():
			return
	else:
		if not await verify_role_refresh() or not await verify_link_menus_and_loot():
			return
	print("AG_PAGES_PASS skin=%s phase=%s" % [client.account_state().ui_skin, OS.get_environment("ADVENTURE_GUIDE_PHASE")])
	quit(0)
