extends "res://tests/housing_catalog_live.gd"

# Read-only native Catalog proof. No purchase, item/decor grant or property admission.
var reset_receipts: Array[Dictionary] = []
var confirmed_before: Dictionary

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	if housing_server != "127.0.0.1:5953" or not housing_account.begins_with("fb_housing_2_") or shots.is_empty():
		fail("Catalog Reset requires private5953/fb_housing_2_* evidence configuration")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	await click_control(housing_named_control("MicroMenuUI", "HousingMicroButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HousingDashboardCatalogTabButton") != null and client.housing_state().get("confirmed", false), "confirmed dashboard"): return
	confirmed_before = client.housing_state().duplicate(true)
	await click_control(housing_control("HousingDashboardCatalogTabButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HousingCatalogFiltersResetButton") != null, "source Catalog Reset control"): return
	if housing_control("HousingCatalogFiltersResetButton").is_visible_in_tree():
		fail("Default Catalog filters must hide Reset")
		return
	if not verify_header_geometry(): return
	if not await record_reset("default"): return
	if not await prove_search_reset(): return
	if not await prove_category_reset(): return
	if client.housing_state() != confirmed_before:
		fail("Read-only Catalog filters changed confirmed ownership or storage")
		return
	print("PASS: private5953 source Catalog Reset preserves search/category and confirmed authority ", housing_skin)
	client.free()
	quit(0)

func verify_header_geometry() -> bool:
	var dashboard := housing_control("HousingDashboardFrame").get_global_rect()
	var header := housing_control("HousingCatalogFilters").get_global_rect()
	var reset := housing_control("HousingCatalogFiltersResetButton").get_global_rect()
	var scale: float = dashboard.size.x / 814.0
	var expected := Vector2(dashboard.end.x - 110.0 * scale, dashboard.position.y + 30.0 * scale)
	if header.size.distance_to(Vector2(100.0, 20.0) * scale) > 0.1 or header.position.distance_to(expected) > 0.1:
		fail("Catalog Filter lost source100x20 TOPRIGHT(-10,-30): " + str(header))
		return false
	if reset.size.distance_to(Vector2(23.0, 23.0) * scale) > 0.1 or reset.get_center().distance_to(Vector2(header.end.x - 3.0 * scale, header.position.y)) > 0.1:
		fail("Catalog Reset lost source23x23 TOPRIGHT-3: " + str(reset))
		return false
	return true

func prove_search_reset() -> bool:
	var search := housing_control("HousingCatalogSearchBox") as LineEdit
	await click_control(search, MOUSE_BUTTON_LEFT)
	search.select_all()
	await housing_type_text(BOOKCASE_NAME)
	if not await wait_housing(func(): return listed_decor() == [BOOKCASE_DECOR], "authored Bookcase search result"): return false
	await click_control(housing_control("HousingCatalogFilters"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HousingFilterDyeable") != null, "Catalog Filter menu"): return false
	await click_control(housing_control("HousingFilterDyeable"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return listed_decor().is_empty() and housing_control("HousingCatalogFiltersResetButton").is_visible_in_tree(), "Dyeable hides the non-dyeable Bookcase and reveals Reset"): return false
	if not await record_reset("search-filtered"): return false
	await click_control(housing_control("HousingCatalogFiltersResetButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return listed_decor() == [BOOKCASE_DECOR] and not housing_control("HousingCatalogFiltersResetButton").is_visible_in_tree(), "Reset restores Bookcase results"): return false
	if housing_text("HousingCatalogSearchBox") != BOOKCASE_NAME or housing_control("HousingCatalogFilterMenu") == null:
		fail("Reset must retain search and the open source menu")
		return false
	if not await record_reset("search-reset"): return false
	await click_control(housing_control("HousingCatalogFilters"), MOUSE_BUTTON_LEFT)
	return await wait_housing(func(): return housing_control("HousingCatalogFilterMenu") == null, "closed Filter menu")

func prove_category_reset() -> bool:
	await click_control(housing_control("HousingCategory1"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HousingSubcategory6") != null, "authored Furnishings Storage category"): return false
	await click_control(housing_control("HousingSubcategory6"), MOUSE_BUTTON_LEFT)
	var category_path := housing_text("HousingCatalogCategoryText")
	if category_path.is_empty() or housing_text("HousingCatalogSearchBox") != "":
		fail("Category focus must have a real path and clear the previous search")
		return false
	var before_ids := listed_decor()
	if before_ids.is_empty():
		fail("Real Storage category must contain visible catalog entries")
		return false
	await click_control(housing_control("HousingCatalogFilters"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HousingFilterCollectionCollected") != null, "Collection filter"): return false
	await click_control(housing_control("HousingFilterCollectionCollected"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HousingCatalogFiltersResetButton").is_visible_in_tree(), "nondefault Collection reveals Reset"): return false
	if not await record_reset("category-filtered"): return false
	await click_control(housing_control("HousingCatalogFiltersResetButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return listed_decor() == before_ids and not housing_control("HousingCatalogFiltersResetButton").is_visible_in_tree(), "Reset restores category results"): return false
	if housing_text("HousingCatalogCategoryText") != category_path or housing_text("HousingCatalogSearchBox") != "":
		fail("Reset changed the focused category or search")
		return false
	return await record_reset("category-reset")

func record_reset(phase: String) -> bool:
	await hover_point(Vector2(960, 1030))
	reset_receipts.append({"phase": phase, "skin": housing_skin, "viewport": root.size, "header": housing_control("HousingCatalogFilters").get_global_rect(), "reset_visible": housing_control("HousingCatalogFiltersResetButton").is_visible_in_tree(), "search": housing_text("HousingCatalogSearchBox"), "category": housing_text("HousingCatalogCategoryText"), "listed": listed_decor(), "housing": client.housing_state()})
	var output := FileAccess.open(shots.path_join("catalog-reset-receipts.json"), FileAccess.WRITE)
	if output == null:
		fail("Cannot save Catalog Reset receipt")
		return false
	output.store_string(JSON.stringify(reset_receipts, "\t"))
	output.flush()
	var receipt_error := output.get_error()
	output.close()
	if receipt_error != OK:
		fail("Cannot persist Catalog Reset receipt: " + str(receipt_error))
		return false
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	if image.get_size() != Vector2i(1920, 1080):
		fail("Catalog Reset acceptance requires untouched1920x1080 originals")
		return false
	if image.save_png(shots.path_join(housing_skin.to_lower() + "-catalog-reset-" + phase + ".png")) != OK:
		fail("Cannot save full-HD Catalog Reset PNG")
		return false
	return true
