extends "res://tests/housing_catalog_live.gd"

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty():
		fail("Catalog search needs private5870/fb_housing_* evidence")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	await click_control(housing_named_control("MicroMenuUI", "HousingMicroButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HousingDashboardCatalogTabButton") != null and client.housing_state().get("confirmed", false), "confirmed search dashboard"): return
	if not await click_exposed_control(housing_control("HousingDashboardCatalogTabButton")): return
	if not await wait_housing(func(): return catalog_input() != null, "Catalog native input"): return
	var before: Dictionary = client.housing_state()
	if owned(1693) != 1 or owned(2330) != 1:
		fail("Catalog search requires genuine paid bookcase and Tesla")
		return
	for text: String in ["W".repeat(45), "é".repeat(45)]:
		if not await type_catalog(text, text.left(40)): return
		if catalog_input().max_length != 40:
			fail("Catalog source max_length must be40")
			return
	if not await click_exposed_control(housing_control("HousingCatalogSearchClearButton")): return
	if not await wait_housing(func(): return catalog_blurred("") and not catalog_clear_visible(), "clear empties backing text/releases focus"): return
	var style: StyleBox = catalog_input().get_theme_stylebox("normal")
	if absf(style.get_content_margin(SIDE_LEFT)-16.0)>0.01 or absf(style.get_content_margin(SIDE_RIGHT)-20.0)>0.01:
		fail("Catalog source16/20 gutters missing")
		return
	if not await click_exposed_control(housing_control("HousingCatalogFilters")): return
	if not await click_exposed_control(housing_control("HousingFilterCollectionCollected")): return
	if not await click_exposed_control(housing_control("HousingCatalogFilters")): return
	if not await click_exposed_control(housing_control("HousingCategory2")): return
	for raw: String in ["z", "zz", "éé"]:
		if not await type_catalog(raw, raw): return
		if not await wait_housing(func(): return housing_text("HousingCatalogCategoryText")=="Structural" and listed_decor()==[2330], "shortUTF8 retains category and paid result"): return
	if not await click_exposed_control(housing_control("HousingSubcategoryAll")): return
	if catalog_input().text != "éé":
		fail("No-op child-All erased short query")
		return
	if not await capture_housing("catalog-search-short-utf8"): return
	if not await type_catalog("zzz", "zzz"): return
	if not await wait_housing(func(): return housing_text("HousingCatalogCategoryText")=="All" and listed_decor().is_empty(), "third character applies query and clears category"): return
	if not await capture_housing("catalog-search-empty-results"): return
	if not await type_catalog("tEs", "tEs"): return
	if not await wait_housing(func(): return listed_decor()==[2330], "Catalog case-insensitive paid Tesla"): return
	for key: Key in [KEY_ENTER, KEY_ESCAPE]:
		if not await click_exposed_control(catalog_input()): return
		await catalog_key(key)
		if not await wait_housing(func(): return catalog_blurred("tEs") and listed_decor()==[2330], "source focus key preserves query/results"): return
	if not await click_exposed_control(housing_control("HousingCategory1")): return
	if not await wait_housing(func(): return catalog_blurred("") and not catalog_clear_visible() and listed_decor()==[1693], "specific category clears registry query and blurs"): return
	if not await click_exposed_control(housing_control("HousingCategoryBackButton")): return
	if not await click_exposed_control(catalog_input()): return
	await catalog_key(KEY_KP_ENTER)
	if not await wait_housing(func(): return catalog_blurred("") and not catalog_clear_visible(), "empty keypadEnter hides clear"): return
	if client.housing_state().stored != before.stored or client.housing_state().placed != before.placed or client.housing_state().decor != before.decor:
		fail("Catalog browsing changed paid quantities/layout")
		return
	if not await capture_housing("catalog-search-restored"): return
	print("PASS: private5870 shared Catalog search template ", housing_skin)
	client.free()
	quit(0)

func catalog_input() -> LineEdit:
	return housing_control("HousingCatalogSearchBox") as LineEdit

func catalog_blurred(text: String) -> bool:
	return catalog_input() != null and catalog_input().text==text and not catalog_input().has_focus()

func catalog_clear_visible() -> bool:
	var button: Control = housing_control("HousingCatalogSearchClearButton")
	return button != null and button.is_visible_in_tree()

func type_catalog(text: String, expected: String) -> bool:
	if not await click_exposed_control(catalog_input()): return false
	catalog_input().select_all()
	await housing_type_text(text)
	return await wait_housing(func(): return catalog_input().text==expected, "Catalog text="+expected)

func catalog_key(key: Key) -> void:
	for pressed: bool in [true, false]:
		var event: InputEventKey = InputEventKey.new()
		event.keycode = key
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
