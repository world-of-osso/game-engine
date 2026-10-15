extends "res://tests/world_merchant_live_flow.gd"

# Private5870 only. Real Pawdo purchase and normal bag UseItem; no granted decor/items.
# Gold and position setup belong to the external private fixture, not this script.
const BOOKCASE_ITEM: int = 245259
const BOOKCASE_NAME: String = "Small Val'sharah Bookcase"
const PAWDO_NAME: String = "Second Chair Pawdo"
var housing_receipts: Array = []
var housing_server: String
var housing_account: String
var housing_skin: String

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty():
		fail("Housing collection needs explicit private5870/fb_housing_* evidence configuration")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await show_housing_bookcase(0, "before"): return
	await click_control(housing_control("HousingDashboardFrameCloseButton"), MOUSE_BUTTON_LEFT)
	if not await buy_bookcase(): return
	if not await collect_bookcase(): return
	if not await show_housing_bookcase(1, "collected"): return
	if not await connect_housing_account(): return
	if not await show_housing_bookcase(1, "relogged"): return
	if not write_housing_receipts(): return
	print("PASS: private5870 authentic Pawdo purchase, normal bag collection, authoritative storage and relog ", housing_skin)
	client.free()
	quit(0)

func connect_housing_account() -> bool:
	var error: String = client.connect_account(housing_server, housing_account, "fbtest", false)
	if error != "":
		fail("Housing authentication: " + error)
		return false
	if not await wait_housing(func(): return client.account_state().screen == "CharacterSelect" and housing_roster_visible(), "visible prepared roster", 180000): return false
	print("HOUSING_ROSTER_REVEALED account=",client.account_state()," visible=",client.get_node("CharacterSelectUI").visible)
	if not await click_exposed_control(housing_named_control("CharacterSelectUI", "CharCard_0")): return false
	if not await click_exposed_control(housing_named_control("CharacterSelectUI", "EnterWorld")): return false
	if not await wait_housing(func(): return client.account_state().screen == "InWorld" and housing_named_control("MicroMenuUI", "HousingMicroButton") != null, "world", 420000): return false
	if str(client.account_state().ui_skin) != housing_skin:
		fail("Expected " + housing_skin + " skin, observed " + str(client.account_state().ui_skin))
		return false
	await frames(60)
	return true

# Session state precedes the loading curtain's reveal; existence is not input readiness.
func housing_roster_visible() -> bool:
	var ui: CanvasLayer = client.get_node_or_null("CharacterSelectUI") as CanvasLayer
	if ui == null or not ui.visible: return false
	var card: Control = housing_named_control("CharacterSelectUI", "CharCard_0")
	var enter: Control = housing_named_control("CharacterSelectUI", "EnterWorld")
	return card != null and card.is_visible_in_tree() and enter != null and enter.is_visible_in_tree()

func buy_bookcase() -> bool:
	var vendor: Dictionary = await find_named_vendor(PAWDO_NAME)
	if vendor.is_empty() or not await open_pawdo_shop(vendor): return false
	var before: Dictionary = client.merchant_state()
	var item_index: int = before.items.find(BOOKCASE_NAME)
	if item_index < 0 or bag_count(before, BOOKCASE_ITEM) != 0:
		fail("Needs authentic Pawdo bookcase listing and initially empty bookcase bags: " + str(before))
		return false
	var price: int = int(OS.get_environment("HOUSING_BOOKCASE_PRICE"))
	if price <= 0:
		fail("Bookcase fixture requires its verified vendor price")
		return false
	await click_control(merchant_control("MerchantItem%d" % (item_index + 1)), MOUSE_BUTTON_RIGHT)
	if not await wait_for(func(s): return bag_count(s, BOOKCASE_ITEM) == 1 and int(s.money) == int(before.money) - price, "authentic bookcase purchase"): return false
	housing_receipts.append({"phase": "vendor-purchase", "skin": housing_skin, "before": before, "after": client.merchant_state(), "price": price})
	if not write_housing_receipts(): return false
	if not await capture_housing("vendor-purchase"): return false
	await click_control(merchant_control("MerchantFrameCloseButton"), MOUSE_BUTTON_LEFT)
	return await wait_for(func(s): return not s.open, "merchant close before collection")

func open_pawdo_shop(vendor: Dictionary) -> bool:
	# Authored menu40669, option1: "Show me your wares." Pawdo has QUEST_GIVER+GOSSIP flags.
	await click(vendor.point, MOUSE_BUTTON_RIGHT)
	await frames(10)
	print("HOUSING_PAWDO_CLICK target=", client.target_state(), " merchant=", client.merchant_state().open)
	for node in client.find_children("*Gossip*", "", true, false):
		print("HOUSING_GOSSIP_NODE ", node.get_path(), " visible=", node.is_visible_in_tree() if node is CanvasItem else false)
	if not await capture_housing("pawdo-after-right-click"): return false
	if not await wait_housing(func(): return housing_named_control("QuestFrameUI", "GossipOption1Text") != null, "Pawdo's authored gossip"): return false
	var text: Control = housing_named_control("QuestFrameUI", "GossipOption1Text")
	if text == null or str(text.get("text")) != "Show me your wares.":
		fail("Pawdo option1 does not match the authentic wares choice")
		return false
	await click_control(housing_named_control("QuestFrameUI", "GossipOption1Text"), MOUSE_BUTTON_LEFT)
	return await wait_for(func(s): return s.open and s.vendor_name == PAWDO_NAME, "Pawdo merchant frame")

func collect_bookcase() -> bool:
	var backpack: Control = housing_named_control("BagsUI", "ContainerFrame0")
	if backpack == null or not backpack.is_visible_in_tree():
		await tap(KEY_B)
	if not await wait_housing(func():
		var visible_bag: Control = housing_named_control("BagsUI", "ContainerFrame0")
		return visible_bag != null and visible_bag.is_visible_in_tree(), "backpack"): return false
	var item: Dictionary = bag_slot(client.merchant_state(), BOOKCASE_ITEM)
	if item.is_empty():
		fail("Purchased bookcase has no authoritative bag placement")
		return false
	var before_money: int = int(client.merchant_state().money)
	await click_control(housing_named_control("BagsUI", "ContainerFrame%dSlot%d" % [item.bag, item.slot]), MOUSE_BUTTON_RIGHT)
	if not await wait_for(func(s): return bag_count(s, BOOKCASE_ITEM) == 0 and int(s.money) == before_money, "normal bag bookcase collection"): return false
	housing_receipts.append({"phase": "bag-use", "skin": housing_skin, "item": item, "after": client.merchant_state()})
	if not write_housing_receipts(): return false
	await tap(KEY_B)
	return true

func show_housing_bookcase(quantity: int, phase: String) -> bool:
	await click_control(housing_named_control("MicroMenuUI", "HousingMicroButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HousingDashboardCatalogTabButton") != null, "dashboard"): return false
	await click_control(housing_control("HousingDashboardCatalogTabButton"), MOUSE_BUTTON_LEFT)
	var search: LineEdit = housing_control("HousingCatalogSearchBox") as LineEdit
	if search == null:
		fail("Housing catalog search is not a native LineEdit")
		return false
	await click_control(search, MOUSE_BUTTON_LEFT)
	search.select_all()
	await housing_type_text(BOOKCASE_NAME)
	if not await wait_housing(func(): return housing_text("HousingPreviewName") == BOOKCASE_NAME and catalog_stored(1693) == quantity and housing_count(1693, "stored") == quantity, "authoritative bookcase count " + str(quantity)): return false
	housing_receipts.append({"phase": phase, "skin": housing_skin, "account": client.account_state(), "owned": housing_text("HousingPreviewNumOwned"), "sources": housing_text("HousingPreviewSourceInfo"), "status": housing_text("HousingCatalogStatus"), "bags": client.merchant_state().bags})
	if not write_housing_receipts(): return false
	return await capture_housing(phase)

# HousingCatalogEntry InfoText: the stored quantity, hidden at zero.
func catalog_stored(decor_id: int) -> int:
	var badge: Control = housing_control("HousingCatalogEntry%dQuantity" % decor_id)
	if badge == null or not badge.is_visible_in_tree(): return 0
	return int(str(badge.get("text")))

# Server-confirmed account count of `decor_id` in housing_state()[kind] ("stored"/"placed").
func housing_count(decor_id: int, kind: String) -> int:
	for decor in client.housing_state().get(kind, []):
		if int(decor.decor_id) == decor_id: return int(decor.quantity)
	return 0

func housing_control(name: String) -> Control:
	return housing_named_control("HousingDashboardUI", name)

func housing_named_control(canvas: String, name: String) -> Control:
	var node: Node = client.get_node_or_null(canvas)
	return node.find_child(name, true, false) as Control if node != null else null

func housing_text(name: String) -> String:
	var control: Control = housing_control(name)
	return str(control.get("text")) if control != null else ""

func wait_housing(predicate: Callable, label: String, timeout_ms: int = 15000) -> bool:
	var deadline: int = Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call(): return true
	fail("Housing timeout: " + label)
	return false

func housing_type_text(text: String) -> void:
	for character in text:
		for pressed in [true, false]:
			var event: InputEventKey = InputEventKey.new()
			event.unicode = character.unicode_at(0)
			event.pressed = pressed
			root.push_input(event, true)
			await process_frame
	await frames(3)

func capture_housing(phase: String) -> bool:
	await RenderingServer.frame_post_draw
	var image: Image = root.get_texture().get_image()
	if image.get_width() > 1280:
		image.resize(1280, int(float(image.get_height()) * 1280.0 / float(image.get_width())), Image.INTERPOLATE_LANCZOS)
	var path: String = shots.path_join("%s-%s.png" % [housing_skin.to_lower(), phase])
	if image.save_png(path) != OK:
		fail("Housing PNG save failed: " + path)
		return false
	return true

func write_housing_receipts() -> bool:
	var file: FileAccess = FileAccess.open(shots.path_join("collection-receipts.json"), FileAccess.WRITE)
	if file == null:
		fail("Cannot write housing collection receipts")
		return false
	file.store_string(JSON.stringify(housing_receipts, "\t"))
	if file.get_error() != OK:
		fail("Housing collection receipt write failed")
		return false
	return true

# Source map pins can overlap. Prove a visible, unobscured portion with Godot's
# real hover dispatch, rather than clicking a centre covered by another pin.
func click_exposed_control(control: Control) -> bool:
	if control == null:
		fail("Missing source control to pick")
		return false
	var rect: Rect2 = control.get_global_rect()
	for y in [0.5, 0.15, 0.85, 0.3, 0.7]:
		for x in [0.5, 0.15, 0.85, 0.3, 0.7]:
			var point: Vector2 = rect.position + rect.size * Vector2(x, y)
			await hover_point(point)
			var hovered: Control = root.gui_get_hovered_control()
			if hovered == control or (hovered != null and control.is_ancestor_of(hovered)):
				await click(point, MOUSE_BUTTON_LEFT)
				return true
	fail("Source control is fully obscured: " + str(control.name))
	return false
