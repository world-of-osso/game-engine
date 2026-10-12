extends "res://tests/housing_collection_live.gd"

# Private5870 only. Retail HousingCatalog collection and source UI for an account that
# already owns real decor: owned count (stored + placed), authored CollectableSource lines
# named through the creature query, the entry tooltip, the Collected filter and the
# focused category's subcategory column. Grants nothing.
const BOOKCASE_DECOR: int = 1693
var catalog_receipts: Array = []

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty():
		fail("Catalog proof requires private5870/fb_housing_* evidence configuration")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	await click_control(housing_named_control("MicroMenuUI", "HousingMicroButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HousingDashboardCatalogTabButton") != null and client.housing_state().get("confirmed", false), "authoritative dashboard"): return
	await click_control(housing_control("HousingDashboardCatalogTabButton"), MOUSE_BUTTON_LEFT)
	if not await show_bookcase_sources(): return
	if not await show_entry_tooltip(): return
	if not await show_collected(): return
	if not await show_subcategories(): return
	var file: FileAccess = FileAccess.open(shots.path_join("catalog-receipts.json"), FileAccess.WRITE)
	if file == null:
		fail("Cannot record catalog receipts")
		return
	file.store_string(JSON.stringify(catalog_receipts, "\t"))
	file.close()
	print("PASS: private5870 Retail catalog owned counts, authored sources, entry tooltip, Collected filter and subcategories ", housing_skin)
	client.free()
	quit(0)

func owned(decor_id: int) -> int:
	return housing_count(decor_id, "stored") + housing_count(decor_id, "placed")

func show_bookcase_sources() -> bool:
	var search: LineEdit = housing_control("HousingCatalogSearchBox") as LineEdit
	await click_control(search, MOUSE_BUTTON_LEFT)
	search.select_all()
	await housing_type_text(BOOKCASE_NAME)
	var total: int = owned(BOOKCASE_DECOR)
	if total <= 0:
		fail("Catalog proof needs an account that already owns the paid bookcase: " + str(client.housing_state()))
		return false
	# Pawdo (252312) sells it in Stormwind City and Orgrimmar (CollectableSourceVendorSparse).
	if not await wait_housing(func(): return housing_text("HousingPreviewName") == BOOKCASE_NAME and housing_text("HousingPreviewNumOwned") == str(total) and housing_text("HousingPreviewSourceInfo").contains("Vendor: " + PAWDO_NAME + " in Stormwind City") and housing_text("HousingPreviewSourceInfo").contains("Vendor: " + PAWDO_NAME + " in Orgrimmar"), "owned count %d and creature-query vendor sources" % total): return false
	catalog_receipts.append({"phase": "sources", "skin": housing_skin, "owned": housing_text("HousingPreviewNumOwned"), "sources": housing_text("HousingPreviewSourceInfo"), "state": client.housing_state()})
	return await capture_housing("catalog-sources")

func show_entry_tooltip() -> bool:
	var tile: Control = housing_control("HousingCatalogEntry%d" % BOOKCASE_DECOR)
	await hover_point(tile.get_global_rect().get_center())
	var line: String = "Total Owned: %d (Placed: %d, Storage: %d)" % [owned(BOOKCASE_DECOR), housing_count(BOOKCASE_DECOR, "placed"), housing_count(BOOKCASE_DECOR, "stored")]
	if not await wait_housing(func(): return client.tooltip_state().visible and client.tooltip_state().title == BOOKCASE_NAME and str(client.tooltip_state().lines).contains(line), "entry tooltip " + line): return false
	catalog_receipts.append({"phase": "tooltip", "skin": housing_skin, "tooltip": client.tooltip_state()})
	if not await capture_housing("catalog-tooltip"): return false
	await hover_point(Vector2(640, 700))
	return true

func show_collected() -> bool:
	var search: LineEdit = housing_control("HousingCatalogSearchBox") as LineEdit
	await click_control(search, MOUSE_BUTTON_LEFT)
	search.select_all()
	var backspace: InputEventKey = InputEventKey.new()
	backspace.keycode = KEY_BACKSPACE
	for pressed in [true, false]:
		backspace.pressed = pressed
		root.push_input(backspace.duplicate(), true)
		await process_frame
	if not await wait_housing(func(): return housing_text("HousingCatalogSearchBox") == "", "cleared search"): return false
	await click_control(housing_control("HousingCatalogFilters"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HousingFilterCollectionCollected") != null, "filter dropdown"): return false
	await click_control(housing_control("HousingFilterCollectionCollected"), MOUSE_BUTTON_LEFT)
	var expected: Array = []
	for kind in ["stored", "placed"]:
		for decor in client.housing_state().get(kind, []):
			if int(decor.quantity) > 0 and not expected.has(int(decor.decor_id)): expected.append(int(decor.decor_id))
	expected.sort()
	if not await wait_housing(func(): return listed_decor() == expected and housing_control("HousingFilterCollectionCollectedTick") != null, "Collected filter lists exactly owned decor " + str(expected)): return false
	catalog_receipts.append({"phase": "collected", "skin": housing_skin, "listed": listed_decor(), "status": housing_text("HousingCatalogStatus")})
	if not await capture_housing("catalog-collected"): return false
	await click_control(housing_control("HousingFilterCollectionAll"), MOUSE_BUTTON_LEFT)
	await click_control(housing_control("HousingCatalogFilters"), MOUSE_BUTTON_LEFT)
	return await wait_housing(func(): return housing_control("HousingCatalogFilterMenu") == null, "closed filter dropdown")

func show_subcategories() -> bool:
	await click_control(housing_control("HousingCategory1"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HousingCategoryBackButton") != null and housing_control("HousingSubcategoryAll") != null and housing_control("HousingSubcategory1") != null and housing_text("HousingCatalogCategoryText") == "Furnishings", "Furnishings subcategory column"): return false
	await click_control(housing_control("HousingSubcategory1"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_text("HousingCatalogCategoryText") == "Furnishings | Seating", "Retail category path text"): return false
	catalog_receipts.append({"phase": "subcategories", "skin": housing_skin, "listed": listed_decor().size(), "status": housing_text("HousingCatalogStatus")})
	if not await capture_housing("catalog-subcategories"): return false
	await click_control(housing_control("HousingCategoryBackButton"), MOUSE_BUTTON_LEFT)
	return await wait_housing(func(): return housing_control("HousingCategory1") != null and housing_control("HousingCategoryBackButton") == null and housing_text("HousingCatalogCategoryText") == "All", "BACK to top-level All category")

func listed_decor() -> Array:
	var ids: Array = []
	var container: Node = housing_control("HousingCatalogOptionsContainer")
	if container == null: return ids
	for child in container.find_children("HousingCatalogEntry*", "", true, false):
		var id: String = str(child.name).trim_prefix("HousingCatalogEntry")
		if id.is_valid_int(): ids.append(int(id))
	ids.sort()
	return ids
