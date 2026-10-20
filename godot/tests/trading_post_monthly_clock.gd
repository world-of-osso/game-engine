extends "res://tests/trading_post_live.gd"
# Opt-in owned5899 proof. Host aligns the server clock before the vendor Query;
# client time, membership and frozen state are never fabricated by this fixture.
const CompletionContract = preload("res://tests/completion.gd")
var completion
var reserved := -1
var time_owner := ""
var opening_tender := 0

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_completion")

func run_completion() -> void:
	completion = CompletionContract.new(self, 7)
	var result: bool = await run_monthly()
	if client != null:
		client.free()
	completion.finish(result)

func fail(message: String) -> void:
	completion.fail("Trading Post monthly clock: " + message)

func initialize_monthly_client() -> bool:
	print("TP_MONTHLY_NATIVE_PID ", OS.get_process_id())
	root.size = Vector2i(1920, 1080)
	if not completion.require(DisplayServer.screen_get_size() == Vector2i(1920, 1080), "private1920x1080 compositor required"): return false
	if not completion.require(server == "127.0.0.1:5899" and not shots.is_empty() and not receipts.is_empty(), "explicit private5899 and evidence paths required"): return false
	for directory in [shots, receipts]:
		if not completion.require(DirAccess.make_dir_recursive_absolute(directory) == OK, "evidence directory: " + directory): return false
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	return completion.step(await wait_until(func(): return not client.account_state().assets_starting, "asset startup", 180000), "asset startup")

func run_monthly() -> bool:
	if not completion.require(phase in ["rollover", "readback"] and skin in ["modern", "forever"], "explicit clock phase/skin required"): return false
	if not completion.step(await initialize_monthly_client(), "initialize native client"): return false
	var account := OS.get_environment("TP_ACCOUNT")
	if not completion.require(account.begins_with("fb_tpmonth_"), "fresh monthly fb_* cohort required"): return false
	var error: String = client.connect_account(server, account, OS.get_environment("TP_PASSWORD"), false)
	if not completion.require(error.is_empty(), "private account connection: " + error): return false
	if not completion.step(await wait_until(func(): return control("CharCard_0") != null, "character select", 180000), "character select"): return false
	if not completion.step(await checked_click("CharCard_0"), "select character"): return false
	if not completion.step(await wait_until(func(): return control("EnterWorld") != null, "EnterWorld", 180000), "EnterWorld ready"): return false
	if not completion.step(await checked_click("EnterWorld"), "enter world"): return false
	if not completion.step(await wait_until(func(): return client.account_state().screen == "InWorld" and control("MinimapLauncherButton") != null, "world", 180000), "world ready"): return false
	var expected := "Modern" if skin == "modern" else "Forever"
	if not completion.step(await select_monthly_skin(expected), "native skin selection"): return false
	printerr("CASE_STARTED trading_post_monthly_clock")
	if not completion.step(signal_host("world-ready"), "signal native world readiness"): return false
	if not completion.step(await wait_until(func(): return FileAccess.file_exists(receipts.path_join("clock-ready.json")), "host private clock alignment", 120000), "clock aligned"): return false
	if not completion.step(await open_vendor(), "authentic vendor Query"): return false
	if phase == "readback":
		return await readback_cases()
	return await rollover_cases()

func select_monthly_skin(expected: String) -> bool:
	if str(client.account_state().ui_skin) == expected: return true
	for name in ["MinimapLauncherButton", "LauncherEntryMainMenuMicroButton", "MenuBtnOptions", "OptionsTabhud", "Choiceui_layout", "Choiceui_layout0Hit" if expected == "Modern" else "Choiceui_layout1Hit", "OptionsCloseButton"]:
		if not completion.step(await checked_click(name), "skin selector " + name): return false
	if control("MenuBtnResume") != null:
		if not completion.step(await checked_click("MenuBtnResume"), "resume after skin selection"): return false
	return completion.step(await wait_until(func(): return str(client.account_state().ui_skin) == expected, "active requested skin"), "skin applied")

func checked_click(name: String) -> bool:
	if not completion.require(control(name) != null, "visible control " + name): return false
	await click(name)
	return completion.require(not completion.failed, "native click " + name)

func signal_host(name: String) -> bool:
	var file := FileAccess.open(receipts.path_join(name), FileAccess.WRITE)
	if not completion.require(file != null, "create host phase signal"): return false
	file.store_string("ready\n")
	file.close()
	return true

func capture_case(name: String) -> bool:
	await capture(name)
	if not completion.require(not completion.failed, "capture " + name): return false
	if not completion.require(FileAccess.file_exists(shots.path_join(skin + "-" + phase + "-" + name + ".png")), "native PNG saved"): return false
	return completion.case_done()

func first_time_owner() -> String:
	for node in client.find_children("PerksProgramTimeRemaining*", "Control", true, false):
		if node.is_visible_in_tree(): return str(node.name)
	return ""

func rollover_cases() -> bool:
	var state: Dictionary = client.trading_post_state()
	opening_tender = int(state.tender)
	if not completion.require(int(state.items) == 42 and int(state.frozen) == -1, "October42 offers and fresh empty frozen slot"): return false
	if not completion.step(await capture_case("october42"), "October inventory case"): return false
	time_owner = first_time_owner()
	if not completion.require(not time_owner.is_empty(), "ordinary Time column is visible"): return false
	if not completion.step(await wait_until(func(): return control(time_owner) != null and str(control(time_owner).text) == "1m", "natural one-minute threshold", 180000), "server countdown reaches one minute"): return false
	if not completion.require(control(time_owner).get_global_rect().size == Vector2(42, 38), "source Time rectangle42x38"): return false
	if not completion.require(control(time_owner).get_theme_color("font_color").is_equal_approx(Color(1, 0.1, 0.1, 1)), "under-hour Time is red"): return false
	if not completion.step(await capture_case("one-minute"), "one-minute Time case"): return false

	if not completion.step(await select_first_available(), "choose real current product"): return false
	reserved = int(client.trading_post_state().selected)
	if not completion.step(await drag_to_frozen("PerksProgramProduct%d" % reserved), "native drag to empty frozen slot"): return false
	if not completion.step(await wait_until(func(): return int(client.trading_post_state().frozen) == reserved and not client.trading_post_state().pending, "durable freeze", 30000), "freeze acknowledged"): return false
	if not completion.step(await capture_case("committed-freeze"), "frozen reservation case"): return false

	time_owner = first_time_owner()
	if not completion.require(not time_owner.is_empty(), "ordinary clock remains beside frozen slot"): return false
	if not completion.step(await wait_until(func(): return control(time_owner) != null and str(control(time_owner).text) == "0m", "natural minute countdown", 90000), "natural1m to0m"): return false
	if not completion.step(await capture_case("zero-minutes"), "minute countdown case"): return false

	if not completion.step(await wait_until(func(): return int(client.trading_post_state().items) == 1 and int(client.trading_post_state().frozen) == reserved, "natural UTC rollover replaces catalog", 100000), "automatic rollover Query replacement"): return false
	if not completion.require(first_time_owner().is_empty(), "no stale ordinary Time labels after month47 replacement"): return false
	if not completion.require(client.trading_post_state().rows.is_empty(), "old ordinary products were replaced"): return false
	if not completion.step(await capture_case("uncovered-month-frozen-only"), "rollover membership case"): return false

	await create_timer(3.0).timeout
	if not completion.require(int(client.trading_post_state().items) == 1 and int(client.trading_post_state().frozen) == reserved and int(client.trading_post_state().tender) == opening_tender, "stable post-rollover frozen/Tender state"): return false
	if not completion.step(signal_host("rollover-stable"), "server Query-count checkpoint"): return false
	if not completion.step(await capture_case("stable-frozen-only"), "stable provider case"): return false
	return await leave_case("leave-restores-hud")

func readback_cases() -> bool:
	reserved = int(OS.get_environment("TP_EXPECTED_FROZEN"))
	var before: Dictionary = client.trading_post_state().duplicate(true)
	if not completion.require(reserved > 0 and int(before.items) == 1 and int(before.frozen) == reserved, "server restart/relog retained exact frozen row in uncovered month"): return false
	if not completion.step(await capture_case("restart-frozen-only"), "restart account state"): return false
	if not completion.require(first_time_owner().is_empty() and before.rows.is_empty(), "restart contains no ordinary expired products"): return false
	if not completion.step(await capture_case("restart-no-expired-rows"), "restart membership"): return false
	if not completion.require(control("PerksProgramFrozenLabel") != null and str(control("PerksProgramFrozenLabel").text).begins_with("Freezing:\n"), "genuine committed frozen caption after restart"): return false
	if not completion.step(await capture_case("restart-frozen-caption"), "restart frozen caption"): return false
	if not completion.step(await leave_case("restart-leave"), "first Leave restores HUD"): return false
	if not completion.step(await open_vendor(), "reopen authentic vendor"): return false
	if not completion.require(int(client.trading_post_state().items) == 1 and int(client.trading_post_state().frozen) == reserved, "reopen cannot reintroduce an expired ordinary list"): return false
	if not completion.step(await capture_case("reopen-frozen-only"), "reopen membership"): return false
	if not completion.step(await checked_click("PerksProgramFrozenSlot"), "select real retained frozen item"): return false
	if not completion.step(await wait_until(func(): return int(client.trading_post_state().selected) == reserved, "retained frozen selection"), "frozen item exposes selected-item ownership"): return false
	if not completion.require(int(client.trading_post_state().tender) == int(before.tender) and not client.trading_post_state().purchased, "read-only reopen preserves wallet and ownership"): return false
	if not completion.step(await capture_case("reopen-conservation"), "reopen conservation"): return false
	return await leave_case("final-leave-restores-hud")

func leave_case(name: String) -> bool:
	if not completion.require(int(client.trading_post_state().hidden_layers) > 0, "vendor owns fullscreen HUD suppression"): return false
	if not completion.step(await checked_click("PerksProgramLeaveButton"), "Leave native input"): return false
	if not completion.step(await wait_until(func(): return not client.trading_post_state().open and control("MinimapLauncherButton") != null, "Leave restores HUD"), "HUD restored"): return false
	return await capture_case(name)
