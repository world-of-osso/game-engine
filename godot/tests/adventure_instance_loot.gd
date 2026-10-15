extends "res://tests/adventure_model_selector.gd"
# Authenticated source catalog and physical input; no loot/lockout/progress injection.

func open_loot_instance(tab: String, tier: int, instance: int) -> bool:
	if not await click(tab) or not await click("EncounterJournalTierDropdown") or not await click("EncounterJournalTier%s" % tier):
		return false
	var name := "JournalInstance%s" % instance
	return await reveal_instance(name) and await click(name)

func first_visible_loot() -> String:
	var list := control("EncounterJournalContent").get_global_rect()
	for node in client.find_children("JournalLoot*", "Button", true, false):
		var suffix := str(node.name).trim_prefix("JournalLoot")
		if suffix.is_valid_int() and node.is_visible_in_tree() and list.has_point(node.get_global_rect().get_center()):
			return str(node.name)
	return ""

func verify_loot_row(name: String, height: float, boss: String) -> bool:
	var row := control(name)
	if row == null:
		stop("Authored loot row missing: " + name)
		return false
	var scale := row.get_global_rect().size.x / 321.0
	if absf(row.get_global_rect().size.y - height * scale) > 1.0:
		stop("Source loot row height mismatch: " + name)
		return false
	if not boss.is_empty() and label_text(name + "Boss") != boss:
		stop("Authored boss source mismatch: " + name)
		return false
	if boss.is_empty() and control(name + "Boss") != null:
		stop("Encounter row incorrectly retains boss label")
		return false
	return true

func verify_loot_scroll() -> bool:
	var first := first_visible_loot()
	if first.is_empty() or client.adventure_guide_state().loot_count * 64 <= 382:
		stop("Real instance loot does not provide second-page coverage")
		return false
	var before := control(first).get_global_rect().position.y
	for notch in 6:
		if not await wheel("EncounterJournalContent", MOUSE_BUTTON_WHEEL_DOWN):
			return false
	if control(first).get_global_rect().position.y >= before - 1.0 or first_visible_loot() == first:
		stop("Physical loot wheel did not reach later authored rows")
		return false
	if not await shot("instance-loot-scrolled"):
		return false
	for notch in 6:
		if not await wheel("EncounterJournalContent", MOUSE_BUTTON_WHEEL_UP):
			return false
	if absf(control(first).get_global_rect().position.y - before) > 1.0:
		stop("Physical loot wheel did not restore first page")
		return false
	return true

func probe_shift_comparison() -> bool:
	for pressed in [true, false]:
		var key := InputEventKey.new()
		key.keycode = KEY_SHIFT
		key.physical_keycode = KEY_SHIFT
		key.pressed = pressed
		root.push_input(key, true)
		await settle()
		var tooltip: Dictionary = client.tooltip_state()
		if tooltip.get("title", "") != "Buzzer Blade":
			stop("Physical Shift lost the current item tooltip")
			return false
		var shopping: Array = tooltip.get("shopping", [])
		if pressed:
			print("INSTANCE_LOOT_SHIFT actual_shopping=" + str(shopping))
			if not shopping.is_empty() and not await shot("instance-loot-shift-comparison"):
				return false
		elif not shopping.is_empty():
			stop("Shift release retained comparison tooltip")
			return false
	return true

func choose_slot(slot: int) -> bool:
	if not await click("JournalSlotFilter") or not await click("JournalSlot%s" % slot):
		return false
	if client.adventure_guide_state().slot != slot:
		stop("Physical inventory-slot filter did not apply")
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
	while (not client.adventure_guide_state().loaded or not client.adventure_guide_state().journal_item_catalog_ready) and Time.get_ticks_msec() < deadline:
		await process_frame
	if not await open_loot_instance("EncounterJournalDungeonTab", 73, 63):
		return
	if client.adventure_guide_state().difficulty != 1:
		if not await click("JournalDifficultyDropdown") or not await click("JournalDifficulty1"):
			return
	var model := control("EncounterJournalPage3") as BaseButton
	if model == null or not model.disabled or control("EncounterJournalPage1") != null:
		stop("Instance model/ability source availability mismatch")
		return
	if not await shot("instance-loot-source-tabs") or not await click("EncounterJournalPage2"):
		return
	if client.adventure_guide_state().encounter != -1 or not await verify_loot_row("JournalLoot2169", 64, "Boss: Glubtok"):
		return
	if label_text("JournalLoot2169Slot") != "One-Hand" or label_text("JournalLoot2169ArmorClass") != "Dagger":
		stop("Source slot/subclass missing or combined with invented item-level text")
		return
	if not await move_to("JournalLoot2169") or not client.tooltip_state().get("visible", false) or client.tooltip_state().get("title", "") != "Buzzer Blade":
		stop("Shared actual item tooltip missing")
		return
	if not await shot("instance-loot-item-tooltip") or not await probe_shift_comparison() or not await verify_loot_scroll():
		return
	if not await choose_slot(10):
		return
	if client.tooltip_state().get("title", "") == "Buzzer Blade":
		stop("Filtered-out item retained its tooltip")
		return
	if not await verify_loot_row("JournalLoot5195", 64, "Boss: Glubtok") or not await move_to("JournalLoot5195") or not await shot("instance-loot-slot-filter"):
		return
	if not await choose_slot(0) or not await click("JournalBoss89"):
		return
	if client.adventure_guide_state().page != "Loot":
		stop("Boss selection lost the source retained Loot tab")
		return
	if not await verify_loot_row("JournalLoot2169", 45, "") or not await shot("encounter-loot-source-row"):
		return
	if not await open_loot_instance("EncounterJournalRaidTab", 396, 1180):
		return
	if client.adventure_guide_state().page != "Loot" or client.adventure_guide_state().encounter != -1 or client.adventure_guide_state().loot_count * 64 <= 382:
		stop("Genuine Ny'alotha instance loot absent or insufficient for overflow")
		return
	var raid_item := first_visible_loot()
	if raid_item.is_empty() or not await move_to(raid_item) or not await shot("raid-instance-loot-tooltip"):
		return
	print("INSTANCE_LOOT_PASS skin=%s physicalinstance/boss/scroll/slot/tooltip/raid" % client.account_state().ui_skin)
	quit(0)
