extends "res://tests/adventure_journeys_watch.gd"
# Physical navigation and authored creatures; no catalog/model injection.

func expect_model(creature: int, display: int, title: String) -> bool:
	var deadline := Time.get_ticks_msec() + 90000
	while client.adventure_guide_state().model_display != display and Time.get_ticks_msec() < deadline:
		await process_frame
	var state: Dictionary = client.adventure_guide_state()
	if state.creature != creature or state.model_display != display or label_text("EncounterJournalModelTitle") != title:
		stop("Selected creature/model/title mismatch: " + str(state))
		return false
	for frame in 30:
		await process_frame
	print("MODEL_SELECTOR_ATTACHED creature=%s display=%s title=%s" % [creature, display, title])
	return true

func reveal_instance(name: String) -> bool:
	for attempt in 40:
		var row := control(name)
		var list := control("EncounterJournalContent")
		if row != null and list.get_global_rect().has_point(row.get_global_rect().get_center()):
			return true
		if not await wheel("EncounterJournalContent", MOUSE_BUTTON_WHEEL_DOWN):
			return false
	stop("Instance not revealed: " + name)
	return false

func open_instance(tab: String, instance: int) -> bool:
	if not await click(tab) or not await click("EncounterJournalTierDropdown") or not await click("EncounterJournalTier73"):
		return false
	var name := "JournalInstance%s" % instance
	return await reveal_instance(name) and await click(name)

func verify_selection_sizes(selected: String, other: String) -> bool:
	var selected_button := control(selected) as BaseButton
	var other_button := control(other) as BaseButton
	if selected_button == null or other_button == null:
		stop("Authored creature selector missing")
		return false
	var scale := selected_button.size.x / 64.0
	if not selected_button.disabled or other_button.disabled or absf(selected_button.size.y - 61.0 * scale) > 1.0 or other_button.size.distance_to(Vector2(50,49) * scale) > 1.0:
		stop("Source selected64x61/other50x49 or disabled state mismatch")
		return false
	return true

func run() -> void:
	var server := OS.get_environment("ADVENTURE_GUIDE_SERVER")
	var account := OS.get_environment("ADVENTURE_GUIDE_ACCOUNT")
	shots = OS.get_environment("ADVENTURE_GUIDE_SHOTS")
	if server != "127.0.0.1:5771" or not account.begins_with("fb_") or shots.is_empty():
		stop("Explicit private5771/fb_* account and shots required")
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
	while not client.adventure_guide_state().loaded and Time.get_ticks_msec() < deadline:
		await process_frame
	await settle()
	if not await open_instance("EncounterJournalDungeonTab", 63):
		return
	if client.adventure_guide_state().difficulty != 1:
		if not await click("JournalDifficultyDropdown") or not await click("JournalDifficulty1"):
			return
	if not await click("JournalBoss90") or not await click("EncounterJournalPage3"):
		return
	if not await expect_model(359, 33002, "Helix Gearbreaker") or not verify_selection_sizes("JournalCreature359", "JournalCreature630"):
		return
	if not await move_to("JournalCreature630") or not await shot("model-selector-helix"):
		return
	if client.tooltip_state().get("title", "") != "Lumbering Oaf":
		stop("Authored Oaf tooltip absent")
		return
	if not await click("JournalCreature630") or not await expect_model(630, 33003, "Lumbering Oaf") or not verify_selection_sizes("JournalCreature630", "JournalCreature359"):
		return
	if not await move_to("EncounterJournalModelTitle") or not await shot("model-selector-oaf"):
		return
	if not await click("EncounterJournalPage1") or not await click("EncounterJournalPage3") or not await expect_model(359, 33002, "Helix Gearbreaker"):
		return
	if not await open_instance("EncounterJournalRaidTab", 74) or not await click("JournalBoss154") or not await click("EncounterJournalPage3"):
		return
	if not await click("JournalCreature555") or not await expect_model(555, 35232, "Nezir") or not await move_to("JournalCreature555"):
		return
	var tooltip: Dictionary = client.tooltip_state()
	if tooltip.get("title", "") != "Nezir" or not str(tooltip.get("lines", "")).contains("Lord of the North Wind"):
		stop("Authored Nezir tooltip description absent")
		return
	if not await shot("model-selector-authored-tooltip"):
		return
	print("MODEL_SELECTOR_PASS skin=%s physicalHelix/Oaf/reset/Nezir-tooltip" % client.account_state().ui_skin)
	quit(0)
