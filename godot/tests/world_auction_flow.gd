extends "res://tests/world_loot_options_flow.gd"

# Main runs AFTER game-cli proof, using an owned loopback server and disposable roster.
# Startup: -- --screen inworld --server <loopback> --char <fixture character>.
# Required env: GODOT_AUCTION_CLI_PROVED=1, GODOT_TEST_SERVER, GODOT_AUCTION_NPC,
# GODOT_AUCTION_MODE=seller|buyer. Seller needs >=3 sellable items and deposit money.
# Buyer needs externally seeded listings: GODOT_AUCTION_BID_ID, GODOT_AUCTION_BUYOUT_ID.
# No server setup, shared account mutations or credentials are embedded here.

const WAIT_MS := 120000
var client: Node
var npc_name: String
var item_id := 2589

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	npc_name = OS.get_environment("GODOT_AUCTION_NPC")
	var mode := OS.get_environment("GODOT_AUCTION_MODE")
	if OS.get_environment("GODOT_AUCTION_CLI_PROVED") != "1" or not endpoint.begins_with("127.0.0.1:") or npc_name.is_empty() or mode not in ["seller", "buyer"]:
		fail("Owned loopback, named auctioneer, explicit mode and prior CLI proof required")
		return
	var supplied_item := OS.get_environment("GODOT_AUCTION_ITEM_ID")
	if not supplied_item.is_empty():
		item_id = supplied_item.to_int()
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_until(func(): return client.account_state().screen == "InWorld", "world", WAIT_MS):
		return
	if not await open_auction():
		return
	await capture_auction("auction-open.png")
	if mode == "seller":
		if not await seller_operations():
			return
	else:
		if not await buyer_operations():
			return
	await tap(KEY_ESCAPE)
	if client.auction_state().open or client.get_node_or_null("GameMenuUI") != null:
		fail("Escape did not exclusively close AH")
		return
	if not await open_auction():
		return
	if not await click_name("AuctionHouseFrameCloseButton"):
		return
	if not await wait_until(func(): return not client.auction_state().open, "close button"):
		return
	print("FIXTURE NATIVE_AUCTION_DONE mode=", mode)
	client.free()
	quit(0)

func seller_operations() -> bool:
	if not await wait_until(func(): return client.auction_state().inventory.any(func(row): return row.item_id == item_id) and client.auction_state().money > 0, "sellable inventory and money"):
		return false
	var posted: Array[int] = []
	for token in ["12", "24", "48"]:
		if not await click_name("AuctionHouseFrameTab2"):
			return false
		var inventory: Array = client.auction_state().inventory
		var wanted: Dictionary = {}
		for item in inventory:
			if item.item_id == item_id:
				wanted = item
				break
		if wanted.is_empty():
			fail("Seller lacks required sellable item")
			return false
		if not await click_action("auction_sell_item:%s" % wanted.guid):
			return false
		if not set_text("AuctionHouseFrameItemSellFrameQuantityInputBox", "1") or not set_text("AuctionHouseFrameItemSellFramePriceInputSilver", "2"):
			return false
		await frames(3)
		if not await click_action("auction_duration_menu") or not await click_action("auction_duration:" + token):
			return false
		var before: Array = client.auction_state().owned.map(func(row): return row.auction_id)
		if not await click_name("AuctionHouseFrameItemSellFramePostButton"):
			return false
		if not await wait_until(func(): return client.auction_state().owned.any(func(row): return row.auction_id not in before), "posted auction"):
			return false
		for row in client.auction_state().owned:
			if row.auction_id not in before:
				posted.append(row.auction_id)
		print("FIXTURE POST duration=", token, " owned=", client.auction_state().owned)
	# Query and actual item-selection path, then server rejection of buying one's listing.
	if not await browse_item():
		return false
	if not await click_action("auction_select:%s" % posted[0]) or not await click_action("auction_buyout") or not await accept_auction_popup():
		return false
	if not await wait_until(func(): return world_error_contains("own auction"), "own-buyout rejection"):
		return false
	for id in posted:
		if not await click_name("AuctionHouseFrameTab3") or not await click_action("auction_auctions_tab:auctions") or not await click_action("auction_select:%s" % id) or not await click_action("auction_cancel"):
			return false
		if not await wait_until(func(): return client.auction_state().owned.all(func(row): return row.auction_id != id), "cancelled auction"):
			return false
	print("FIXTURE SELL_CANCEL_REJECTION")
	return true

func buyer_operations() -> bool:
	if not await wait_until(func(): return client.auction_state().money > 0, "buyer money"):
		return false
	var bid_id := OS.get_environment("GODOT_AUCTION_BID_ID").to_int()
	var buyout_id := OS.get_environment("GODOT_AUCTION_BUYOUT_ID").to_int()
	if bid_id <= 0 or buyout_id <= 0 or bid_id == buyout_id:
		fail("Buyer needs two distinct externally seeded auction IDs")
		return false
	if not await browse_item() or not await click_action("auction_select:%s" % bid_id) or not await click_action("auction_bid") or not await accept_auction_popup():
		return false
	if not await wait_until(func(): return client.auction_state().bids.any(func(row): return row.auction_id == bid_id), "accepted bid"):
		return false
	if not await click_name("AuctionHouseFrameTab3") or not await click_action("auction_auctions_tab:bids"):
		return false
	if not await click_action("auction_select:%s" % bid_id):
		return false
	if not await browse_item() or not await click_action("auction_select:%s" % buyout_id) or not await click_action("auction_buyout"):
		return false
	var before: int = client.auction_state().money
	if not await accept_auction_popup():
		return false
	if not await wait_until(func(): return client.auction_state().money < before and client.auction_state().search.all(func(row): return row.auction_id != buyout_id), "completed buyout"):
		return false
	print("FIXTURE BID_BUYOUT money=", client.auction_state().money)
	return true

func browse_item() -> bool:
	if not await click_name("AuctionHouseFrameTab1") or not set_text("AuctionHouseFrameSearchBox", ""):
		return false
	var revision: int = client.auction_state().search_revision
	if not await click_action("auction_search") or not await wait_until(func(): return client.auction_state().search_revision > revision, "browse reply"):
		return false
	for _page in range(20):
		if client.auction_state().groups.any(func(row): return row.item_id == item_id):
			revision = client.auction_state().search_revision
			if not await click_action("auction_browse_item:%s" % item_id):
				return false
			return await wait_until(func(): return client.auction_state().search_revision > revision, "exact item reply")
		var next := ui().find_child("AuctionPageNext", true, false) as Button
		if next == null or next.disabled:
			break
		revision = client.auction_state().search_revision
		await pointer(next.get_global_rect().get_center(), MOUSE_BUTTON_LEFT)
		if not await wait_until(func(): return client.auction_state().search_revision > revision, "browse next page reply"):
			return false
	fail("Item absent from fetched browse pages")
	return false

func open_auction() -> bool:
	var deadline := Time.get_ticks_msec() + WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var units := client.get_node_or_null("WorldUnits")
		var camera := root.get_camera_3d()
		if units == null or camera == null:
			continue
		for unit in units.get_children():
			if str(unit.name) != npc_name:
				continue
			var area := unit.find_child("UnitPick", true, false) as Area3D
			if area == null:
				continue
			var picked := auctioneer_surface_point(unit, area, camera)
			if picked.is_empty():
				continue
			await pointer(picked.point, MOUSE_BUTTON_RIGHT)
			if not await wait_until(func(): return client.auction_state().open or (ui() != null and ui().find_child("AuctionGossip", true, false) != null), "auction interaction"):
				return false
			if not client.auction_state().open:
				var selected := false
				for option in ui().find_children("AuctionGossipOption*", "Button", true, false):
					if "auction" in option.text.to_lower():
						await pointer(option.get_global_rect().get_center(), MOUSE_BUTTON_LEFT)
						selected = true
						break
				if not selected:
					fail("Gossip lacks explicit auction option")
					return false
			return await wait_until(func(): return client.auction_state().open, "native auction open")
	var units := client.get_node_or_null("WorldUnits")
	var camera := root.get_camera_3d()
	print("FIXTURE AUCTION_PICK_DIAGNOSTIC camera=", camera.global_transform if camera != null else null, " units=", units.get_children().map(func(unit): return {"name":str(unit.name),"position":unit.global_position}) if units != null else [])
	await capture_auction("auction-pick-failure.png")
	fail("Auctioneer must be nearby, visible and unoccluded")
	return false

func auctioneer_surface_point(unit: Node, area: Area3D, camera: Camera3D) -> Dictionary:
	var model := unit.get_node_or_null("NpcVisualRoot/NpcModel")
	if model == null:
		return {}
	for node in model.find_children("*", "MeshInstance3D", true, false):
		var mesh := node as MeshInstance3D
		if mesh.mesh == null or not mesh.is_visible_in_tree():
			continue
		var palette := corpse_bone_palette(mesh)
		for surface in range(mesh.mesh.get_surface_count()):
			for center in corpse_triangle_centroids(mesh, surface, palette):
				var point := camera.unproject_position(center)
				if camera.is_position_in_frustum(center) and Rect2(Vector2.ZERO, Vector2(root.size)).has_point(point) and UnitPicker.pick(camera, point) == area.get_meta("unit_server_id"):
					return {"point":point}
	return {}

func capture_auction(file: String) -> void:
	var directory := OS.get_environment("GODOT_AUCTION_SCREENSHOTS")
	if directory.is_empty():
		return
	var error := DirAccess.make_dir_recursive_absolute(directory)
	if error != OK:
		fail("Cannot create auction screenshot directory: " + str(error))
		return
	await RenderingServer.frame_post_draw
	error = root.get_texture().get_image().save_png(directory.path_join(file))
	if error != OK:
		fail("Cannot save auction screenshot: " + str(error))

func ui() -> Node:
	return client.get_node_or_null("AuctionUI")

func click_name(name: String) -> bool:
	var control := ui().find_child(name, true, false) as Control if ui() != null else null
	if control == null or not control.is_visible_in_tree():
		fail("Missing visible native control " + name)
		return false
	await pointer(control.get_global_rect().get_center(), MOUSE_BUTTON_LEFT)
	return true

func click_action(action: String) -> bool:
	# Find the registry-authored action, then use actual native pointer dispatch.
	for _page in range(20):
		if ui() == null:
			break
		var control := ui().control_for_action(action) as Control
		if control != null:
			await pointer(control.get_global_rect().get_center(), MOUSE_BUTTON_LEFT)
			return true
		var next := ui().find_child("AuctionRowsNext", true, false) as Button
		if next != null and not next.disabled and next.is_visible_in_tree():
			await pointer(next.get_global_rect().get_center(), MOUSE_BUTTON_LEFT)
			continue
		var page_next := ui().find_child("AuctionPageNext", true, false) as Button
		if page_next == null or page_next.disabled or not page_next.is_visible_in_tree():
			break
		var old_revision: int = client.auction_state().search_revision
		await pointer(page_next.get_global_rect().get_center(), MOUSE_BUTTON_LEFT)
		if not await wait_until(func(): return client.auction_state().search_revision > old_revision, "next server page"):
			return false
	fail("Missing enabled action " + action)
	return false

func accept_auction_popup() -> bool:
	var host := client.get_node_or_null("StaticPopupUI")
	var accept := host.find_child("StaticPopup1Button1", true, false) as Button if host != null else null
	if accept == null or not accept.is_visible_in_tree() or accept.disabled:
		fail("Missing enabled global auction confirmation")
		return false
	await pointer(accept.get_global_rect().get_center(), MOUSE_BUTTON_LEFT)
	return true

func set_text(name: String, value: String) -> bool:
	var edit := ui().find_child(name, true, false) as LineEdit if ui() != null else null
	if edit == null:
		fail("Missing native edit box " + name)
		return false
	edit.text = value
	edit.text_changed.emit(value)
	return true

func world_error_contains(text: String) -> bool:
	var errors := client.get_node_or_null("UIErrors")
	if errors != null:
		for label in errors.find_children("*", "Label", true, false):
			if label.is_visible_in_tree() and text in label.text.to_lower():
				return true
	return false

func wait_until(predicate: Callable, description: String, timeout := 8000) -> bool:
	var deadline := Time.get_ticks_msec() + timeout
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call():
			await frames(3)
			return true
	fail("Timed out: " + description + " " + str(client.auction_state()))
	return false

func pointer(point: Vector2, button: MouseButton) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await process_frame
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = button
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await frames(3)

func tap(key: Key) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = key
		event.physical_keycode = key
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await frames(3)

func frames(count: int) -> void:
	for _frame in range(count):
		await process_frame

func fail(message: String) -> void:
	push_error("FIXTURE NATIVE_AUCTION: " + message)
	quit(1)
