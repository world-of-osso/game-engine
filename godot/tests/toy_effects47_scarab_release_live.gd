extends "res://tests/toy_effects43_scarab_live.gd"
# Same source/count/follow/23..27s expiry gate; fresh accounts select saved Options normally.
func run_fixture() -> void:
	root.size = Vector2i(1920, 1080)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_until(func(): return not client.account_state().assets_starting, "assets", 180000): return
	if OS.get_environment("GODOT_TEST_SERVER") != "127.0.0.1:5911": fail("private5911 required"); return
	var account := OS.get_environment("TOYFX_ACCOUNT")
	if not account.begins_with("fb_"): fail("private fb_* account required"); return
	var error: String = client.connect_account("127.0.0.1:5911", account, "fbtest", false)
	if error != "": fail(error); return
	if not await wait_until(func(): return control("CharCard_0") != null, "roster", 120000): return
	await click("CharCard_0")
	await click("EnterWorld")
	if not await wait_until(func(): return client.account_state().screen == "InWorld" and control("MinimapLauncherButton") != null, "world", 180000): return
	await tap(KEY_ESCAPE)
	await click("MenuBtnOptions")
	await click("OptionsTabhud")
	var wanted_skin := "Modern" if skin == "modern" else "Forever"
	if client.account_state().ui_skin != wanted_skin:
		await click("Choiceui_layout%sHit" % ("1" if skin == "forever" else "0"))
	if control("CollectionsMicroButton") == null:
		await click("ToggleSwitchlayout_show_micro_menuRightHit")
	await click("OptionsDoneButton")
	if control("MenuBtnResume") != null: await click("MenuBtnResume")
	if client.account_state().ui_skin != wanted_skin: fail("wrong native art skin"); return
	if not await wait_until(func(): return control("CollectionsMicroButton") != null, "enabled native Micro Menu"): return
	if not await wait_until(func(): return native_display(-1), "native player model", 120000): return
	if not await wait_until(func(): return client.merchant_state().item_catalog_loaded, "source item catalog", 180000): return
	if not await wait_until(func(): return client.toybox_state().catalog_count == 1166, "full authenticated toy catalog", 120000): return
	await capture("native")
	for sample in proof_cases():
		await prove_toy(sample)
		if failed: return
	write_receipts()
	print("PASS toy-effects14 actual release-server Scarab ", skin)
	client.free()
	quit(0)
