extends "res://tests/world_merchant_cursor_flow.gd"

# Verifier-only observation flow (native_input_fixture ui-ownership).
# Records what happens; the peer logs every decoded request between case markers.
# Setup problems fail; behaviour differences are printed as UIOWN OBS lines only.
const UO_SETTLE_MS := 900
const UO_MERCHANT := "MerchantUI"
const UO_BAGS := "BagsUI"

var uo_vendor: Dictionary
var uo_bag1_source := ""
var uo_bag1_covered := ""
var uo_bag1_free := ""
var uo_merchant_clear := Vector2.INF
var uo_merchant_over_bag := Vector2.INF

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	if not endpoint.begins_with("127.0.0.1:") or endpoint.ends_with(":0"):
		fail("ui-ownership requires owned UDP endpoint")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_screen(client, "Loading", 180000):
		return
	print("FIXTURE UIOWN_LOADING")
	if not await wait_world(client):
		return
	print("FIXTURE UIOWN_READY")
	if not await uo_wait(func(): return client.merchant_state().bags.size() == 4, 10000):
		fail("snapshot not applied: " + str(client.merchant_state()))
		return
	uo_vendor = await find_vendor(client)
	if uo_vendor.is_empty() or not await open_vendor(client, uo_vendor):
		return
	await uo_settle()
	uo_obs("OPEN_VENDOR", client, "embedded_bag0=%s bagsui_bag0=%s bagsui_bag1=%s (Retail OpenAllBags opens every bag)" % [uo_visible(uo_ctl(client, UO_MERCHANT, "ContainerFrame0")), uo_visible(uo_ctl(client, UO_BAGS, "ContainerFrame0")), uo_visible(uo_ctl(client, UO_BAGS, "ContainerFrame1"))])
	# Close cases first: later item drags orbit the camera off the vendor.
	await uo_close_cases(client)
	if not await uo_reopen(client, "TARGET"):
		fail("vendor reopen for target cases failed")
		return
	if not uo_visible(uo_ctl(client, UO_BAGS, "ContainerFrame1")) and not await uo_open_bag1(client):
		return
	if not await uo_overlap_merchant(client):
		return
	await uo_target_cases(client)
	await uo_order_cases(client)
	await uo_world_case(client)
	await uo_auction_cases(client)
	await uo_equipment_case(client)
	print("FIXTURE UIOWN_DONE")
	while true:
		await process_frame

# ---------- helpers ----------

func uo_ui(client: Node, ui_name: String) -> Node:
	return client.get_node_or_null(ui_name)

func uo_ctl(client: Node, ui_name: String, control_name: String) -> Control:
	var ui := uo_ui(client, ui_name)
	return ui.find_child(control_name, true, false) as Control if ui != null else null

func uo_visible(control: Control) -> bool:
	return control != null and control.is_visible_in_tree()

func uo_center(client: Node, ui_name: String, control_name: String) -> Vector2:
	var control := uo_ctl(client, ui_name, control_name)
	if not uo_visible(control):
		return Vector2.INF
	return control.get_global_rect().get_center()

func uo_cursor_held(client: Node) -> bool:
	return uo_visible(client.find_child(MC_CURSOR, true, false) as Control)

func uo_popup(client: Node) -> bool:
	return uo_visible(client.find_child("StaticPopup1", true, false) as Control)

func uo_wait(predicate: Callable, ms: int) -> bool:
	var deadline := Time.get_ticks_msec() + ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call():
			return true
	return false

func uo_settle(ms: int = UO_SETTLE_MS) -> void:
	var deadline := Time.get_ticks_msec() + ms
	while Time.get_ticks_msec() < deadline:
		await process_frame

func uo_hover_path(point: Vector2) -> String:
	mc_motion(point, false)
	var hovered := root.gui_get_hovered_control()
	return str(hovered.get_path()) if hovered != null else "<none>"

func uo_obs(case_name: String, client: Node, extra: String = "") -> void:
	var state: Dictionary = client.merchant_state()
	print("UIOWN OBS ", case_name, " cursor_held=", uo_cursor_held(client), " popup=", uo_popup(client), " merchant_open=", state.open, " merchant_frame=", uo_visible(uo_ctl(client, UO_MERCHANT, "MerchantFrame")), " bag1=", uo_visible(uo_ctl(client, UO_BAGS, "ContainerFrame1")), " ", extra)

func uo_begin(case_name: String) -> void:
	print("FIXTURE UIOWN_CASE ", case_name, " BEGIN")
	await process_frame

func uo_end(case_name: String, client: Node, extra: String = "") -> void:
	await uo_settle()
	uo_obs(case_name, client, extra)
	print("FIXTURE UIOWN_CASE ", case_name, " END")
	await uo_reset_cursor(client)

func uo_reset_cursor(client: Node) -> void:
	if uo_popup(client):
		var no := client.find_child("StaticPopup1Button2", true, false) as Control
		if uo_visible(no):
			await uo_click(no.get_global_rect().get_center())
		await uo_settle(300)
		uo_obs("RESET_AFTER_POPUP_NO", client, "no_button=%s" % (str(no.get_path()) if no != null else "<missing>"))
	if uo_cursor_held(client):
		push_key(KEY_ESCAPE, true)
		await process_frame
		push_key(KEY_ESCAPE, false)
		await uo_settle(300)
		uo_obs("RESET_AFTER_ESCAPE", client)
	await uo_settle(400)
	if uo_cursor_held(client) or uo_popup(client):
		print("UIOWN SETUP_WARN cursor/popup still present after reset")

func uo_camera() -> String:
	var camera := root.get_viewport().get_camera_3d()
	return str(camera.global_transform) if camera != null else "<none>"

func uo_find_vendor(client: Node) -> Dictionary:
	var deadline := Time.get_ticks_msec() + 3000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var camera := root.get_viewport().get_camera_3d()
		var units := client.get_node_or_null("WorldUnits")
		if camera == null or units == null:
			continue
		for unit in units.get_children():
			if str(unit.name) != VENDOR:
				continue
			var area := unit.find_child("UnitPick", true, false) as Area3D
			if area == null:
				continue
			var center := (area.get_child(0) as Node3D).global_position
			if not camera.is_position_in_frustum(center):
				continue
			var point := camera.unproject_position(center)
			if UnitPicker.pick(camera, point) == area.get_meta("unit_server_id"):
				return {"point": point}
	return {}

func uo_click(point: Vector2) -> void:
	mc_motion(point, false)
	await process_frame
	mc_edge(point, true)
	await process_frame
	mc_edge(point, false)
	await process_frame

func uo_drag(start: Vector2, finish: Vector2) -> void:
	mc_motion(start, false)
	await process_frame
	mc_edge(start, true)
	await process_frame
	await process_frame
	mc_motion(finish, true)
	await process_frame
	mc_edge(finish, false)
	await process_frame

# ---------- setup ----------

func uo_open_bag1(client: Node) -> bool:
	var toggle := uo_ctl(client, UO_BAGS, "CharacterBag0Slot")
	if not uo_visible(toggle):
		fail("missing BagsUI CharacterBag0Slot")
		return false
	await uo_click(toggle.get_global_rect().get_center())
	if not await uo_wait(func(): return uo_visible(uo_ctl(client, UO_BAGS, "ContainerFrame1")), 5000):
		fail("bag1 did not open in BagsUI")
		return false
	await uo_settle(300)
	return true

func uo_overlap_merchant(client: Node) -> bool:
	var frame := uo_ctl(client, UO_MERCHANT, "MerchantFrame")
	var bag := uo_ctl(client, UO_BAGS, "ContainerFrame1")
	var canvas := uo_ctl(client, UO_MERCHANT, "RegistryCanvas")
	var scale := canvas.get_global_transform().get_scale().x
	var m := frame.get_global_rect()
	var b := bag.get_global_rect()
	var s0 := uo_center(client, UO_BAGS, "ContainerFrame1Slot0")
	# Merchant right edge at bag1's horizontal center (covers its left half), bottom
	# edge at the embedded backpack top so the two MerchantUI windows stay disjoint.
	var embedded := uo_ctl(client, UO_MERCHANT, "ContainerFrame0").get_global_rect()
	var left := b.get_center().x - m.size.x
	var top := maxf(0.0, embedded.position.y - m.size.y)
	var title := m.position + Vector2(m.size.x * 0.4, 10.0 * scale)
	var target := title + Vector2(left, top) - m.position
	print("UIOWN GEOMETRY before merchant=", m, " bag1=", b, " slot0=", s0, " scale=", scale, " title_press=", title, " title_target=", target)
	await drag_title(title, target)
	await uo_settle(300)
	m = frame.get_global_rect()
	print("UIOWN GEOMETRY after merchant=", m, " embedded_bag0=", uo_ctl(client, UO_MERCHANT, "ContainerFrame0").get_global_rect())
	for slot in range(8):
		var c := uo_center(client, UO_BAGS, "ContainerFrame1Slot%s" % slot)
		var covered := m.has_point(c)
		print("UIOWN GEOMETRY bag1 slot", slot, " center=", c, " under_merchant=", covered, " hovered=", uo_hover_path(c))
		if covered and uo_bag1_covered.is_empty() and uo_hover_path(c).ends_with("/MerchantFrame"):
			uo_bag1_covered = "ContainerFrame1Slot%s" % slot
			uo_merchant_over_bag = c
		if not covered and slot in [0, 3] and uo_bag1_source.is_empty():
			uo_bag1_source = "ContainerFrame1Slot%s" % slot
		if not covered and not (slot in [0, 3]) and uo_bag1_free.is_empty():
			uo_bag1_free = "ContainerFrame1Slot%s" % slot
	# A merchant background point outside every BagsUI control.
	for fraction in [Vector2(0.5, 0.33), Vector2(0.5, 0.5), Vector2(0.3, 0.4), Vector2(0.7, 0.4)]:
		var p: Vector2 = m.position + m.size * fraction
		if not b.has_point(p) and mc_outside_active_hit(uo_ui(client, UO_BAGS), p) == null and uo_hover_path(p).ends_with("/MerchantFrame"):
			uo_merchant_clear = p
			break
	print("UIOWN GEOMETRY source=", uo_bag1_source, " covered=", uo_bag1_covered, " free=", uo_bag1_free, " merchant_over_bag=", uo_merchant_over_bag, " merchant_clear=", uo_merchant_clear, " clear_hovered=", uo_hover_path(uo_merchant_clear))
	if uo_bag1_source.is_empty() or uo_bag1_covered.is_empty() or uo_merchant_clear == Vector2.INF:
		fail("overlap geometry not established")
		return false
	return true

# ---------- property 1: target ----------

func uo_target_cases(client: Node) -> void:
	var embedded := uo_center(client, UO_MERCHANT, "ContainerFrame0Slot0")
	var source := uo_center(client, UO_BAGS, uo_bag1_source)
	await uo_begin("T1_PRESS_MERCHANT_OVER_BAG_EMPTY_CURSOR")
	await uo_click(uo_merchant_over_bag)
	await uo_end("T1_PRESS_MERCHANT_OVER_BAG_EMPTY_CURSOR", client, "expect no pickup of covered bag1 item, 0 requests; hovered=" + uo_hover_path(uo_merchant_over_bag))

	print("UIOWN CAMERA before T2 ", uo_camera())
	await uo_begin("T2_DRAG_EMBEDDED_BAG0_TO_MERCHANT_OVER_BAG1")
	await uo_drag(embedded, uo_merchant_over_bag)
	await uo_end("T2_DRAG_EMBEDDED_BAG0_TO_MERCHANT_OVER_BAG1", client, "expect exactly 1 SellItem guid7100001, 0 SwapItem")

	print("UIOWN CAMERA after T2 ", uo_camera())
	await uo_begin("T3_DRAG_BAGSUI_BAG1_TO_MERCHANT_OVER_BAG1")
	await uo_drag(source, uo_merchant_over_bag)
	await uo_end("T3_DRAG_BAGSUI_BAG1_TO_MERCHANT_OVER_BAG1", client, "expect exactly 1 SwapItem Bag{1,3} to Bag{1,0}: the press raises the bags, so bag1 slot0 is topmost at release")

	await uo_begin("T4_DRAG_BAGSUI_BAG1_TO_MERCHANT_CLEAR")
	await uo_drag(source, uo_merchant_clear)
	await uo_end("T4_DRAG_BAGSUI_BAG1_TO_MERCHANT_CLEAR", client, "expect exactly 1 SellItem (bag1 item)")

	await uo_begin("T5_CLICK_BAGSUI_BAG1_THEN_CLICK_MERCHANT")
	await uo_click(source)
	await uo_settle(300)
	var held := uo_cursor_held(client)
	await uo_click(uo_merchant_clear)
	await uo_end("T5_CLICK_BAGSUI_BAG1_THEN_CLICK_MERCHANT", client, "held_after_first_click=%s expect exactly 1 SellItem" % held)

	await uo_begin("T6_BAG_RAISE_ON_CLICK")
	var free := uo_center(client, UO_BAGS, uo_bag1_free) if not uo_bag1_free.is_empty() else uo_center(client, UO_BAGS, "ContainerFrame1Title")
	await uo_click(free)
	await uo_settle(300)
	var hovered := uo_hover_path(uo_merchant_over_bag)
	await uo_end("T6_BAG_RAISE_ON_CLICK", client, "after clicking bag1 at %s, topmost at covered point=%s (Retail ContainerFrameContainer toplevel: BagsUI expected)" % [free, hovered])

# ---------- property 2: order ----------

func uo_order_cases(client: Node) -> void:
	var embedded := uo_center(client, UO_MERCHANT, "ContainerFrame0Slot0")
	var source := uo_center(client, UO_BAGS, uo_bag1_source)
	await uo_begin("O1_SAME_FRAME_DRAG_SELL")
	mc_motion(embedded, false)
	await process_frame
	var frame := Engine.get_process_frames()
	mc_edge(embedded, true)
	mc_motion(uo_merchant_clear, true)
	mc_edge(uo_merchant_clear, false)
	var same := Engine.get_process_frames() == frame
	await uo_end("O1_SAME_FRAME_DRAG_SELL", client, "one_frame=%s expect exactly 1 SellItem guid7100001, cursor empty" % same)

	await uo_begin("O2_SAME_FRAME_CLICK_THEN_SAME_FRAME_DROP")
	mc_motion(embedded, false)
	await process_frame
	mc_edge(embedded, true)
	mc_edge(embedded, false)
	await uo_settle(300)
	var held := uo_cursor_held(client)
	mc_motion(uo_merchant_clear, false)
	await process_frame
	mc_edge(uo_merchant_clear, true)
	mc_edge(uo_merchant_clear, false)
	await uo_end("O2_SAME_FRAME_CLICK_THEN_SAME_FRAME_DROP", client, "held_after_click=%s expect exactly 1 SellItem guid7100001" % held)

	await uo_begin("O3_ONE_FRAME_MERCHANTUI_CLICK_THEN_BAGSUI_CLICK")
	mc_motion(embedded, false)
	await process_frame
	frame = Engine.get_process_frames()
	mc_edge(embedded, true)
	mc_edge(embedded, false)
	mc_motion(source, false)
	mc_edge(source, true)
	mc_edge(source, false)
	same = Engine.get_process_frames() == frame
	await uo_end("O3_ONE_FRAME_MERCHANTUI_CLICK_THEN_BAGSUI_CLICK", client, "one_frame=%s expect exactly 1 SwapItem from Bag{0,0} to Bag{1,%s}" % [same, uo_bag1_source])

	await uo_begin("O4_ONE_FRAME_BAGSUI_CLICK_THEN_MERCHANTUI_CLICK")
	mc_motion(source, false)
	await process_frame
	frame = Engine.get_process_frames()
	mc_edge(source, true)
	mc_edge(source, false)
	mc_motion(embedded, false)
	mc_edge(embedded, true)
	mc_edge(embedded, false)
	same = Engine.get_process_frames() == frame
	await uo_end("O4_ONE_FRAME_BAGSUI_CLICK_THEN_MERCHANTUI_CLICK", client, "one_frame=%s expect exactly 1 SwapItem from Bag{1,%s} to Bag{0,0}" % [same, uo_bag1_source])

	await uo_begin("O5_ONE_FRAME_DRAG_SELL_THEN_CLICK_PICKUP")
	mc_motion(embedded, false)
	await process_frame
	frame = Engine.get_process_frames()
	mc_edge(embedded, true)
	mc_motion(uo_merchant_clear, true)
	mc_edge(uo_merchant_clear, false)
	mc_motion(source, false)
	mc_edge(source, true)
	mc_edge(source, false)
	same = Engine.get_process_frames() == frame
	await uo_end("O5_ONE_FRAME_DRAG_SELL_THEN_CLICK_PICKUP", client, "one_frame=%s expect exactly 1 SellItem guid7100001 then bag1 item on cursor (cursor_held=true), 0 SwapItem" % same)

# ---------- world release ----------

func uo_world_point(client: Node) -> Vector2:
	var viewport := root.get_visible_rect()
	for fraction in [Vector2(0.5, 0.5), Vector2(0.6, 0.3), Vector2(0.75, 0.25), Vector2(0.6, 0.6)]:
		var point: Vector2 = viewport.position + viewport.size * fraction
		if mc_outside_active_hit(client, point) == null:
			return point
	return Vector2.INF

func uo_world_case(client: Node) -> void:
	if not await uo_reopen(client, "W1"):
		return
	await uo_settle(300)
	var embedded := uo_center(client, UO_MERCHANT, "ContainerFrame0Slot1")
	var world := uo_world_point(client)
	if world == Vector2.INF:
		print("UIOWN SETUP_WARN no world point")
		return
	print("UIOWN CAMERA before W1 ", uo_camera())
	await uo_begin("W1_DRAG_BAG_ITEM_TO_WORLD")
	await uo_drag(embedded, world)
	await uo_settle(300)
	var popups := 0
	for node in client.find_children("StaticPopup*", "Control", true, false):
		if str(node.name).length() == 12 and (node as Control).is_visible_in_tree():
			popups += 1
	print("UIOWN CAMERA after W1 drag ", uo_camera())
	var popup_node := client.find_child("StaticPopup1", true, false)
	if popup_node != null:
		var names := []
		for child in popup_node.find_children("*", "Control", true, false):
			names.append(str(child.name))
		print("UIOWN POPUP path=", popup_node.get_path(), " children=", names)
	var watch_until := Time.get_ticks_msec() + 6000
	var last := ""
	while Time.get_ticks_msec() < watch_until:
		var now := "held=%s popup=%s" % [uo_cursor_held(client), uo_popup(client)]
		if now != last:
			print("UIOWN W1 WATCH t=", Time.get_ticks_msec(), " frame=", Engine.get_process_frames(), " ", now)
			last = now
		await process_frame
	await uo_end("W1_DRAG_BAG_ITEM_TO_WORLD", client, "world=%s hovered=%s visible_popups=%s expect DELETE_ITEM popup once, 0 requests, cursor held" % [world, uo_hover_path(world), popups])

# ---------- property 3: open/close ----------

func uo_reopen(client: Node, case_name: String) -> bool:
	if client.merchant_state().open:
		return true
	var vendor := await uo_find_vendor(client)
	var point: Vector2
	if vendor.is_empty():
		# No unit under the pointer: right-click interacts with the current target.
		point = uo_world_point(client)
		print("UIOWN REOPEN ", case_name, " vendor not ray-pickable (camera=", uo_camera(), " target=", client.target_state(), "); right-click world point ", point)
	else:
		point = vendor.point
	pointer_at(point, MOUSE_BUTTON_RIGHT, true)
	await process_frame
	pointer_at(point, MOUSE_BUTTON_RIGHT, false)
	var opened := await uo_wait(func(): return client.merchant_state().open and uo_visible(uo_ctl(client, UO_MERCHANT, "MerchantFrame")), 6000)
	print("UIOWN REOPEN ", case_name, " opened=", opened)
	return opened

func uo_pick_vendor_item(client: Node) -> bool:
	var cell := uo_center(client, UO_MERCHANT, MC_SOURCE)
	if cell == Vector2.INF:
		return false
	await uo_click(cell)
	return await uo_wait(func(): return uo_cursor_held(client), 2000)

func uo_escape() -> void:
	push_key(KEY_ESCAPE, true)
	await process_frame
	push_key(KEY_ESCAPE, false)
	await process_frame

func uo_close_cases(client: Node) -> void:
	if not client.merchant_state().open:
		await uo_reopen(client, "C1")
	if not uo_visible(uo_ctl(client, UO_BAGS, "ContainerFrame1")):
		await uo_open_bag1(client)
	uo_obs("C1_BEFORE", client)
	await uo_begin("C1_ESCAPE_EMPTY_CURSOR")
	await uo_escape()
	await uo_end("C1_ESCAPE_EMPTY_CURSOR", client, "embedded_bag0=%s expect merchant+backpack closed, bag1 closed (Retail CloseAllBags), exactly 1 CloseInteraction" % uo_visible(uo_ctl(client, UO_MERCHANT, "ContainerFrame0")))

	if not await uo_reopen(client, "C2"):
		return
	await uo_settle(300)
	await uo_begin("C2_ESCAPE_WITH_VENDOR_CURSOR")
	var picked := await uo_pick_vendor_item(client)
	await uo_escape()
	await uo_settle(300)
	var after_first := "cursor_held=%s merchant_open=%s" % [uo_cursor_held(client), client.merchant_state().open]
	await uo_escape()
	await uo_end("C2_ESCAPE_WITH_VENDOR_CURSOR", client, "picked=%s after_first_escape=[%s] expect first Escape clears cursor only, second closes; exactly 1 CloseInteraction, 0 BuyItem" % [picked, after_first])

	if not await uo_reopen(client, "C3"):
		return
	await uo_settle(300)
	await uo_begin("C3_SERVER_CLOSE_WITH_VENDOR_CURSOR")
	picked = await uo_pick_vendor_item(client)
	print("FIXTURE UIOWN_SERVER_CLOSE")
	await uo_settle()
	var icon_after_close := uo_cursor_held(client)
	var bag1_open := uo_visible(uo_ctl(client, UO_BAGS, "ContainerFrame1"))
	if not bag1_open:
		await uo_open_bag1(client)
	var empty := uo_center(client, UO_BAGS, "ContainerFrame1Slot5")
	await uo_click(empty)
	await uo_end("C3_SERVER_CLOSE_WITH_VENDOR_CURSOR", client, "picked=%s icon_after_close=%s bag1_open_after_close=%s then clicked bag1 slot5; expect merchant closed, icon hidden, 0 CloseInteraction, 0 BuyItem" % [picked, icon_after_close, bag1_open])

	if not await uo_reopen(client, "C4"):
		return
	await uo_settle(300)
	await uo_begin("C4_CLOSE_BUTTON_WITH_VENDOR_CURSOR_THEN_BAG_CLICK")
	picked = await uo_pick_vendor_item(client)
	var close := uo_center(client, UO_MERCHANT, "MerchantFrameCloseButton")
	await uo_click(close)
	await uo_settle(300)
	icon_after_close = uo_cursor_held(client)
	if not uo_visible(uo_ctl(client, UO_BAGS, "ContainerFrame1")):
		await uo_open_bag1(client)
	await uo_click(uo_center(client, UO_BAGS, "ContainerFrame1Slot5"))
	await uo_end("C4_CLOSE_BUTTON_WITH_VENDOR_CURSOR_THEN_BAG_CLICK", client, "picked=%s icon_after_close=%s expect exactly 1 CloseInteraction, 0 BuyItem" % [picked, icon_after_close])

func uo_any_backpack(client: Node) -> String:
	var shown := []
	for node in client.find_children("ContainerFrame0", "Control", true, false):
		if (node as Control).is_visible_in_tree():
			shown.append(str(node.get_path()))
	return str(shown)

func uo_ah_visible(client: Node) -> bool:
	for frame_name in ["AuctionHouseFrame", "AuctionFrame"]:
		var node := client.find_child(frame_name, true, false) as Control
		if uo_visible(node):
			return true
	return false

func uo_auction_cases(client: Node) -> void:
	# Retail OpenAllBags is a no-op while any bag is open (IsAnyBagOpen): start with none.
	if uo_visible(uo_ctl(client, UO_BAGS, "ContainerFrame1")) or client.merchant_state().open:
		await uo_escape()
		await uo_settle(300)
	uo_obs("A1_BEFORE", client)
	await uo_begin("A1_AUCTION_OPEN_ESCAPE")
	print("FIXTURE UIOWN_AH_OPEN")
	var opened := await uo_wait(func(): return uo_ah_visible(client), 6000)
	await uo_settle(300)
	var backpack := uo_any_backpack(client)
	await uo_escape()
	await uo_settle(300)
	await uo_end("A1_AUCTION_OPEN_ESCAPE", client, "ah_opened=%s backpack_while_open=%s ah_visible_after_escape=%s expect backpack shown (Retail OpenAllBags), Escape closes AH+bags, exactly 1 CloseInteraction" % [opened, backpack, uo_ah_visible(client)])

	await uo_begin("A2_AUCTION_SERVER_CLOSE")
	print("FIXTURE UIOWN_AH_OPEN")
	opened = await uo_wait(func(): return uo_ah_visible(client), 6000)
	await uo_settle(300)
	print("FIXTURE UIOWN_SERVER_CLOSE")
	await uo_settle()
	await uo_end("A2_AUCTION_SERVER_CLOSE", client, "ah_opened=%s ah_visible_after_server_close=%s expect closed, 0 CloseInteraction" % [opened, uo_ah_visible(client)])

const UO_CHARACTER := "CharacterFrameUI"

func uo_toggle_character(client: Node) -> void:
	push_key(KEY_C, true)
	await process_frame
	push_key(KEY_C, false)
	await uo_settle(300)

# Paperdoll slot name -> global centre, read while the CharacterFrame shows.
func uo_paperdoll_centers(client: Node) -> Dictionary:
	var centers := {}
	var ui := uo_ui(client, UO_CHARACTER)
	if ui == null:
		return centers
	for control in ui.find_children("Character*Slot", "Control", true, false):
		if (control as Control).is_visible_in_tree():
			centers[str(control.name)] = (control as Control).get_global_rect().get_center()
	return centers

# E1: a paperdoll slot over the merchant's embedded backpack slot. A drag from BagsUI
# bag1 (raised by the press, away from both) releases on that paperdoll slot: exactly
# one SwapItem to its equipment slot, nothing to the covered bag slot.
func uo_equipment_case(client: Node) -> void:
	await uo_toggle_character(client)
	var slots := uo_paperdoll_centers(client)
	var frame_ctl := uo_ctl(client, UO_CHARACTER, "CharacterFrame")
	var frame_rect := frame_ctl.get_global_rect() if uo_visible(frame_ctl) else Rect2()
	await uo_toggle_character(client)
	if slots.size() != 18:
		fail("C did not show the CharacterFrame with 18 paperdoll slots: " + str(slots.keys()))
		return
	if not await uo_reopen(client, "E1"):
		fail("vendor reopen for E1 failed")
		return
	if not uo_visible(uo_ctl(client, UO_BAGS, "ContainerFrame1")) and not await uo_open_bag1(client):
		return
	var target := await uo_cover_embedded_slot(client, slots)
	if target.is_empty():
		return
	var point: Vector2 = slots[target]
	await uo_toggle_character(client)
	# Raise the CharacterFrame over the merchant: press an uncovered part of it.
	var raise := uo_uncovered_frame_point(frame_rect)
	if raise == Vector2.INF:
		fail("E1 geometry: the merchant covers the whole CharacterFrame")
		return
	await uo_click(raise)
	await uo_settle(300)
	var top_at_slot := uo_hover_path(point)
	var source := uo_center(client, UO_BAGS, "ContainerFrame1Slot3")
	print("UIOWN GEOMETRY E1 slot=", target, " at ", point, " topmost=", top_at_slot, " raise=", raise, " source=", source, " source_hovered=", uo_hover_path(source))
	if not top_at_slot.ends_with("/" + target) or source == Vector2.INF:
		fail("E1 geometry: " + target + " not topmost over the embedded backpack")
		return
	await uo_begin("E1_EQUIP_OVER_BAG")
	await uo_drag(source, point)
	await uo_end("E1_EQUIP_OVER_BAG", client, "slot=%s character_visible=%s expect exactly 1 SwapItem Bag{1,3} to that Equipment slot, nothing to the covered ContainerFrame0 slot" % [target, uo_visible(uo_ctl(client, UO_CHARACTER, "CharacterFrame"))])
	await uo_escape()
	await uo_settle(300)

# Drag the merchant by its title so an embedded backpack slot centres on a paperdoll
# slot, keeping the merchant on screen; the hover there then names that bag slot (the
# title press raised the merchant). Returns the covering paperdoll slot's name.
func uo_cover_embedded_slot(client: Node, slots: Dictionary) -> String:
	var frame := uo_ctl(client, UO_MERCHANT, "MerchantFrame")
	var canvas := uo_ctl(client, UO_MERCHANT, "RegistryCanvas")
	var scale := canvas.get_global_transform().get_scale().x
	var screen := Vector2(root.size)
	var m := frame.get_global_rect()
	var bag := uo_ctl(client, UO_MERCHANT, "ContainerFrame0").get_global_rect()
	var extent := m.merge(bag)
	print("UIOWN GEOMETRY E1 merchant=", m, " embedded_bag0=", bag, " screen=", screen, " slots=", slots)
	for target in slots:
		var point: Vector2 = slots[target]
		for slot in range(16):
			var name := "ContainerFrame0Slot%s" % slot
			var center := uo_center(client, UO_MERCHANT, name)
			if center == Vector2.INF:
				continue
			var moved := extent
			moved.position += point - center
			if moved.position.x < 0.0 or moved.position.y < 0.0 or moved.end.x > screen.x or moved.end.y > screen.y:
				continue
			var title := m.position + Vector2(m.size.x * 0.4, 10.0 * scale)
			await drag_title(title, title + point - center)
			await uo_settle(300)
			var hovered := uo_hover_path(point)
			print("UIOWN GEOMETRY E1 moved merchant=", frame.get_global_rect(), " ", name, " at ", uo_center(client, UO_MERCHANT, name), " under ", target, " hovered=", hovered)
			if hovered.ends_with("/" + name):
				return target
			fail("E1 geometry: " + name + " not under " + target + ": " + hovered)
			return ""
	fail("E1 geometry: no embedded backpack slot fits under a paperdoll slot")
	return ""

# A CharacterFrame point (not a paperdoll slot) the pointer reaches on top.
func uo_uncovered_frame_point(rect: Rect2) -> Vector2:
	for y in [0.15, 0.3, 0.5, 0.7, 0.9]:
		for x in [0.02, 0.1, 0.5, 0.9, 0.97]:
			var point := rect.position + rect.size * Vector2(x, y)
			var path := uo_hover_path(point)
			if path.contains("/" + UO_CHARACTER + "/") and not path.ends_with("Slot"):
				return point
	return Vector2.INF
