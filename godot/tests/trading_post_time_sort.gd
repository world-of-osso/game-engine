extends "res://tests/trading_post_monthly_clock.gd"
# Seven checked cases. Real header/menu input only; no invented per-item times or purchases.
var sort_before: Dictionary
var name_prefix: Array
var sort_position: Vector3

func fail(message: String) -> void:
	completion.fail("Trading Post Time sort: " + message)

func run_monthly() -> bool:
	if not completion.require(phase == "time-sort" and skin in ["modern", "forever"], "explicit Time-sort phase/skin required"): return false
	if not completion.step(await initialize_monthly_client(), "initialize native client"): return false
	var account := OS.get_environment("TP_ACCOUNT")
	if not completion.require(account.begins_with("fb_tpmonth_"), "owned monthly cohort required"): return false
	var error: String = client.connect_account(server, account, OS.get_environment("TP_PASSWORD"), false)
	if not completion.require(error.is_empty(), "private account connection: " + error): return false
	if not completion.step(await wait_until(func(): return control("CharCard_0") != null, "character select", 180000), "character select"): return false
	if not completion.step(await checked_click("CharCard_0"), "select character"): return false
	if not completion.step(await wait_until(func(): return control("EnterWorld") != null, "EnterWorld", 180000), "EnterWorld ready"): return false
	if not completion.step(await checked_click("EnterWorld"), "enter world"): return false
	if not completion.step(await wait_until(func(): return client.account_state().screen == "InWorld" and control("MinimapLauncherButton") != null, "world", 180000), "world ready"): return false
	var expected := "Modern" if skin == "modern" else "Forever"
	if not completion.step(await select_monthly_skin(expected), "native skin selection"): return false
	printerr("CASE_STARTED trading_post_time_sort")
	if not completion.step(signal_host("world-ready"), "signal native world readiness"): return false
	if not completion.step(await wait_until(func(): return FileAccess.file_exists(receipts.path_join("clock-ready.json")), "host clock acknowledgment", 120000), "clock acknowledged"): return false
	if not completion.step(await open_vendor(), "authentic vendor Query"): return false
	return await time_sort_cases()

func capture_case(name: String) -> bool:
	if not completion.require(Vector3(client.account_state().local_player_position).is_equal_approx(sort_position), "sampled world position unchanged"): return false
	return await super.capture_case(name)

func assert_sort(field: String, ascending: bool, expected_rows: Array) -> bool:
	var state: Dictionary = client.trading_post_state()
	if not completion.require(str(state.sort) == field and bool(state.ascending) == ascending, "native sort field/direction"): return false
	return completion.require(Array(state.rows) == expected_rows, "observed first12 prefix matches ordinary equal-time name ties")

func filter_click(name: String) -> bool:
	if control("PerksProgramFilterMenu") == null:
		if not completion.step(await checked_click("PerksProgramFilter"), "open filter menu"): return false
		if not completion.step(await wait_until(func(): return control("PerksProgramFilterMenu") != null, "filter menu"), "filter visible"): return false
	return completion.step(await checked_click(name), "filter control " + name)

func close_filter() -> bool:
	if control("PerksProgramFilterMenu") != null:
		if not completion.step(await checked_click("PerksProgramFilter"), "close filter menu"): return false
	return completion.step(await wait_until(func(): return control("PerksProgramFilterMenu") == null, "filter closed"), "filter dismissed")

func open_sort_menu() -> bool:
	if not completion.step(await filter_click("PerksProgramFilterSort"), "Sort By submenu"): return false
	return completion.step(await wait_until(func(): return control("PerksProgramFilterSortTime") != null, "Time radio"), "sort menu visible")

func time_sort_cases() -> bool:
	sort_before = client.trading_post_state().duplicate(true)
	sort_position = client.account_state().local_player_position
	if not completion.require(sort_before.filter_collected and sort_before.filter_uncollected and sort_before.cart.is_empty(), "unfiltered empty-cart baseline"): return false
	if not completion.require(int(sort_before.items) > 12 and int(sort_before.frozen) > 0, "preserved genuine reservation plus multiple ordinary pages"): return false
	if not completion.require(str(sort_before.sort) == "price" and not sort_before.ascending, "default Price descending"): return false
	if not completion.require(not control("PerksProgramTimeSortButton").disabled, "live monthly Time header enabled"): return false
	# Every account in this read-only cohort has no purchase history. Name is the
	# accepted existing tie-break oracle; record only the first12 actually exposed rows.
	if not completion.step(await checked_click("PerksProgramNameSortButton"), "derive ordinary name prefix"): return false
	name_prefix = Array(client.trading_post_state().rows)
	if not completion.require(name_prefix.size() == 12 and name_prefix != Array(sort_before.rows), "fixture has a real Price-to-name reorder"): return false
	if not completion.step(await checked_click("PerksProgramPriceSortButton"), "restore baseline Price"): return false
	if not completion.step(assert_sort("price", false, Array(sort_before.rows)), "baseline restored"): return false
	if not completion.step(await capture_case("price-baseline"), "baseline case"): return false

	if not completion.step(await checked_click("PerksProgramTimeSortButton"), "Time header ascending"): return false
	if not completion.step(assert_sort("time", true, name_prefix), "Time ascending prefix"): return false
	if not completion.require(control("PerksProgramTimeSortArrow") != null, "native Time arrow shown"): return false
	if not completion.step(await capture_case("time-ascending"), "ascending case"): return false

	if not completion.step(await checked_click("PerksProgramTimeSortButton"), "Time header descending"): return false
	if not completion.step(assert_sort("time", false, name_prefix), "descending keeps ascending name ties"): return false
	if not completion.step(await capture_case("time-descending-name-ties"), "descending case"): return false

	if not completion.step(await checked_click("PerksProgramPriceSortButton"), "Price before menu selection"): return false
	if not completion.step(await open_sort_menu(), "open native sort submenu"): return false
	if not completion.require(not control("PerksProgramFilterSortTime").disabled, "native Time radio enabled"): return false
	if not completion.step(await checked_click("PerksProgramFilterSortTime"), "Time radio input"): return false
	if not completion.step(assert_sort("time", true, name_prefix), "menu Time default ascending"): return false
	if not completion.step(await capture_case("time-menu-selection"), "menu case"): return false
	if not completion.step(await close_filter(), "dismiss sort menu"): return false

	if not completion.step(await filter_click("PerksProgramFilterCollected"), "filter rebuild"): return false
	if not completion.step(assert_sort("price", false, Array(sort_before.rows)), "filter resets Time to Price"): return false
	if not completion.step(await filter_click("PerksProgramFilterCollected"), "restore collected filter"): return false
	if not completion.step(await close_filter(), "dismiss rebuilt filter"): return false
	if not completion.step(await capture_case("provider-reset-price"), "provider rebuild case"): return false

	if not completion.step(await filter_click("PerksProgramFilterUncollected"), "empty ordinary provider"): return false
	if not completion.require(client.trading_post_state().rows.is_empty(), "no purchased ordinary rows in authentic cohort"): return false
	if not completion.step(await close_filter(), "dismiss empty filter"): return false
	if not completion.require(control("PerksProgramTimeSortButton").disabled, "empty ordinary Time header disabled"): return false
	if not completion.step(await checked_click("PerksProgramTimeSortButton"), "disabled header native click"): return false
	if not completion.step(assert_sort("price", false, []), "disabled header cannot sort"): return false
	if not completion.step(await open_sort_menu(), "empty-provider Time radio"): return false
	if not completion.require(control("PerksProgramFilterSortTime").disabled, "empty-provider Time radio disabled"): return false
	if not completion.step(await capture_case("frozen-only-disabled-time"), "empty provider case"): return false
	if not completion.step(await close_filter(), "dismiss empty sort menu"): return false
	if not completion.step(await filter_click("PerksProgramFilterUncollected"), "restore ordinary provider"): return false
	if not completion.step(await close_filter(), "dismiss restored filter"): return false

	var restored: Dictionary = client.trading_post_state()
	for field in ["tender", "points", "frozen", "items", "selected", "cart", "filter_collected", "filter_uncollected", "filter_usable", "types", "rows", "hide_player_preview"]:
		if not completion.require(restored[field] == sort_before[field], "conserved " + field): return false
	if not completion.require(not restored.pending and str(restored.error).is_empty(), "no request pending/error"): return false
	return await leave_case("leave-restores-hud")
