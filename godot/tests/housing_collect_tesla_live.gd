extends "res://tests/housing_collection_live.gd"

# A second real Pawdo item, not the original hard-coded bookcase:
# item246487 -> collect1242335/effect324 -> decor2330; 75g, no ExtendedCost.
const TESLA_ITEM: int = 246487
const TESLA_DECOR: int = 2330
const TESLA_NAME: String = "Gnomish Tesla Coil"
const TESLA_PRICE: int = 750000
var tesla_receipts: Array = []

func run_test() -> void:
	root.size = Vector2i(1280,720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty():
		fail("Tesla proof requires private5870/fb_housing_* evidence configuration")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await show_tesla(0,"tesla-before"): return
	await click_control(housing_control("HousingDashboardFrameCloseButton"), MOUSE_BUTTON_LEFT)
	var vendor: Dictionary = await find_named_vendor(PAWDO_NAME)
	if vendor.is_empty() or not await open_pawdo_shop(vendor): return
	var before: Dictionary = client.merchant_state()
	var index: int = before.items.find(TESLA_NAME)
	if index < 0 or bag_count(before,TESLA_ITEM) != 0:
		fail("Needs real Pawdo Tesla listing and no existing Tesla bag item")
		return
	await click_control(merchant_control("MerchantItem%d" % (index+1)), MOUSE_BUTTON_RIGHT)
	if not await wait_for(func(s): return bag_count(s,TESLA_ITEM) == 1 and int(s.money) == int(before.money)-TESLA_PRICE, "paid authentic Tesla purchase"): return
	tesla_receipts.append({"phase":"paid", "before":before, "after":client.merchant_state(), "price":TESLA_PRICE})
	if not await capture_housing("tesla-paid"): return
	await click_control(merchant_control("MerchantFrameCloseButton"), MOUSE_BUTTON_LEFT)
	await tap(KEY_B)
	var item: Dictionary = bag_slot(client.merchant_state(),TESLA_ITEM)
	if item.is_empty():
		fail("Paid Tesla has no authoritative bag placement")
		return
	await click_control(housing_named_control("BagsUI","ContainerFrame%dSlot%d" % [item.bag,item.slot]), MOUSE_BUTTON_RIGHT)
	if not await wait_for(func(s): return bag_count(s,TESLA_ITEM) == 0 and int(s.money) == int(before.money)-TESLA_PRICE, "normal non-bookcase decor bag use"): return
	await tap(KEY_B)
	if not await show_tesla(1,"tesla-collected"): return
	if not await connect_housing_account(): return
	if not await show_tesla(1,"tesla-relogged"): return
	var file: FileAccess = FileAccess.open(shots.path_join("tesla-receipts.json"),FileAccess.WRITE)
	if file == null:
		fail("Cannot record actual Tesla acquisition")
		return
	file.store_string(JSON.stringify(tesla_receipts,"\t"))
	file.close()
	print("PASS: private5870 second authentic vendor decor, paid750000, normal bag collect2330 and relog ",housing_skin)
	client.free()
	quit(0)

func show_tesla(quantity: int, phase: String) -> bool:
	await click_control(housing_named_control("MicroMenuUI","HousingMicroButton"),MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HousingDashboardCatalogTabButton") != null, "catalog"): return false
	await click_control(housing_control("HousingDashboardCatalogTabButton"),MOUSE_BUTTON_LEFT)
	var search: LineEdit = housing_control("HousingCatalogSearchBox") as LineEdit
	await click_control(search,MOUSE_BUTTON_LEFT)
	search.select_all()
	await housing_type_text(TESLA_NAME)
	if not await wait_housing(func(): return housing_text("HousingPreviewName") == TESLA_NAME and catalog_stored(TESLA_DECOR) == quantity and housing_count(TESLA_DECOR, "stored") == quantity, "actual Tesla storage %d" % quantity): return false
	tesla_receipts.append({"phase":phase, "skin":housing_skin, "owned":housing_text("HousingPreviewNumOwned"), "sources":housing_text("HousingPreviewSourceInfo"), "state":client.housing_state(), "merchant":client.merchant_state()})
	return await capture_housing(phase)
