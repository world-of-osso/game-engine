extends "res://tests/adventure_instance_loot.gd"
# Retail EJ_DIFFICULTIES order on genuine instance menus; no catalog injection.

func verify_menu_order(instance: int, tier: int, ids: Array) -> bool:
	# The selected bottom tab is disabled; switch away before opening another grid.
	if client.adventure_guide_state().tab == "Raids" and not await click("EncounterJournalDungeonTab"):
		return false
	if not await open_loot_instance("EncounterJournalRaidTab", tier, instance) or not await click("JournalDifficultyDropdown"):
		return false
	var previous := -INF
	for id in ids:
		var row := control("JournalDifficulty%s" % id)
		if row == null or not row.is_visible_in_tree() or row.get_global_rect().position.y <= previous:
			stop("Retail difficulty order mismatch for instance%s id%s" % [instance,id])
			return false
		previous = row.get_global_rect().position.y
	if instance == 741 and control("JournalDifficulty18") != null:
		stop("Non-journal Event18 leaked into Molten Core menu")
		return false
	if not await shot("difficulty-order-%s" % instance):
		return false
	if not await click("JournalDifficulty%s" % ids[0]) or client.adventure_guide_state().difficulty != ids[0]:
		stop("Physical first source difficulty selection failed")
		return false
	print("JOURNAL_DIFFICULTY_ORDER instance=%s ids=%s" % [instance,ids])
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
	if not await verify_menu_order(72, 73, [3,5,4,6]):
		return
	if not await verify_menu_order(1180, 396, [17,14,15,16]):
		return
	if not await verify_menu_order(741, 68, [9]):
		return
	print("JOURNAL_DIFFICULTY_ORDER_PASS skin=" + str(client.account_state().ui_skin))
	quit(0)
