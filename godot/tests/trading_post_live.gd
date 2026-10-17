extends SceneTree
# Trading Post live proof: native input and read-only observations against an owned
# private server only. TP_PHASE=browse captures the Log, categories, previews, freeze
# and purchase; TP_PHASE=relog proves the frozen slot and purchase survived.
var client: Node
var shots := OS.get_environment("TP_SHOTS")
var receipts := OS.get_environment("TP_RECEIPTS")
var skin := OS.get_environment("TP_SKIN")
var phase := OS.get_environment("TP_PHASE")
var server := OS.get_environment("TP_SERVER")
var menu_audio_events := []
var paused_private_server := 0
# Retail70391 SoundKitEntry rows for218932/218933.
const MENU_OPEN_FILES := [4956048,4956050,4956052,4956054,4956056]
const MENU_CLOSE_FILES := [4555731,4555733,4555735,4555737,4555739,4555741,4555743,4555745,4555747,4555749]

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_fixture")

func run_fixture() -> void:
	root.size = Vector2i(1920, 1080)
	if DisplayServer.screen_get_size() != Vector2i(1920, 1080):
		fail("Private compositor must expose 1920x1080 output")
		return
	var account := OS.get_environment("TP_ACCOUNT")
	if server != "127.0.0.1:5791" or not account.begins_with("fb_") or shots.is_empty() or receipts.is_empty() or not phase in ["refund-icon-tooltip", "row-cart-toggle", "frozen-tooltip", "cart-tooltips", "pending-dialogs", "confirmations", "button-tooltips", "item-tooltips", "menu-sounds", "layout", "layout-proof", "browse", "relog", "purchase", "refund", "cart", "cart-lifetime", "rotate-controls", "mount-rider", "mount-rider-cancel", "freeze-workflow", "authored-camera", "frozen-slot-before", "frozen-slot-after"]:
		fail("Explicit private 127.0.0.1:5791, fb_* account, phase and evidence paths required")
		return
	for directory in [shots, receipts]:
		if DirAccess.make_dir_recursive_absolute(directory) != OK:
			push_error("Cannot create Trading Post evidence directory: " + directory)
			quit(1)
			return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_until(func(): return not client.account_state().assets_starting, "asset startup", 180000): return
	var error: String = client.connect_account(server, account, OS.get_environment("TP_PASSWORD"), false)
	if error != "": fail(error); return
	if not await wait_until(func(): return control("CharCard_0") != null and control("CharCard_0").is_visible_in_tree(), "character select reveal", 180000): return
	await click("CharCard_0")
	if not await wait_until(func(): return control("EnterWorld") != null and control("EnterWorld").is_visible_in_tree(), "selected character ready", 180000): return
	await click("EnterWorld")
	if not await wait_until(func(): return client.account_state().screen == "InWorld" and control("MinimapLauncherButton") != null, "world", 180000): return
	var expected := "Modern" if skin == "modern" else "Forever"
	if client.account_state().ui_skin != expected:
		fail("Active skin %s, expected %s" % [client.account_state().ui_skin, expected]); return
	if not await wait_until(func(): return float(client.account_state().local_player_health) > 0.0, "private character alive", 120000): return
	client.set_world_minutes(720.0)
	await wait_frames(120)
	if not phase in ["refund-icon-tooltip", "row-cart-toggle", "frozen-tooltip", "cart-tooltips", "pending-dialogs", "confirmations", "button-tooltips", "item-tooltips", "menu-sounds", "layout", "layout-proof", "cart", "cart-lifetime", "rotate-controls", "mount-rider", "mount-rider-cancel", "freeze-workflow", "authored-camera", "frozen-slot-before", "frozen-slot-after"] and not await travelers_log(): return
	if phase == "menu-sounds" and not watch_menu_sound_players(): return
	if not await open_vendor(): return
	if phase in ["cart", "cart-lifetime"]:
		# Collection UI owners may replace a hidden CanvasLayer while this vendor is open.
		var old_layer := CanvasLayer.new()
		old_layer.name = "TradingPostLayerLifetimeFixture"
		client.add_child(old_layer)
		await wait_frames(3)
		old_layer.free()
		var new_layer := CanvasLayer.new()
		new_layer.name = "TradingPostLayerLifetimeFixture"
		client.add_child(new_layer)
		await wait_frames(3)
		if new_layer.visible: fail("Replacement UI owner stayed visible over vendor"); return
	match phase:
		"layout":
			await capture("vendor-all")
			await click("PerksProgramFilter")
			if not await wait_until(func(): return control("PerksProgramFilterMenu") != null, "filter description"): return
			await capture("category-dropdown")
			await choose_category(1)
			await capture("vendor-transmog")
			finish("layout")
		"layout-proof": await layout_proof()
		"relog": await relog_checks()
		"purchase": await purchase_each_collection()
		"cart": await checkout_cart()
		"cart-lifetime": await finish("layer-lifetime")
		"rotate-controls": await rotate_control_proof()
		"mount-rider": await mount_rider_proof()
		"mount-rider-cancel": await mount_rider_cancel_proof()
		"freeze-workflow": await freeze_workflow_proof()
		"authored-camera": await authored_camera_proof()
		"menu-sounds": await menu_sound_proof()
		"item-tooltips": await item_tooltip_proof()
		"confirmations": await confirmation_escape_proof()
		"pending-dialogs": await pending_dialog_proof()
		"frozen-tooltip": await frozen_item_tooltip_proof()

		"button-tooltips": await button_tooltip_proof()
		"refund-icon-tooltip": await refund_icon_tooltip_proof()
		"row-cart-toggle": await row_cart_toggle_proof()
		"cart-tooltips": await cart_item_tooltip_proof()
		"frozen-slot-before": await frozen_slot_geometry_proof(false)
		"frozen-slot-after": await frozen_slot_geometry_proof(true)
		"refund": await refund_after_relog()
		_: await browse()

func refund_icon_tooltip_proof() -> void:
	var before: Dictionary = client.trading_post_state().duplicate(true)
	if not await choose_category(1): return
	var target := await select_other_cartable([])
	if target <= 0: return
	var product: Dictionary = client.trading_post_state().duplicate(true)
	var price := int(product.price)
	await click("PerksProgramPurchaseButton")
	await click("PerksProgramConfirmButton")
	if not await wait_until(func(): return client.trading_post_state().purchased and not client.trading_post_state().pending and int(client.trading_post_state().tender) == int(before.tender) - price, "refund tooltip genuine purchase", 120000): return
	if int(client.trading_post_state().refund_seconds) <= 0:
		fail("Refund tooltip needs explicit private refund window, not production policy"); return
	var other := await select_other_cartable([target])
	if other <= 0: return
	var row := control("PerksProgramProduct%d" % target)
	if row == null: fail("Purchased refund row not visible"); return
	var scale := control("PerksProgramLeaveButton").get_global_rect().size.x / 180.0
	var icon := Rect2(row.get_global_rect().position + Vector2(389, 13.5) * scale, Vector2(21, 21) * scale)
	await motion(icon.get_center(), false)
	await wait_frames(30)
	var tooltip: Dictionary = client.tooltip_state().duplicate(true)
	var prefix := "Time remaining to refund this item: "
	var matched := str(tooltip.get("title", "")).begins_with(prefix)
	await capture("refund-hover")
	var anchor_ok := matched and absf(float(tooltip.rect[0]) - icon.end.x / scale) <= 1.5 and absf(float(tooltip.rect[1]) + float(tooltip.rect[3]) - icon.position.y / scale) <= 1.5
	var state: Dictionary = client.trading_post_state()
	var conserved: bool = int(state.selected) == other and int(state.tender) == int(before.tender) - price and state.frozen == before.frozen and state.cart.is_empty()
	await motion(row.get_global_rect().position + Vector2(96, 24) * scale, false)
	if not await wait_until(func(): return client.tooltip_state().get("title", "") == str(product.selected_name), "refund icon leave restores product tooltip"): return
	await motion(control("PerksProgramLeaveButton").get_global_rect().get_center(), false)
	if not await wait_until(func(): return not client.tooltip_state().get("visible", false), "refund icon pointer leave"): return
	await capture("refund-leave")
	# Restore the genuine purchase even when the tooltip assertion is RED.
	await click("PerksProgramLeaveButton")
	if not await wait_until(func(): return not client.trading_post_state().open and not client.tooltip_state().get("visible", false), "refund tooltip vendor teardown"): return
	if not await open_vendor(): return
	if not await choose_category(1): return
	await click("PerksProgramProduct%d" % target)
	if not await wait_until(func(): return int(client.trading_post_state().selected) == target, "purchased product reselect"): return
	if control("PerksProgramRefundButton") == null:
		fail("Private refund expired before cleanup"); return
	await click("PerksProgramRefundButton")
	await click("PerksProgramConfirmButton")
	if not await wait_until(func(): return not client.trading_post_state().purchased and not client.trading_post_state().pending and int(client.trading_post_state().tender) == int(before.tender), "refund tooltip restores authoritative Tender", 120000): return
	if client.trading_post_state().frozen != before.frozen:
		fail("Refund tooltip roundtrip changed frozen reservation"); return
	await capture("refunded-restored")
	print("TP_REFUND_ICON_TOOLTIP target=", target, " selected=", other, " tooltip=", tooltip, " matched=", matched, " anchor_ok=", anchor_ok, " conserved=", conserved)
	if not matched:
		fail("Retail row refund icon tooltip missing; genuine purchase refunded before assertion"); return
	if not anchor_ok or not conserved:
		fail("Refund icon tooltip lost source owner anchor or changed selection/authority"); return
	await finish("refund-icon-tooltip")

func row_cart_toggle_proof() -> void:
	if not await choose_category(1): return
	var target := await select_other_cartable([])
	if target <= 0: return
	var target_name: String = client.trading_post_state().selected_name
	var target_item := int(client.trading_post_state().item_id)
	var other := await select_other_cartable([target])
	if other <= 0: return
	var initial: Dictionary = client.trading_post_state().duplicate(true)
	var owner := "PerksProgramCartToggle%d" % target
	var row := "PerksProgramProduct%d" % target
	var scale := control("PerksProgramLeaveButton").get_global_rect().size.x / 180.0
	await motion(control("PerksProgramLeaveButton").get_global_rect().get_center(), false)
	if control(owner) != null: fail("Row cart toggle visible without row hover"); return
	await motion(control(row).get_global_rect().position + Vector2(96, 24) * scale, false)
	if not await wait_until(func(): return control(owner) != null, "Retail row cart toggle on eligible product hover", 10000): return
	var rect := control(owner).get_global_rect()
	var expected := control(row).get_global_rect().position + Vector2(443, 6) * scale
	if rect.position.distance_to(expected) > 1.5 or rect.size.distance_to(Vector2(36, 36) * scale) > 1.5:
		fail("Row cart toggle lacks source36x36/TimeRemaining center anchor"); return
	await wait_frames(3)
	await RenderingServer.frame_post_draw
	var glyph_rect := Rect2i(Vector2i(rect.position + Vector2(8, 8) * scale), Vector2i(Vector2(20, 20) * scale))
	var normal_contrast := cart_glyph_contrast(root.get_texture().get_image().get_region(glyph_rect))
	await motion(rect.get_center(), false)
	if not await wait_until(func(): return client.tooltip_state().get("title", "") == "Add to Cart", "row cart Add tooltip"): return
	await wait_frames(3)
	await RenderingServer.frame_post_draw
	var hover_contrast := cart_glyph_contrast(root.get_texture().get_image().get_region(glyph_rect))
	print("TP_ROW_CART_GLYPH_CONTRAST normal=", normal_contrast, " hover=", hover_contrast)
	if normal_contrast <= 0.0 or hover_contrast < normal_contrast * 0.25:
		fail("Hovered row cart glyph lost base contrast under highlight"); return
	var tooltip: Dictionary = client.tooltip_state()
	if absf(float(tooltip.rect[0]) + float(tooltip.rect[2]) - rect.position.x / scale) > 1.5 or absf(float(tooltip.rect[1]) + float(tooltip.rect[3]) - rect.position.y / scale) > 1.5:
		fail("Row cart tooltip lacks source ANCHOR_LEFT0/0"); return
	await capture("add-hover")
	await button_event(rect.get_center(), MOUSE_BUTTON_LEFT, true)
	await capture("pressed")
	await button_event(rect.get_center(), MOUSE_BUTTON_LEFT, false)
	if not await wait_until(func(): return target in client.trading_post_state().cart, "row Add toggles hovered item"): return
	if not await wait_until(func(): return client.tooltip_state().get("title", "") == "Remove from Cart", "row tooltip tracks cart membership"): return
	var added: Dictionary = client.trading_post_state()
	if int(added.selected) != other or int(added.tender) != int(initial.tender) or int(added.frozen) != int(initial.frozen) or added.pending:
		fail("Row cart toggle changed selected preview/Tender/frozen authority"); return
	await capture("remove-hover")
	await click(owner)
	if not await wait_until(func(): return client.trading_post_state().cart == initial.cart, "row Remove restores initial cart"): return
	await motion(control(row).get_global_rect().position + Vector2(96, 24) * scale, false)
	if not await wait_until(func(): return client.tooltip_state().get("title", "") == target_name, "leaving cart toggle restores product tooltip"): return
	await capture("row-tooltip-restored")
	await motion(control("PerksProgramLeaveButton").get_global_rect().get_center(), false)
	if not await wait_until(func(): return control(owner) == null and not client.tooltip_state().get("visible", false), "row leave hides toggle and tooltip"): return
	if not await hover_row_cart_toggle(target, "Add to Cart"): return
	await click(owner)
	await click("PerksProgramViewCartButton")
	if not await wait_until(func(): return client.trading_post_state().cart_visible, "open row-added cart"): return
	if not target_item in client.trading_post_state().cart_preview_items:
		fail("Row-added item not initially selected in open cart"); return
	await click("PerksProgramCartItem%d" % target)
	if not await wait_until(func(): return not target_item in client.trading_post_state().cart_preview_items, "disable existing cart preview before append"): return
	if not await hover_row_cart_toggle(other, "Add to Cart"): return
	await click("PerksProgramCartToggle%d" % other)
	if not await wait_until(func(): return other in client.trading_post_state().cart and int(initial.item_id) in client.trading_post_state().cart_preview_items, "append open-cart wearable default"): return
	if target_item in client.trading_post_state().cart_preview_items:
		fail("Open-cart row add reset disabled preview choice"); return
	await capture("open-cart-added")
	await click("PerksProgramCartToggle%d" % other)
	if not await hover_row_cart_toggle(target, "Remove from Cart"): return
	await click(owner)
	if not await wait_until(func(): return client.trading_post_state().cart == initial.cart and not client.trading_post_state().cart_visible, "row remove closes emptied cart"): return
	var final: Dictionary = client.trading_post_state()
	if int(final.selected) != other or int(final.tender) != int(initial.tender) or int(final.frozen) != int(initial.frozen) or final.pending:
		fail("Row toggle roundtrip changed model/Tender/frozen authority"); return
	print("TP_ROW_CART_TOGGLE target=", target, " selected=", other, " tender=", final.tender, " frozen=", final.frozen)
	await finish("row-cart-toggle")

# Pixel structure, not node/texture existence: highlighted cart glyph remains painted.
func cart_glyph_contrast(image: Image) -> float:
	var total := 0.0
	var squares := 0.0
	var count := image.get_width() * image.get_height()
	for y in image.get_height():
		for x in image.get_width():
			var pixel := image.get_pixel(x, y)
			var brightness := (pixel.r + pixel.g + pixel.b) / 3.0
			total += brightness
			squares += brightness * brightness
	return sqrt(maxf(0.0, squares / count - pow(total / count, 2.0)))

func hover_row_cart_toggle(vendor: int, title: String) -> bool:
	var scale := control("PerksProgramLeaveButton").get_global_rect().size.x / 180.0
	var owner := "PerksProgramCartToggle%d" % vendor
	await motion(control("PerksProgramProduct%d" % vendor).get_global_rect().position + Vector2(96, 24) * scale, false)
	if not await wait_until(func(): return control(owner) != null, "row toggle visible on hover"): return false
	await motion(control(owner).get_global_rect().get_center(), false)
	return await wait_until(func(): return client.tooltip_state().get("title", "") == title, "row toggle tooltip " + title)

func frozen_item_tooltip_proof() -> void:
	var initial: Dictionary = client.trading_post_state().duplicate(true)
	if int(initial.frozen) < 0:
		await motion(control("PerksProgramFrozenSlot").get_global_rect().get_center(), false)
		if not await wait_until(func(): return not client.tooltip_state().get("visible", false), "empty frozen slot has no item tooltip"): return
		if not await choose_category(1): return
		if not await select_first_available(int(initial.tender)): return
		var candidate := int(client.trading_post_state().selected)
		if not await drag_to_frozen("PerksProgramProduct%d" % candidate): return
		if not await wait_until(func(): return int(client.trading_post_state().frozen) == candidate and not client.trading_post_state().pending, "real private frozen-item setup", 120000): return
	else:
		await click("PerksProgramFrozenSlot")
		if not await wait_until(func(): return int(client.trading_post_state().selected) == int(initial.frozen), "select actual occupied frozen product"): return
	var frozen: Dictionary = client.trading_post_state().duplicate(true)
	print("TP_FROZEN_TOOLTIP_SETUP initial=", initial.frozen, " used=", frozen.frozen, " item=", frozen.item_id, " tender=", frozen.tender)
	if int(frozen.category) != 1 and not await choose_category(1): return
	var other := await select_other_cartable([int(frozen.frozen)])
	if other <= 0: return
	await motion(control("PerksProgramFrozenSlot").get_global_rect().get_center(), false)
	if not await wait_until(func(): return client.tooltip_state().get("visible", false) and client.tooltip_state().get("title", "") == frozen.selected_name, "Retail inherited frozen product item tooltip", 10000): return
	var tooltip: Dictionary = client.tooltip_state()
	var lines := str(tooltip.get("lines", []))
	if not lines.contains("Item ID: %d" % int(frozen.item_id)) or lines.contains("Sell Price"):
		fail("Frozen tooltip lost displayed item identity or included SellPrice"); return
	var rect := control("PerksProgramFrozenSlot").get_global_rect()
	var scale := control("PerksProgramLeaveButton").get_global_rect().size.x / 180.0
	if absf(float(tooltip.rect[0]) - (rect.end.x / scale - 16.0)) > 1.5 or absf(float(tooltip.rect[1]) + float(tooltip.rect[3]) - rect.position.y / scale) > 1.5:
		fail("Frozen tooltip lacks inherited source Right−16/Y0 owner anchor"); return
	var state: Dictionary = client.trading_post_state()
	if int(state.selected) != other or int(state.frozen) != int(frozen.frozen) or int(state.tender) != int(initial.tender) or state.pending:
		fail("Frozen tooltip hover changed selected model/frozen authority/Tender"); return
	await capture("settled-frozen-hover")
	await motion(control("PerksProgramLeaveButton").get_global_rect().get_center(), false)
	if not await wait_until(func(): return not client.tooltip_state().get("visible", false), "frozen item leave clears tooltip"): return
	await capture("leave-hidden")
	# Keep genuine private setup state. Retail explicitly removes OnDragStart
	# from frozen products; do not use the engine's drag-off path as a source oracle.
	await finish("frozen-tooltip")

func pause_private_reply() -> bool:
	var pid := int(OS.get_environment("TP_SERVER_PID"))
	if server != "127.0.0.1:5791" or pid <= 1:
		fail("Pending proof requires host-verified owned private server PID"); return false
	var output := []
	if OS.execute("/usr/bin/kill", ["-STOP", str(pid)], output, true) != 0:
		fail("Cannot pause own private server: " + str(output)); return false
	paused_private_server = pid
	return true

func resume_private_reply() -> void:
	if paused_private_server > 1:
		var output := []
		var result := OS.execute("/usr/bin/kill", ["-CONT", str(paused_private_server)], output, true)
		print("TP_PRIVATE_SERVER_CONT pid=", paused_private_server, " code=", result)
		paused_private_server = 0

func assert_pending_painted() -> bool:
	var dialog := control("PerksProgramConfirmDialog")
	var spinner := control("PerksProgramConfirmSpinner")
	var overlay := control("PerksProgramConfirmDarkOverlay")
	var scale := dialog.get_global_rect().size.x / 320.0
	var expected := dialog.get_global_rect().grow(-11.0 * scale)
	if not overlay.get_global_rect().is_equal_approx(expected):
		fail("Pending dark overlay lacks source 11px inset"); return false
	# Counterfactual image controls, not forged UI state: freeze the host update
	# briefly so native visibility toggles survive until each actual frame draws.
	client.set_process(false)
	var center := dialog.get_global_rect().get_center()
	var rect := Rect2i(center - Vector2(30, 30) * scale, Vector2(60, 60) * scale)
	await RenderingServer.frame_post_draw
	var painted := root.get_texture().get_image().get_region(rect).get_data()
	spinner.hide()
	await process_frame
	await RenderingServer.frame_post_draw
	var without_spinner := root.get_texture().get_image().get_region(rect).get_data()
	overlay.hide()
	await process_frame
	await RenderingServer.frame_post_draw
	var without_overlay := root.get_texture().get_image().get_region(rect).get_data()
	overlay.show()
	spinner.show()
	client.set_process(true)
	if painted == without_spinner:
		fail("Pending spinner node rotates but paints no pixels above dialog"); return false
	if without_spinner == without_overlay:
		fail("Pending dark overlay paints no pixels above dialog"); return false
	print("TP_PENDING_PAINT spinner_diff=true overlay_diff=true inset=", overlay.get_global_rect())
	return true

func accepted_dialog_waits(label: String, tender: int, purchased: bool) -> bool:
	if not await pause_private_reply(): return false
	await click("PerksProgramConfirmButton")
	if not await wait_until(func(): return client.trading_post_state().pending, "accepted transaction pending", 3000): return false
	if control("PerksProgramConfirmDialog") == null:
		fail("Retail pending " + label + " dialog disappeared before server reply"); return false
	if control_enabled("PerksProgramConfirmButton") or control_enabled("PerksProgramConfirmCancelButton"):
		fail("Pending transaction left a dialog button enabled"); return false
	if not await wait_until(func(): return texture_ready("PerksProgramConfirmSpinnerRing") and texture_ready("PerksProgramConfirmSpinnerSparks"), "real decoded pending spinner", 3000): return false
	if control("PerksProgramConfirmDarkOverlay") == null:
		fail("Pending transaction has no source dark overlay"); return false
	var spinner := control("PerksProgramConfirmSpinner")
	var initial_rotation := spinner.rotation
	await wait_frames(3)
	if is_equal_approx(spinner.rotation, initial_rotation):
		fail("Pending " + label + " spinner is static despite source rotation"); return false
	print("TP_PENDING_SPINNER_ROTATION kind=", label, " before=", initial_rotation, " after=", spinner.rotation)
	if not await assert_pending_painted(): return false
	await button_event(control("PerksProgramConfirmButton").get_global_rect().get_center(), MOUSE_BUTTON_LEFT, true)
	await button_event(control("PerksProgramConfirmButton").get_global_rect().get_center(), MOUSE_BUTTON_LEFT, false)
	await tap(KEY_ESCAPE)
	var state: Dictionary = client.trading_post_state()
	if not state.pending or int(state.tender) != tender or bool(state.purchased) != purchased or control("PerksProgramConfirmDialog") == null:
		fail("Pending input mutated authoritative state or dismissed dialog"); return false
	if label == "purchase" and int(state.selected) in state.cart:
		fail("Accepted single purchase did not remove its product from cart"); return false
	await capture(label + "-pending-before-reply")
	resume_private_reply()
	return true

func pending_dialog_proof() -> void:
	var tender := int(client.trading_post_state().tender)
	if not await choose_category(1): return
	if not await select_first_available(tender): return
	var price := int(client.trading_post_state().price)
	await click("PerksProgramAddToCartButton")
	await click("PerksProgramPurchaseButton")
	if not await accepted_dialog_waits("purchase", tender, false): return
	if not await wait_until(func(): return not client.trading_post_state().pending and client.trading_post_state().purchased and control("PerksProgramConfirmDialog") == null and int(client.trading_post_state().tender) == tender - price, "purchase reply closes pending dialog", 120000): return
	await capture("purchase-reply-completed")
	if int(client.trading_post_state().refund_seconds) <= 0:
		fail("Pending proof requires existing explicit private refund policy"); return
	await click("PerksProgramRefundButton")
	if not await accepted_dialog_waits("refund", tender - price, true): return
	if not await wait_until(func(): return not client.trading_post_state().pending and not client.trading_post_state().purchased and control("PerksProgramConfirmDialog") == null and int(client.trading_post_state().tender) == tender, "refund reply closes pending dialog", 120000): return
	await capture("refund-reply-completed")
	await finish("pending-dialogs")

func preserve_confirmation_on_escape(label: String, item_frame: bool) -> bool:
	if not await wait_until(func(): return control("PerksProgramConfirmDialog") != null, label + " confirmation"): return false
	if item_frame:
		if not await wait_until(func(): return texture_ready("PerksProgramConfirmItem") and texture_ready("PerksProgramConfirmButtonNormalArt") and texture_ready("PerksProgramConfirmBorder"), "decoded confirmation art"): return false
		if not assert_dialog_rects_and_art(): return false
	var before: Dictionary = client.trading_post_state()
	await tap(KEY_ESCAPE)
	if control("PerksProgramConfirmDialog") == null:
		fail("Retail " + label + " confirmation consumed no hideOnEscape but native dismissed it"); return false
	var after: Dictionary = client.trading_post_state()
	if after.pending or before.tender != after.tender or before.cart != after.cart or before.selected != after.selected or before.purchased != after.purchased:
		fail("Escape mutated confirmation authority/cart state"); return false
	await capture(label + "-escape-preserved")
	return true

func cancel_confirmation_and_check(tender: int) -> bool:
	await click("PerksProgramConfirmCancelButton")
	if not await wait_until(func(): return control("PerksProgramConfirmDialog") == null, "explicit Cancel dismisses confirmation"): return false
	if int(client.trading_post_state().tender) != tender or client.trading_post_state().pending:
		fail("Cancel changed Tender or issued a request"); return false
	return true

func confirmation_escape_proof() -> void:
	var tender := int(client.trading_post_state().tender)
	if not await choose_category(1): return
	if not await select_first_available(tender): return
	var selected := int(client.trading_post_state().selected)
	var price := int(client.trading_post_state().price)
	await click("PerksProgramPurchaseButton")
	if not await preserve_confirmation_on_escape("purchase", true): return
	if not await cancel_confirmation_and_check(tender): return
	await click("PerksProgramAddToCartButton")
	await click("PerksProgramViewCartButton")
	await click("PerksProgramCartPurchaseButton")
	if not await preserve_confirmation_on_escape("cart-purchase", false): return
	if not await cancel_confirmation_and_check(tender): return
	await click("PerksProgramCartClearButton")
	if not await preserve_confirmation_on_escape("clear-cart", false): return
	if not await cancel_confirmation_and_check(tender): return
	if not selected in client.trading_post_state().cart:
		fail("Cancel removed the cart item"); return
	await click("PerksProgramCartCloseButton")
	await click("PerksProgramPurchaseButton")
	await click("PerksProgramConfirmButton")
	if not await wait_until(func(): return client.trading_post_state().purchased and not client.trading_post_state().pending and int(client.trading_post_state().tender) == tender - price, "one confirmed authoritative purchase", 120000): return
	if int(client.trading_post_state().refund_seconds) <= 0:
		fail("Confirmation proof needs explicit private refund policy, never production policy change"); return
	await click("PerksProgramRefundButton")
	if not await preserve_confirmation_on_escape("refund", true): return
	if not await cancel_confirmation_and_check(tender - price): return
	if not client.trading_post_state().purchased:
		fail("Refund Cancel revoked purchase"); return
	await click("PerksProgramRefundButton")
	await click("PerksProgramConfirmButton")
	if not await wait_until(func(): return not client.trading_post_state().purchased and not client.trading_post_state().pending and int(client.trading_post_state().tender) == tender, "one confirmed authoritative refund", 120000): return
	await capture("refunded-tender-restored")
	await finish("confirmations")

func expect_cart_item_tooltip(owner_name: String, title: String, item_id: int, label: String, before: Dictionary) -> bool:
	var owner := control(owner_name)
	if owner == null:
		fail("Missing real cart tooltip owner " + owner_name); return false
	print("TP_CART_CURSOR_BEFORE raw=", root.get_mouse_position(), " target=", owner.get_global_rect().get_center())
	await motion(owner.get_global_rect().get_center(), false)
	print("TP_CART_CURSOR_AFTER raw=", root.get_mouse_position(), " hovered=", root.gui_get_hovered_control())
	if not await wait_until(func(): return client.tooltip_state().get("visible", false) and client.tooltip_state().get("title", "") == title, "Retail cart hovered item tooltip", 10000): return false
	var tooltip: Dictionary = client.tooltip_state()
	var lines := str(tooltip.get("lines", []))
	if not lines.contains("Item ID: %d" % item_id) or lines.contains("Sell Price"):
		fail("Cart tooltip lost hovered item ID or included SellPrice: " + str(tooltip)); return false
	var rect := owner.get_global_rect()
	var scale := control("PerksProgramLeaveButton").get_global_rect().size.x / 180.0
	if absf(float(tooltip.rect[0]) + float(tooltip.rect[2]) - (rect.position.x / scale - 8.0)) > 1.5 or absf(float(tooltip.rect[1]) + float(tooltip.rect[3]) - (rect.position.y / scale + 20.0)) > 1.5:
		fail("Cart tooltip lacks source left(-8,-20) owner offset: " + str(tooltip)); return false
	var state: Dictionary = client.trading_post_state()
	if state.selected != before.selected or state.tender != before.tender or state.cart != before.cart or state.cart_preview_items != before.cart_preview_items:
		fail("Cart tooltip hover changed selected preview/cart/Tender"); return false
	await capture(label)
	return true

func cart_item_tooltip_proof() -> void:
	var tender := int(client.trading_post_state().tender)
	if not await choose_category(1): return
	if not await select_first_available(tender): return
	var regular: Dictionary = client.trading_post_state().duplicate(true)
	await click("PerksProgramAddToCartButton")
	if not await choose_category(8): return
	if not await select_first_available(tender): return
	var ensemble: Dictionary = client.trading_post_state().duplicate(true)
	var members: PackedInt64Array = ensemble.preview_item_ids
	if members.size() < 2:
		fail("Cart tooltip proof needs genuine ensemble member identities"); return
	await click("PerksProgramAddToCartButton")
	await click("PerksProgramViewCartButton")
	var regular_name := "PerksProgramCartItem%d" % int(regular.selected)
	if not await wait_until(func(): return control(regular_name) != null and control("PerksProgramCartSet%d" % int(ensemble.selected)) != null, "real regular and ensemble cart rows"): return
	var before: Dictionary = client.trading_post_state().duplicate(true)
	if not await expect_cart_item_tooltip(regular_name, str(regular.selected_name), int(regular.item_id), "regular-item", before): return
	for index in range(2):
		var item_id := int(members[index])
		var owner_name := "PerksProgramCartSubItem%d_%d" % [int(ensemble.selected), item_id]
		if control(owner_name + "Name") == null:
			fail("Missing real ensemble member name " + owner_name); return
		var title: String = control(owner_name + "Name").text
		if not await expect_cart_item_tooltip(owner_name, title, item_id, "ensemble-member-%d" % index, before): return
	await motion(control("PerksProgramCartTitle").get_global_rect().get_center(), false)
	if not await wait_until(func(): return not client.tooltip_state().get("visible", false), "cart row leave clears item tooltip"): return
	if not await expect_cart_item_tooltip(regular_name, str(regular.selected_name), int(regular.item_id), "regular-before-remove", before): return
	await click(regular_name + "Remove")
	if not await wait_until(func(): return control(regular_name) == null and not client.tooltip_state().get("visible", false), "removed cart row clears owned tooltip"): return
	await capture("removed-owner-hidden")
	await click("PerksProgramCartCloseButton")
	if not await wait_until(func(): return control("PerksProgramShoppingCartFrame") == null and not client.tooltip_state().get("visible", false), "cart panel close clears tooltip"): return
	if int(client.trading_post_state().tender) != tender:
		fail("Tooltip-only proof changed authoritative Tender"); return
	await finish("cart-tooltips")

func expect_button_tooltip(owner_name: String, title: String, label: String) -> bool:
	var owner := control(owner_name)
	if owner == null:
		fail("Missing tooltip owner " + owner_name); return false
	await motion(owner.get_global_rect().get_center(), false)
	if not await wait_until(func(): return client.tooltip_state().get("visible", false) and client.tooltip_state().get("title", "") == title, "Retail button tooltip " + title, 10000): return false
	var tooltip: Dictionary = client.tooltip_state()
	var scale := control("PerksProgramLeaveButton").get_global_rect().size.x / 180.0
	var rect := owner.get_global_rect()
	if absf(float(tooltip.rect[0]) + float(tooltip.rect[2]) - rect.position.x / scale) > 1.5 or absf(float(tooltip.rect[1]) + float(tooltip.rect[3]) - rect.position.y / scale) > 1.5:
		fail("Button tooltip lacks source ANCHOR_LEFT zero offset: " + str(tooltip)); return false
	await capture(label)
	return true

func button_tooltip_proof() -> void:
	var tender := int(client.trading_post_state().tender)
	if not client.trading_post_state().cart.is_empty():
		fail("Button tooltip proof requires preserved empty local cart on open"); return
	if control_enabled("PerksProgramViewCartButton"):
		fail("Empty View Cart unexpectedly enabled"); return
	if not await expect_button_tooltip("PerksProgramViewCartButton", "View Cart", "disabled-view-cart"): return
	if not await choose_category(1): return
	if not await select_first_available(tender): return
	var selected := int(client.trading_post_state().selected)
	if not await expect_button_tooltip("PerksProgramPurchaseButton", "Purchase", "purchase"): return
	await motion(control("PerksProgramLeaveButton").get_global_rect().get_center(), false)
	if not await wait_until(func(): return not client.tooltip_state().get("visible", false), "button leave hides tooltip"): return
	await click("PerksProgramAddToCartButton")
	if not await expect_button_tooltip("PerksProgramViewCartButton", "View Cart", "enabled-view-cart"): return
	await click("PerksProgramViewCartButton")
	if not await expect_button_tooltip("PerksProgramCartPurchaseButton", "Purchase Cart", "checkout"): return
	if not await expect_button_tooltip("PerksProgramCartClearButton", "Clear Cart", "clear-cart"): return
	await click("PerksProgramCartClearButton")
	if not await wait_until(func(): return control("PerksProgramConfirmDialog") != null, "clear confirmation blocks tooltip owner"): return
	if not await wait_until(func(): return not client.tooltip_state().get("visible", false), "modal blocks underlying button tooltip"): return
	await click("PerksProgramConfirmButton")
	if not await wait_until(func(): return client.trading_post_state().cart.is_empty(), "confirmed local cart clear"): return
	# Existing clear-cart lifecycle hides the panel; do not force an unreachable
	# empty panel just to manufacture disabled Clear/Checkout native states.
	if not await wait_until(func(): return control("PerksProgramShoppingCartFrame") == null and not client.tooltip_state().get("visible", false), "cart teardown clears tooltip owner"): return
	await capture("clear-panel-hidden")
	if not await expect_button_tooltip("PerksProgramViewCartButton", "View Cart", "cleared-view-cart"): return
	if int(client.trading_post_state().tender) != tender or int(client.trading_post_state().selected) != selected:
		fail("Button tooltip/cart-clear proof changed Tender or selected row"); return
	await finish("button-tooltips")

# Product OnEnter (Elements.lua306-314): hovered item, not selected preview;
# ANCHOR_RIGHT -16, no SellPrice. Native hit testing must own the lifecycle.
func item_tooltip_proof() -> void:
	if not await choose_category(1): return
	var products := []
	# Lower visible rows leave room above ANCHOR_RIGHT without screen clamping.
	var rows: PackedStringArray = client.trading_post_state().rows
	rows.reverse()
	for row in rows:
		var id := int(str(row).split("|")[0])
		var name := "PerksProgramProduct%d" % id
		if control(name) == null: continue
		await click(name)
		if not await wait_until(func(): return int(client.trading_post_state().selected) == id, "tooltip fixture selection"): return
		products.append(client.trading_post_state().duplicate(true))
		if products.size() == 2: break
	if products.size() != 2:
		fail("Tooltip fixture needs two distinct visible transmog rows"); return
	var selected := int(products[1].selected)
	var tender := int(client.trading_post_state().tender)
	for product in products:
		var name := "PerksProgramProduct%d" % int(product.selected)
		await motion(control(name).get_global_rect().get_center(), false)
		await wait_frames(30)
		print("TP_TOOLTIP_PROBE owner=", name, " expected=", product.selected_name, " rect=", control(name).get_global_rect(), " hovered=", root.gui_get_hovered_control(), " tooltip=", client.tooltip_state())
		await capture("hover-probe-%d" % int(product.selected))
		if not await wait_until(func(): return client.tooltip_state().get("visible", false) and client.tooltip_state().get("title", "") == product.selected_name, "Retail hovered product item tooltip", 10000): return
		var tooltip: Dictionary = client.tooltip_state()
		var lines := str(tooltip.get("lines", []))
		if not lines.contains("Item ID: %d" % int(product.item_id)) or not lines.contains("Item Level ") or lines.contains("Sell Price"):
			fail("Product tooltip lost item detail/identity or included SellPrice: " + str(tooltip)); return
		var scale := control("PerksProgramLeaveButton").get_global_rect().size.x / 180.0
		var owner := control(name).get_global_rect()
		if absf(float(tooltip.rect[0]) - (owner.end.x / scale - 16.0)) > 1.5 or absf(float(tooltip.rect[1]) + float(tooltip.rect[3]) - owner.position.y / scale) > 1.5:
			fail("Product tooltip lacks source ANCHOR_RIGHT -16: " + str(tooltip)); return
		if int(client.trading_post_state().selected) != selected or int(client.trading_post_state().tender) != tender:
			fail("Hover changed selection or Tender"); return
		await capture("product-%d" % int(product.selected))
	await motion(control("PerksProgramLeaveButton").get_global_rect().get_center(), false)
	if not await wait_until(func(): return not client.tooltip_state().get("visible", false), "product leave hides tooltip"): return
	await capture("leave-hidden")
	var last_name := "PerksProgramProduct%d" % selected
	await motion(control(last_name).get_global_rect().get_center(), false)
	if not await wait_until(func(): return client.tooltip_state().get("visible", false), "product tooltip re-enter"): return
	await tap(KEY_ESCAPE)
	if not await wait_until(func(): return not client.trading_post_state().open and not client.tooltip_state().get("visible", false), "vendor close hides owned tooltip"): return
	await capture("closed-hidden")
	if not await open_vendor(): return
	await finish("item-tooltips")

func layout_proof() -> void:
	print("TP_LAYOUT_INITIAL footer=", control("PerksProgramFooterFrame").get_global_rect(), " purchase=", control("PerksProgramPurchaseButton").get_global_rect(), " filter=", control("PerksProgramFilter").get_global_rect())
	if control("PerksProgramFilterBackground").mouse_filter != Control.MOUSE_FILTER_IGNORE:
		fail("Decorative WowStyle2 Button intercepted owner input")
		return
	if not assert_vendor_rects(): return
	if not await assert_header_hover(): return
	await capture("vendor-all")
	await click("PerksProgramPriceSortButton")
	if not await wait_until(func(): return client.trading_post_state().ascending, "ascending price header"): return
	await capture("price-ascending")
	await click("PerksProgramNameSortButton")
	if not await wait_until(func(): return client.trading_post_state().sort == "name", "name header"): return
	await click("PerksProgramFilter")
	if not await wait_until(func(): return control("PerksProgramFilterMenu") != null, "filter description"): return
	await capture("filter-description")
	if not await assert_filter_interactions(): return
	if not await choose_category(2): return
	if not await select_first_available(int(client.trading_post_state().tender)): return
	if not assert_masked_icon("PerksProgramIcon%d" % int(client.trading_post_state().selected)): return
	await capture("vendor-mount")
	var first := int(client.trading_post_state().selected)
	if not await drag_to_frozen("PerksProgramProduct%d" % first): return
	if not await wait_until(func(): return int(client.trading_post_state().frozen) == first and not client.trading_post_state().pending, "native freeze", 180000): return
	if not assert_masked_icon("PerksProgramFrozenIcon"): return
	await capture("frozen")
	if not await choose_category(1): return
	if not await select_first_available(int(client.trading_post_state().tender)): return
	var replacement := int(client.trading_post_state().selected)
	if replacement == first: fail("replacement did not change vendor identity"); return
	if not await drag_to_frozen("PerksProgramProduct%d" % replacement): return
	if not await wait_until(func(): return int(client.trading_post_state().frozen) == replacement and not client.trading_post_state().pending, "replace frozen item", 180000): return
	await capture("frozen-replaced")
	if not await drag_off_frozen(): return
	if not await wait_until(func(): return int(client.trading_post_state().frozen) < 0 and not client.trading_post_state().pending, "native drag-off unfreeze", 180000): return
	await capture("unfrozen")
	var tender := int(client.trading_post_state().tender)
	var price := int(client.trading_post_state().price)
	var footer := control("PerksProgramFooterFrame")
	print("TP_LAYOUT_FOOTER ", footer.get_global_rect() if footer != null else "no projected control", " button=", control("PerksProgramPurchaseButton").get_global_rect(), " model=", control("PerksProgramMainModelScene").get_global_rect(), " enabled=", control_enabled("PerksProgramPurchaseButton"))
	await click("PerksProgramPurchaseButton")
	if not await wait_until(func(): return control("PerksProgramConfirmDialog") != null, "source popup"): return
	if not await wait_until(func(): return texture_ready("PerksProgramConfirmItem") and texture_ready("PerksProgramConfirmButtonNormalArt") and texture_ready("PerksProgramConfirmBorder"), "decoded StaticPopup source art"): return
	if not assert_dialog_rects_and_art(): return
	await capture("purchase-confirm")
	await tap(KEY_ESCAPE)
	if control("PerksProgramConfirmDialog") == null or not client.trading_post_state().open:
		fail("Source confirmation cover failed to consume Escape"); return
	await click("PerksProgramConfirmCancelButton")
	if not await wait_until(func(): return control("PerksProgramConfirmDialog") == null, "Cancel dismisses purchase popup"): return
	if int(client.trading_post_state().tender) != tender: fail("cancel changed Tender"); return
	await click("PerksProgramPurchaseButton")
	await click("PerksProgramConfirmButton")
	if not await wait_until(func(): return client.trading_post_state().purchased and int(client.trading_post_state().tender) == tender-price, "authoritative purchase"): return
	await capture("purchased")
	await click("PerksProgramRefundButton")
	if not await wait_until(func(): return control("PerksProgramConfirmDialog") != null, "refund popup"): return
	await capture("refund-confirm")
	await click("PerksProgramConfirmButton")
	if not await wait_until(func(): return not client.trading_post_state().purchased and int(client.trading_post_state().tender) == tender, "authoritative refund"): return
	await capture("refunded")
	finish("layout-proof")

func assert_filter_interactions() -> bool:
	await click("PerksProgramFilterCollected")
	if not await wait_until(func(): return not client.trading_post_state().filter_collected, "Collected checkbox"): return false
	await click("PerksProgramFilterUncollected")
	if not await wait_until(func(): return not client.trading_post_state().filter_uncollected and client.trading_post_state().rows.is_empty(), "both collection flags exclude all rows"): return false
	await capture("filter-empty")
	await click("PerksProgramFilterResetButton")
	if not await wait_until(func(): return client.trading_post_state().filter_collected and client.trading_post_state().filter_uncollected and not client.trading_post_state().rows.is_empty(), "Reset restores source filter flags"): return false
	if control("PerksProgramFilterMenu") == null: await click("PerksProgramFilter")
	await click("PerksProgramFilterUsable")
	if not await wait_until(func(): return client.trading_post_state().filter_usable, "Useable Only checkbox"): return false
	await motion(control("PerksProgramFilterSort").get_global_rect().get_center(), false)
	if not await wait_until(func(): return control("PerksProgramSortMenu") != null, "Sort By hover submenu"): return false
	if control_enabled("PerksProgramFilterSortTime"):
		fail("Unsourced per-item remaining time became enabled"); return false
	await capture("filter-sort")
	await click("PerksProgramFilterSortPrice")
	if not await wait_until(func(): return client.trading_post_state().sort == "price" and not client.trading_post_state().ascending, "Price radio"): return false
	await click("PerksProgramFilterAscending")
	if not await wait_until(func(): return client.trading_post_state().ascending, "Ascending checkbox"): return false
	await click("PerksProgramFilterResetButton")
	if not await wait_until(func(): return not client.trading_post_state().filter_usable and client.trading_post_state().sort == "price" and client.trading_post_state().ascending, "Reset preserves independent sort state"): return false
	if control("PerksProgramFilterMenu") == null: await click("PerksProgramFilter")
	await motion(control("PerksProgramFilterSort").get_global_rect().get_center(), false)
	if not await wait_until(func(): return control("PerksProgramSortMenu") != null, "Sort By after Reset"): return false
	await click("PerksProgramFilterSortName")
	if not await wait_until(func(): return client.trading_post_state().sort == "name", "Name radio"): return false
	await click("PerksProgramFilter")
	return true

func drag_off_frozen() -> bool:
	var slot := control("PerksProgramFrozenSlot")
	var model := control("PerksProgramMainModelScene")
	if slot == null or model == null: fail("Missing frozen slot/model canvas"); return false
	var from := slot.get_global_rect().get_center()
	var to := model.get_global_rect().get_center()
	await motion(from, false)
	await button_event(from, MOUSE_BUTTON_LEFT, true)
	for step in range(1, 11):
		await motion(from.lerp(to, step / 10.0), true, (to-from)/10.0)
	await button_event(to, MOUSE_BUTTON_LEFT, false)
	return true

func browse() -> void:
	var state: Dictionary = client.trading_post_state()
	if int(state.items) == 0: fail("Vendor has no sourced items"); return
	if not await wait_until(func(): return client.trading_post_state().preview_ready, "default selection preview", 120000): return
	await capture("vendor-all")
	await click("PerksProgramFilter")
	if not await wait_until(func(): return control("PerksProgramFilterMenu") != null, "filter description"): return
	await capture("filter-description")
	for category in [[1, "transmog", "player"], [8, "transmog-set", "player"], [3, "pet", "creature"], [2, "mount", "creature"]]:
		if not await choose_category(category[0]): return
		var first: PackedStringArray = str(client.trading_post_state().rows[0]).split("|")
		if first[1] == "0":
			# Unreadable source rows keep their gap text and must not show a stale model.
			await click("PerksProgramProduct" + first[0])
			if not await wait_until(func(): return str(client.trading_post_state().selected) == first[0] and client.trading_post_state().preview_kind == "none", "unavailable row clears preview"): return
			await capture("vendor-" + category[1] + "-unavailable-row")
		if not await select_first_available(): return
		var kind: String = category[2]
		if not await wait_until(func(): return client.trading_post_state().preview_ready and client.trading_post_state().preview_kind == kind, category[1] + " " + kind + " preview", 120000): return
		await capture("vendor-" + category[1])
	if not await rotate_model(): return
	if not await choose_category(5): return
	if control("PerksProgramToyIcon") == null or control("PerksProgramToyDescription") == null:
		fail("ToyOverlay icon/details missing"); return
	await capture("vendor-toy")
	if not await choose_category(2): return
	var frozen := int(client.trading_post_state().selected)
	if not await drag_to_frozen("PerksProgramProduct%d" % frozen): return
	if not await wait_until(func(): return int(client.trading_post_state().frozen) == frozen and not client.trading_post_state().pending, "authoritative freeze"): return
	await capture("frozen")
	await purchase_affordable()

func purchase_affordable() -> void:
	var tender := int(client.trading_post_state().tender)
	print("TP_TENDER ", tender, " selected=", client.trading_post_state())
	if tender < int(client.trading_post_state().price):
		await click("PerksProgramPurchaseButton")
		if control("PerksProgramConfirmDialog") != null: fail("Unaffordable purchase opened confirmation"); return
		await capture("purchase-unaffordable")
		finish("browse-unaffordable")
		return
	await click("PerksProgramPurchaseButton")
	if not await wait_until(func(): return control("PerksProgramConfirmDialog") != null, "purchase confirmation"): return
	if int(client.trading_post_state().tender) != tender: fail("Confirmation debited locally"); return
	await capture("purchase-confirm")
	await click("PerksProgramConfirmButton")
	if not await wait_until(func(): return client.trading_post_state().purchased and int(client.trading_post_state().tender) == tender - int(client.trading_post_state().price), "authoritative purchase and debit"): return
	await capture("purchased")
	finish("browse-purchased")

func relog_checks() -> void:
	var state: Dictionary = client.trading_post_state()
	if int(state.frozen) < 0: fail("Frozen slot did not survive relog"); return
	await capture("relog-vendor")
	finish("relog")

# Vendor category, label and the client collection receipt each purchase must change.
const PURCHASES := [[1, "transmog", "appearance"], [2, "mount", "mounts"], [3, "pet", "pets"], [5, "toy", "toys"]]

func cart_ensemble_preview() -> bool:
	if not await choose_category(8): return false
	if not await select_first_available(): return false
	var state: Dictionary = client.trading_post_state()
	var vendor := int(state.selected)
	var members: PackedInt64Array = state.preview_item_ids
	if members.size() < 2: fail("Authored ensemble needs multiple member rows"); return false
	await click("PerksProgramAddToCartButton")
	if not await wait_until(func(): return client.trading_post_state().cart.size() == 1, "ensemble add"): return false
	await click("PerksProgramViewCartButton")
	if not await wait_until(func(): return control("PerksProgramCartSet%d" % vendor) != null and client.trading_post_state().preview_ready, "source ensemble header and composite preview"): return false
	await capture("cart-ensemble-preview")
	var item := int(members[0])
	await click("PerksProgramCartSubItem%d_%d" % [vendor,item])
	if not await wait_until(func(): return not item in client.trading_post_state().cart_preview_items and client.trading_post_state().preview_ready, "ensemble member deselection"): return false
	await capture("cart-ensemble-member-deselected")
	await click("PerksProgramCartSubItem%d_%d" % [vendor,item])
	if not await wait_until(func(): return item in client.trading_post_state().cart_preview_items, "ensemble member restored"): return false
	await click("PerksProgramCartClearButton")
	if not await wait_until(func(): return control("PerksProgramConfirmButton") != null, "ensemble clear confirmation"): return false
	await click("PerksProgramConfirmButton")
	if not await wait_until(func(): return client.trading_post_state().cart.size() == 0 and not client.trading_post_state().cart_visible, "confirmed clear"): return false
	return true

func checkout_cart() -> void:
	var tender := int(client.trading_post_state().tender)
	if tender < 2000: fail("Admin cart Tender setup missing"); return
	if not await cart_ensemble_preview(): return
	var bought := []
	var total := 0
	for entry in PURCHASES:
		if not await choose_category(entry[0]): return
		if not await select_first_available(tender - total): return
		var state: Dictionary = client.trading_post_state()
		if state.purchased or state.collected: fail("Cart fixture needs unowned authored rows: " + str(state)); return
		if entry[0] == 1:
			if not await wait_until(func(): return client.trading_post_state().preview_ready, "dressed preview"): return
			await capture("cart-outfit-default")
			var default_hide: bool = state.hide_armor
			await click("PerksProgramToggleHideArmor")
			if not await wait_until(func(): return client.trading_post_state().hide_armor != default_hide and client.trading_post_state().preview_ready, "Hide Armor override"): return
			await capture("cart-outfit-hidden")
			await click("PerksProgramToggleHideArmor")
			if not await wait_until(func(): return client.trading_post_state().hide_armor == default_hide, "restore authored outfit"): return
		var before := await collection_count(entry[2], int(state.item_id))
		await click("PerksProgramAddToCartButton")
		total += int(state.price)
		bought.append({"category":entry[0],"kind":entry[2],"vendor":int(state.selected),"item_id":int(state.item_id),"price":int(state.price),"count":before + 1})
		if not await wait_until(func(): return client.trading_post_state().cart.size() == bought.size(), "cart add"): return
	await click("PerksProgramViewCartButton")
	if not await wait_until(func(): return client.trading_post_state().cart_visible and control("PerksProgramCartPurchaseButton") != null and client.trading_post_state().preview_ready and client.trading_post_state().preview_kind == "player", "cart composite preview"): return
	if int(client.trading_post_state().cart_price) != total: fail("Cart total differs from authored prices"); return
	await capture("cart-four-products")
	var wearable = bought[0]
	var mount = bought[1]
	await click("PerksProgramCartItem%d" % int(mount.vendor))
	if not await wait_until(func(): return client.trading_post_state().preview_ready and client.trading_post_state().preview_kind == "creature", "OTHER cart preview"): return
	var dimmed := control("PerksProgramCartItem%d" % int(wearable.vendor))
	if absf(dimmed.modulate.a - 0.7) > 0.01: fail("OTHER preview did not dim wearable to source alpha0.7"); return
	await capture("cart-other-preview")
	await click("PerksProgramCartItem%d" % int(wearable.vendor))
	if not await wait_until(func(): return client.trading_post_state().preview_ready and client.trading_post_state().preview_kind == "player" and int(wearable.item_id) in client.trading_post_state().cart_preview_items, "disabled wearable clears OTHER without deselection"): return
	await capture("cart-other-cleared")
	await click("PerksProgramCartClearButton")
	if not await wait_until(func(): return control("PerksProgramConfirmDialog") != null, "clear confirmation"): return
	await capture("cart-clear-confirm")
	await click("PerksProgramConfirmCancelButton")
	if not await wait_until(func(): return control("PerksProgramConfirmDialog") == null, "cancel clear"): return
	if client.trading_post_state().cart.size() != 4: fail("Cancel clear removed products"); return
	await click("PerksProgramCartPurchaseButton")
	if not await wait_until(func(): return control("PerksProgramConfirmDialog") != null, "cart checkout confirmation"): return
	if control("PerksProgramConfirmItemFrame") != null: fail("Cart incorrectly has single-item frame"); return
	if int(client.trading_post_state().tender) != tender: fail("Cart confirmation debited locally"); return
	await capture("cart-checkout-confirm")
	await click("PerksProgramConfirmButton")
	if not await wait_until(func(): return not client.trading_post_state().pending and int(client.trading_post_state().tender) == tender-total, "atomic checkout reply"): return
	if client.trading_post_state().cart.size() != 0: fail("Accepted cart not cleared"); return
	for row in bought:
		if not await choose_category(int(row.category)): return
		await click("PerksProgramProduct%d" % int(row.vendor))
		if not await wait_until(func(): return int(client.trading_post_state().selected) == int(row.vendor) and client.trading_post_state().purchased and client.trading_post_state().collected, "checkout affected row"): return
		if not await wait_until_count(row.kind,int(row.item_id),int(row.count),"checkout collection"): return
		if control("PerksProgramHistoryRefundTime") == null: fail("Selected purchased item missing source refund history"); return
		await capture("cart-owned-" + str(row.kind))
	var file := FileAccess.open(receipts.path_join(skin + "-purchases.json"),FileAccess.WRITE)
	file.store_string(JSON.stringify({"tender":tender-total,"bought":bought},"\t"))
	file.close()
	finish("cart-checkout")

func purchase_each_collection() -> void:
	var tender := int(client.trading_post_state().tender)
	if tender < 2000: fail("Admin Tender setup missing: %d" % tender); return
	var bought := []
	for entry in PURCHASES:
		if not await choose_category(entry[0]): return
		if not await select_first_available(tender): return
		var state: Dictionary = client.trading_post_state()
		if state.purchased or state.collected: fail("Fresh account already owns " + str(state)); return
		var before := await collection_count(entry[2], int(state.item_id))
		await click("PerksProgramPurchaseButton")
		if not await wait_until(func(): return control("PerksProgramConfirmDialog") != null, entry[1] + " purchase confirmation"): return
		if int(client.trading_post_state().tender) != tender: fail("Confirmation debited locally"); return
		await capture("confirm-" + entry[1])
		await click("PerksProgramConfirmButton")
		var price := int(state.price)
		if not await wait_until(func(): return client.trading_post_state().purchased and client.trading_post_state().collected and int(client.trading_post_state().tender) == tender - price, entry[1] + " authoritative purchase"): return
		if not await wait_until_count(entry[2], int(state.item_id), before + 1, entry[1] + " client collection receipt"): return
		if int(client.trading_post_state().refund_seconds) <= 0 or not control_enabled("PerksProgramRefundButton"): fail("Admin refund window not offered"); return
		tender -= price
		await capture("purchased-" + entry[1])
		bought.append({"category": entry[0], "kind": entry[2], "vendor": int(state.selected), "item_id": int(state.item_id), "price": price, "count": before + 1})
	var file := FileAccess.open(receipts.path_join(skin + "-purchases.json"), FileAccess.WRITE)
	file.store_string(JSON.stringify({"tender": tender, "bought": bought}, "\t"))
	file.close()
	finish("purchased")

func refund_after_relog() -> void:
	var saved: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(receipts.path_join(skin + "-purchases.json")))
	var tender := int(client.trading_post_state().tender)
	if tender != int(saved.tender): fail("Tender changed across relog: %d != %d" % [tender, int(saved.tender)]); return
	for row in saved.bought:
		if not await choose_category(int(row.category)): return
		await click("PerksProgramProduct%d" % int(row.vendor))
		if not await wait_until(func(): return int(client.trading_post_state().selected) == int(row.vendor), "select purchased row"): return
		var state: Dictionary = client.trading_post_state()
		if not state.purchased or not state.collected: fail("Purchase/collection lost across relog: " + str(state)); return
		if not await wait_until_count(row.kind, int(row.item_id), int(row.count), str(row.kind) + " collection after relog"): return
		await capture("relog-owned-" + str(row.kind))
		await click("PerksProgramRefundButton")
		if not await wait_until(func(): return control("PerksProgramConfirmDialog") != null, "refund confirmation"): return
		await capture("refund-confirm-" + str(row.kind))
		await click("PerksProgramConfirmButton")
		tender += int(row.price)
		if not await wait_until(func(): return not client.trading_post_state().purchased and not client.trading_post_state().collected and int(client.trading_post_state().tender) == tender, str(row.kind) + " authoritative refund"): return
		if not await wait_until_count(row.kind, int(row.item_id), int(row.count) - 1, str(row.kind) + " collection revoked"): return
		await capture("refunded-" + str(row.kind))
	finish("refunded")

func collection_count(kind: String, item_id: int) -> int:
	match kind:
		"mounts": return client.mount_journal_state().collected.size()
		"toys":
			for toy in client.toybox_state().learned:
				if int(toy.item_id) == item_id: return 1
			return 0
		"pets":
			var text := await ipc(["collection", "pets"])
			return 0 if text.strip_edges().is_empty() else text.strip_edges().split("\n").size()
	# Appearances: the server computes `collected` from the account appearance store.
	return 1 if client.trading_post_state().collected else 0

func wait_until_count(kind: String, item_id: int, expected: int, label: String) -> bool:
	var deadline := Time.get_ticks_msec() + 30000
	while Time.get_ticks_msec() < deadline:
		if await collection_count(kind, item_id) == expected: return true
		await wait_frames(30)
	fail("timeout: %s (expected %d, have %d)" % [label, expected, await collection_count(kind, item_id)])
	return false

func control_enabled(name: String) -> bool:
	var node := control(name)
	return node != null and not (node is BaseButton and node.disabled)

func travelers_log() -> bool:
	await click("MinimapLauncherButton")
	if not await wait_until(func(): return control("LauncherEntryEJMicroButton") != null, "Adventure Guide launcher entry"): return false
	await click("LauncherEntryEJMicroButton")
	if not await wait_until(func(): return control("EncounterJournalMonthlyActivitiesTab") != null, "Traveler's Log tab"): return false
	await click("EncounterJournalMonthlyActivitiesTab")
	if not await wait_until(func(): return control("JournalMonthlyPoints") != null and "Tender earned" in control("JournalMonthlyPoints").text, "authoritative Log Tender line", 60000): return false
	await capture("travelers-log")
	await tap(KEY_ESCAPE)
	return true

func open_vendor() -> bool:
	# Setup sends the existing NPC interaction request; vendor actions remain native input.
	var reply := await ipc(["quest", "interact", "--npc", "Tawny Seabraid"])
	if reply.is_empty(): return false
	if not await wait_until(func(): return client.trading_post_state().open or control("GossipOption0Text") != null, "Tawny greeting or vendor"): return false
	if not client.trading_post_state().open:
		await click("GossipOption0Text")
	return await wait_until(func(): return client.trading_post_state().open and int(client.trading_post_state().get("items", 0)) > 0, "authoritative Trading Post snapshot", 180000)

func choose_category(id: int) -> bool:
	if control("PerksProgramFilterMenu") == null:
		await click("PerksProgramFilter")
	await motion(control("PerksProgramFilterTypes").get_global_rect().get_center(), false)
	if not await wait_until(func(): return control("PerksProgramTypeMenu") != null, "source Type submenu on hover"): return false
	for row in client.trading_post_state().types:
		var parts: PackedStringArray = str(row).split("|")
		var checked := parts[1] == "1"
		if checked != (int(parts[0]) == id):
			await click("PerksProgramFilterType" + parts[0])
	await capture("filter-type-%d" % id)
	await click("PerksProgramFilter")
	return await wait_until(func(): return int(client.trading_post_state().category) == id and control("PerksProgramFilterMenu") == null, "only source type %d checked" % id)

# First listed row with a bound grant (and, given `budget`, a price within it).
func select_first_available(budget: int = -1) -> bool:
	for row in client.trading_post_state().rows:
		var parts: PackedStringArray = row.split("|")
		if parts[1] != "1": continue
		await click("PerksProgramProduct" + parts[0])
		if not await wait_until(func(): return str(client.trading_post_state().selected) == parts[0], "select available row " + parts[0]): return false
		var state: Dictionary = client.trading_post_state()
		if budget < 0: return true
		if int(state.price) <= budget and not state.purchased and not state.collected: return true
	fail("No available row within %d Tender in category %d" % [budget, int(client.trading_post_state().category)])
	return false

func select_other_cartable(excluded: Array) -> int:
	for row in client.trading_post_state().rows:
		var parts: PackedStringArray = str(row).split("|")
		var vendor := int(parts[0])
		if parts[1] != "1" or vendor in excluded: continue
		await click("PerksProgramProduct%d" % vendor)
		if not await wait_until(func(): return int(client.trading_post_state().selected) == vendor, "cartable selection"): return -1
		if control("PerksProgramAddToCartButton") != null and control_enabled("PerksProgramAddToCartButton"): return vendor
	fail("No second cartable product in current visible source rows")
	return -1

func assert_freeze_waiting(original: int, candidate: int, other: int) -> bool:
	var state: Dictionary = client.trading_post_state()
	if int(state.frozen) != original or state.pending:
		fail("Frozen replacement sent a request before consent"); return false
	if state.cart.size() != 2 or not candidate in state.cart or not other in state.cart:
		fail("Unconfirmed frozen replacement pruned local cart"); return false
	return true

func open_freeze_replacement(original: int, candidate: int, other: int) -> bool:
	if not await drag_to_frozen("PerksProgramProduct%d" % candidate): return false
	if not await wait_until(func(): return control("PerksProgramConfirmDialog") != null or int(client.trading_post_state().frozen) == candidate, "replacement confirmation or premature request"): return false
	if control("PerksProgramConfirmDialog") == null:
		fail("Frozen replacement sent a request before showing Retail confirmation"); return false
	return assert_freeze_waiting(original, candidate, other)

func frozen_slot_geometry_proof(check_geometry: bool) -> void:
	var tender := int(client.trading_post_state().tender)
	if int(client.trading_post_state().frozen) >= 0:
		if not await drag_off_frozen(): return
		if not await wait_until(func(): return int(client.trading_post_state().frozen) < 0 and not client.trading_post_state().pending, "empty reservation for geometry capture"): return
	if not await choose_category(2): return
	if not await select_first_available(): return
	if not await wait_until(func(): return client.trading_post_state().rider_ready, "mount ready for frozen geometry capture", 120000): return
	var original := int(client.trading_post_state().selected)
	await capture("empty-slot")
	if not await drag_to_frozen("PerksProgramProduct%d" % original): return
	if not await wait_until(func(): return int(client.trading_post_state().frozen) == original and not client.trading_post_state().pending, "settled frozen icon"): return
	if check_geometry:
		for name in ["PerksProgramFrozenIcon", "PerksProgramFrozenArtContainer", "PerksProgramFrozenFrame"]:
			if not assert_native_rect(name, Rect2(78,801.5,64,64)): return
		if not assert_native_rect("PerksProgramFrozenLabel", Rect2(150,811,233,45)): return
	else:
		# Preserve a genuinely old-layout before capture, not a hot-reloaded fix.
		if not assert_native_rect("PerksProgramFrozenIcon", Rect2(78,817.5,32,32)): return
		if not assert_native_rect("PerksProgramFrozenFrame", Rect2(62,801.5,64,64)): return
	await capture("frozen-slot")
	if check_geometry:
		for frame_name in ["PerksProgramFrozenIcon", "PerksProgramFrozenSlotArt", "PerksProgramFrozenFrame"]:
			if not assert_texture_pixels(frame_name): return
	var candidate := await select_other_cartable([original])
	if candidate < 0: return
	if not await drag_to_frozen("PerksProgramProduct%d" % candidate): return
	if not await wait_until(func(): return control("PerksProgramConfirmDialog") != null, "pending freeze glow"): return
	if check_geometry:
		if not assert_native_rect("PerksProgramFrozenItemGlow", Rect2(66,789.5,88,88)): return
		var frost := control("PerksProgramFrozenFrame")
		if frost != null and frost.is_visible_in_tree() and frost.modulate.a > 0.01: fail("Pending frozen icon incorrectly kept visible settled frost frame"); return
	await capture("pending-slot")
	if check_geometry and not assert_texture_pixels("PerksProgramFrozenItemGlow"): return
	await click("PerksProgramConfirmCancelButton")
	if not await wait_until(func(): return control("PerksProgramConfirmDialog") == null, "cancel pending geometry capture"): return
	await capture("cancel-restores-frozen-slot")
	if not await drag_off_frozen(): return
	if not await wait_until(func(): return int(client.trading_post_state().frozen) < 0 and not client.trading_post_state().pending, "geometry fixture clears reservation"): return
	if int(client.trading_post_state().tender) != tender: fail("Frozen geometry capture changed Tender"); return
	await finish(phase)

func watch_menu_sound_players() -> bool:
	var kits := client.find_child("UiSoundKits", true, false)
	if kits == null: fail("Native UI sound player is unavailable; not a menu sound RED"); return false
	kits.child_entered_tree.connect(observe_menu_sound_player)
	return true

func observe_menu_sound_player(node: Node) -> void:
	var player := node as AudioStreamPlayer
	if player == null: return
	var stream: AudioStream = player.stream
	if stream == null: return
	var fdid := int(stream.resource_name)
	var kind := "open" if fdid in MENU_OPEN_FILES else "close" if fdid in MENU_CLOSE_FILES else ""
	if kind.is_empty(): return
	var event := {"kind":kind,"fdid":fdid,"length":stream.get_length(),"gain":player.volume_linear,"pitch_scale":player.pitch_scale,"playing_observed":false,"finished":false}
	menu_audio_events.append(event)
	player.finished.connect(func(): event.finished = true)
	call_deferred("observe_menu_sound_playback", player, event)

func observe_menu_sound_playback(player: AudioStreamPlayer, event: Dictionary) -> void:
	if is_instance_valid(player):
		event.playing_observed = player.playing
		event.playback_position = player.get_playback_position()

func menu_sound_count(kit: int) -> int:
	var count := 0
	for played in client.merchant_state().ui_sounds:
		if int(played) == kit: count += 1
	return count

func assert_menu_audio_count(kind: String, expected: int) -> bool:
	var settings_gain := -1.0
	for event in menu_audio_events:
		if event.kind == "open":
			settings_gain = float(event.gain) / 0.562341332
			break
	if settings_gain < 0.0: fail("No native menu open gain observation"); return false
	var source_gain := 0.562341332 if kind == "open" else 0.01
	var count := 0
	for event in menu_audio_events:
		if event.kind != kind: continue
		count += 1
		if absf(float(event.gain) - source_gain*settings_gain) > 0.00001: fail("Menu kit ignored source gain/current audio settings: %s" % event); return false
		if float(event.length) <= 0.0 or not (event.playing_observed or event.finished): fail("Menu kit has no actual decoded native playback: %s" % event); return false
	if count != expected: fail("Decoded %s playback count%d, expected%d" % [kind,count,expected]); return false
	return true

func assert_menu_sound_state(kind: String, expected: int) -> bool:
	var kit := 218932 if kind == "open" else 218933
	if menu_sound_count(kit) != expected:
		fail("Menu %s kit%d played%d times, expected%d" % [kind,kit,menu_sound_count(kit),expected])
		return false
	return assert_menu_audio_count(kind, expected)

func menu_sound_proof() -> void:
	if menu_sound_count(218932) != 1: fail("Retail menu open kit218932 did not play once on actual show"); return
	if menu_sound_count(218933) != 0: fail("Unprojected/initially closed vendor emitted a close sound"); return
	await wait_frames(30)
	if not assert_menu_sound_state("open", 1): return
	await capture("opened-once")
	await click("PerksProgramLeaveButton")
	if not await wait_until(func(): return not client.trading_post_state().open, "Leave closes vendor"): return
	await wait_frames(30)
	if not assert_menu_sound_state("close", 1): return
	await capture("closed-once")
	if not await open_vendor(): return
	await wait_frames(30)
	if not assert_menu_sound_state("open", 2): return
	await capture("reopened-once")
	await tap(KEY_ESCAPE)
	if not await wait_until(func(): return not client.trading_post_state().open, "Escape closes vendor"): return
	await wait_frames(30)
	if not assert_menu_sound_state("close", 2): return
	var chat: CanvasLayer = client.get_node_or_null("ChatFrameUI")
	if int(client.trading_post_state().hidden_layers) != 0 or chat == null or not chat.visible: fail("Sound fixture close did not restore HUD"); return
	await capture("escape-closed-once")
	var file := FileAccess.open(shots.path_join("menu-audio-events.json"), FileAccess.WRITE)
	if file == null: fail("Cannot write menu audio observations"); return
	file.store_string(JSON.stringify(menu_audio_events, "\t"))
	file.close()
	print("TRADING_POST_LIVE PASS skin=", skin, " phase=menu-sounds audio=", menu_audio_events)
	client.free()
	quit(0)

func authored_camera_proof() -> void:
	if not await choose_category(2): return
	if not await select_first_available(): return
	if int(client.trading_post_state().selected) != 223: fail("Camera fixture requires source Celestial Steed223/scene633"); return
	if not await wait_until(func(): return client.trading_post_state().rider_ready, "authored camera mount ready", 120000): return
	var rider: Node3D = instance_from_id(int(client.trading_post_state().rider_actor_id)) as Node3D
	var group: Node = rider.get_parent()
	while group != null and group.name != "TradingPostMountPreview": group = group.get_parent()
	if group == null: fail("Camera fixture mount group missing"); return
	var camera: Camera3D = group.get_viewport().get_camera_3d()
	var authored_yaw := deg_to_rad(137.5)
	var expected_eye := Vector3(-cos(authored_yaw) * 23.0, 0, sin(authored_yaw) * 23.0)
	if not camera.position.is_equal_approx(expected_eye): fail("Trading Post camera did not use authored633/619 base target0/yaw137.5deg/zoom23: %s" % camera.position); return
	var initial_basis: Basis = group.basis
	var initial_actor: Transform3D = group.get_node("MountActor").transform
	var initial_camera: Transform3D = camera.transform
	await capture("authored-base-mounted")
	await click("PerksProgramTogglePlayerPreview")
	if not await wait_until(func(): return not client.trading_post_state().rider_ready, "camera retains mount while hiding rider"): return
	if not camera.transform.is_equal_approx(initial_camera): fail("Hide rider changed authored camera"); return
	await capture("authored-base-mount-only")
	for direction in ["Left", "Right"]:
		var point: Vector2 = control("PerksProgramRotate" + direction + "Button").get_global_rect().get_center()
		var before := camera.position
		await motion(point, false)
		await button_event(point, MOUSE_BUTTON_LEFT, true)
		await wait_frames(10)
		if camera.position.distance_to(before) < 1.0: fail("Held rotation did not orbit actual camera"); return
		if not is_equal_approx(camera.position.length(), 23.0): fail("Orbit refitted camera distance"); return
		if group.basis != initial_basis or group.get_node("MountActor").transform != initial_actor: fail("Orbit rotated actor instead of camera"); return
		await capture("authored-orbit-held-" + direction.to_lower())
		await motion(Vector2(1800,500), true, Vector2(1,0))
		await button_event(Vector2(1800,500), MOUSE_BUTTON_LEFT, false)
		await wait_frames(3)
		var released := camera.transform
		await wait_frames(10)
		if not camera.transform.is_equal_approx(released): fail("Released orbit continued moving"); return
		await capture("authored-orbit-released-" + direction.to_lower())
	var drag_start := Vector2(900,500)
	var yaw_before_drag := float(client.trading_post_state().camera_yaw)
	await motion(drag_start, false)
	await button_event(drag_start, MOUSE_BUTTON_LEFT, true)
	await motion(drag_start + Vector2(60,0), true, Vector2(60,0))
	await button_event(drag_start + Vector2(60,0), MOUSE_BUTTON_LEFT, false)
	var ui_scale := control("PerksProgramMainModelScene").get_global_rect().size.x / 1920.0
	var expected_drag := yaw_before_drag - 60.0 / ui_scale * 0.008
	if absf(float(client.trading_post_state().camera_yaw) - expected_drag) > 0.0001: fail("Drag did not orbit with source effective-scale delta"); return
	if group.basis != initial_basis or group.get_node("MountActor").transform != initial_actor: fail("Drag rotated authored actors"); return
	await capture("authored-drag-camera-only")
	await button_event(drag_start, MOUSE_BUTTON_WHEEL_UP, true)
	await button_event(drag_start, MOUSE_BUTTON_WHEEL_UP, false)
	if not is_equal_approx(camera.position.length(), 13.1): fail("Wheel did not change source zoom percent by0.1"); return
	await capture("authored-wheel-zoom")
	await button_event(drag_start, MOUSE_BUTTON_WHEEL_DOWN, true)
	await button_event(drag_start, MOUSE_BUTTON_WHEEL_DOWN, false)
	if not is_equal_approx(camera.position.length(), 23.0): fail("Wheel did not restore authored base zoom"); return
	await click("PerksProgramTogglePlayerPreview")
	if not await wait_until(func(): return client.trading_post_state().rider_ready, "restored rider under authored camera", 120000): return
	if group.basis != initial_basis or group.get_node("MountActor").transform != initial_actor: fail("Rider restore changed authored actor"); return
	await capture("authored-orbit-rider-restored")
	if not await choose_category(3): return
	if not await select_first_available(): return
	if is_instance_valid(group): fail("Camera category change retained old mount scene"); return
	await capture("pet-base-camera")
	await finish("authored-camera")

func freeze_workflow_proof() -> void:
	var tender := int(client.trading_post_state().tender)
	if int(client.trading_post_state().frozen) >= 0:
		if not await drag_off_frozen(): return
		if not await wait_until(func(): return int(client.trading_post_state().frozen) < 0 and not client.trading_post_state().pending, "owned fixture clears old reservation"): return
	if not await choose_category(1): return
	var original := await select_other_cartable([])
	if original < 0: return
	var original_name := str(client.trading_post_state().selected_name)
	if not await drag_to_frozen("PerksProgramProduct%d" % original): return
	if not await wait_until(func(): return int(client.trading_post_state().frozen) == original and not client.trading_post_state().pending, "empty slot freezes immediately"): return
	if control("PerksProgramConfirmDialog") != null: fail("Empty frozen slot incorrectly asked for replacement consent"); return
	await capture("initial-frozen-item")
	if not await choose_category(2): return
	var candidate := await select_other_cartable([])
	if candidate < 0: return
	var candidate_name := str(client.trading_post_state().selected_name)
	await click("PerksProgramAddToCartButton")
	var other := await select_other_cartable([candidate])
	if other < 0: return
	await click("PerksProgramAddToCartButton")
	if not await wait_until(func(): return client.trading_post_state().cart.size() == 2, "two cart members"): return
	if not await open_freeze_replacement(original, candidate, other): return
	if str(control("PerksProgramConfirmItemText").text) != original_name: fail("Replacement popup did not identify OLD frozen item"); return
	if str(control("PerksProgramFrozenLabel").text) != candidate_name: fail("Pending frozen slot did not preview candidate"); return
	if control_enabled("PerksProgramConfirmButton") or not control_enabled("PerksProgramConfirmCancelButton"): fail("Accept delay did not disable only Confirm"); return
	await click("PerksProgramConfirmButton")
	if not assert_freeze_waiting(original, candidate, other): return
	await capture("replace-delay-old-item")
	await click("PerksProgramConfirmCancelButton")
	if not await wait_until(func(): return control("PerksProgramConfirmDialog") == null, "Cancel dismisses frozen replacement"): return
	if int(client.trading_post_state().frozen) != original or client.trading_post_state().cart.size() != 2: fail("Cancel changed frozen authority/cart"); return
	if str(control("PerksProgramFrozenLabel").text) != original_name: fail("Cancel did not restore old frozen slot"); return
	await capture("cancel-restores-old")
	if not await open_freeze_replacement(original, candidate, other): return
	await tap(KEY_ESCAPE)
	if control("PerksProgramConfirmDialog") == null: fail("Frozen popup incorrectly inherited purchase hideOnEscape"); return
	if not assert_freeze_waiting(original, candidate, other): return
	await capture("escape-keeps-confirmation")
	await button_event(Vector2(1800,500), MOUSE_BUTTON_RIGHT, true)
	await button_event(Vector2(1800,500), MOUSE_BUTTON_RIGHT, false)
	if not await wait_until(func(): return control("PerksProgramConfirmDialog") == null, "global right-click cancels replacement"): return
	if int(client.trading_post_state().frozen) != original or client.trading_post_state().cart.size() != 2: fail("Right-click cancel changed frozen authority/cart"); return
	await capture("right-click-cancel")
	if not await open_freeze_replacement(original, candidate, other): return
	if not await wait_until(func(): return control_enabled("PerksProgramConfirmButton"), "source five-second accept delay expires", 15000): return
	if str(control("PerksProgramConfirmButtonDialogButtonText").text) != "Confirm": fail("Expired accept delay did not restore source Confirm text"); return
	if not assert_freeze_waiting(original, candidate, other): return
	await capture("replacement-ready")
	await click("PerksProgramConfirmButton")
	if not await wait_until(func(): return int(client.trading_post_state().frozen) == candidate and not client.trading_post_state().pending, "accepted replacement authority"): return
	if client.trading_post_state().cart.size() != 1 or int(client.trading_post_state().cart[0]) != other: fail("Confirmed freeze did not remove only its cart member"); return
	await capture("replacement-confirmed-cart-pruned")
	if not await drag_off_frozen(): return
	if not await wait_until(func(): return int(client.trading_post_state().frozen) < 0 and not client.trading_post_state().pending, "clear accepted reservation for empty-slot boundary"): return
	if not await drag_to_frozen("PerksProgramProduct%d" % other): return
	if not await wait_until(func(): return int(client.trading_post_state().frozen) == other and not client.trading_post_state().pending, "cart member frozen into empty slot"): return
	if control("PerksProgramConfirmDialog") != null or client.trading_post_state().cart.size() != 0: fail("Empty freeze did not immediately prune cart without confirmation"); return
	if int(client.trading_post_state().tender) != tender: fail("Freeze workflow mutated Tender"); return
	await capture("empty-slot-cart-pruned")
	if not await drag_off_frozen(): return
	if not await wait_until(func(): return int(client.trading_post_state().frozen) < 0 and not client.trading_post_state().pending, "fixture leaves reservation empty"): return
	await finish("freeze-workflow")

func mount_rider_cancel_proof() -> void:
	if not await choose_category(2): return
	if not await select_first_available(): return
	if not await wait_until(func(): return client.trading_post_state().rider_ready, "initial mounted rider", 120000): return
	await click("PerksProgramTogglePlayerPreview")
	if not await wait_until(func(): return not client.trading_post_state().rider_ready and not client.trading_post_state().rider_pending, "hidden rider before pending boundary"): return
	queue_click("PerksProgramTogglePlayerPreview")
	if not await wait_until(func(): return client.trading_post_state().rider_pending, "actual asynchronous rider request", 30000): return
	print("TP_PENDING_RIDER_BOUNDARY ", client.trading_post_state())
	queue_click("PerksProgramLeaveButton")
	if not await wait_until(func(): return not client.trading_post_state().open, "close while rider pending"): return
	await wait_frames(60)
	var state: Dictionary = client.trading_post_state()
	if state.rider_pending or state.rider_ready or state.preview_ready or int(state.rider_actor_id) != 0: fail("Close retained pending or loaded rider"); return
	if client.find_children("TradingPostMountPreview", "Node3D", true, false).size() != 0: fail("Late rider arrival recreated closed preview"); return
	var chat: CanvasLayer = client.get_node_or_null("ChatFrameUI")
	if chat == null or not chat.visible or int(state.hidden_layers) != 0: fail("Pending-rider close did not restore HUD"); return
	await capture("closed-pending-rider-no-late-arrival")
	print("TRADING_POST_LIVE PASS skin=", skin, " phase=mount-rider-cancel state=", state)
	client.free()
	quit(0)

func queue_click(name: String) -> void:
	var node := control(name)
	if node == null: fail("Missing pending-boundary control " + name); return
	var point := node.get_global_rect().get_center()
	var motion_event := InputEventMouseMotion.new()
	motion_event.position = point
	motion_event.global_position = point
	root.push_input(motion_event, true)
	for down in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.button_mask = MOUSE_BUTTON_MASK_LEFT if down else 0
		event.pressed = down
		root.push_input(event, true)

func mount_rider_proof() -> void:
	if not await choose_category(2): return
	if not await select_first_available(): return
	if not await wait_until(func(): return client.trading_post_state().rider_ready, "dressed mounted rider", 120000): return
	if not assert_native_rect("PerksProgramTogglePlayerPreview", Rect2(1032,951,30,29)): return
	var rider: Node3D = instance_from_id(int(client.trading_post_state().rider_actor_id)) as Node3D
	if not assert_seated_rider(rider): return
	var mount_group: Node = rider.get_parent()
	while mount_group != null and mount_group.name != "TradingPostMountPreview":
		mount_group = mount_group.get_parent()
	if mount_group == null: fail("Rider is not owned by mounted preview"); return
	var mount_camera: Camera3D = mount_group.get_viewport().get_camera_3d()
	var camera_transform: Transform3D = mount_camera.transform
	await capture("mounted-dressed-default")
	await click("PerksProgramTogglePlayerPreview")
	if not await wait_until(func(): return client.trading_post_state().hide_player_preview and not client.trading_post_state().rider_ready and not client.trading_post_state().rider_pending, "Hide Character frees rider"): return
	if is_instance_valid(rider) or not is_instance_valid(mount_group): fail("Hide Character freed mount or retained rider"); return
	if not mount_camera.transform.is_equal_approx(camera_transform): fail("Hide Character refitted mount camera"); return
	await capture("mount-only-hidden")
	await click("PerksProgramTogglePlayerPreview")
	if not await wait_until(func(): return client.trading_post_state().rider_ready, "restored current player", 120000): return
	rider = instance_from_id(int(client.trading_post_state().rider_actor_id))
	if not assert_seated_rider(rider): return
	if not mount_camera.transform.is_equal_approx(camera_transform): fail("Show Character refitted mount camera"); return
	await capture("mounted-restored")
	var actor_transform: Transform3D = mount_group.get_node("MountActor").transform
	if not await rotate_model(): return
	if mount_group.get_node("MountActor").transform != actor_transform: fail("Yaw overwrote authored actor transform"); return
	await click("PerksProgramTogglePlayerPreview")
	if not await choose_category(3): return
	if not await select_first_available(): return
	if is_instance_valid(mount_group) or is_instance_valid(rider): fail("Category change retained mounted actors"); return
	if control("PerksProgramTogglePlayerPreview") != null: fail("Pet exposed mount-only checkbox"); return
	await capture("pet-no-rider-control")
	if not await choose_category(2): return
	if not await select_first_available(): return
	if not await wait_until(func(): return client.trading_post_state().preview_ready, "reselected hidden mount", 120000): return
	if not client.trading_post_state().hide_player_preview or client.trading_post_state().rider_ready or client.trading_post_state().rider_pending: fail("Selection reset Hide Character"); return
	await capture("reselected-retains-hidden")
	await click("PerksProgramTogglePlayerPreview")
	if not await wait_until(func(): return client.trading_post_state().rider_ready, "rider before close", 120000): return
	rider = instance_from_id(int(client.trading_post_state().rider_actor_id))
	await click("PerksProgramLeaveButton")
	if not await wait_until(func(): return not client.trading_post_state().open, "close mounted preview"): return
	await wait_frames(60)
	if is_instance_valid(rider) or client.find_children("TradingPostMountPreview", "Node3D", true, false).size() != 0: fail("Close leaked a mounted preview or late rider"); return
	await capture("closed-no-late-rider")
	if not await open_vendor(): return
	if not await choose_category(2): return
	if not await select_first_available(): return
	if not await wait_until(func(): return client.trading_post_state().rider_ready, "new preview after reopen", 120000): return
	if not assert_seated_rider(instance_from_id(int(client.trading_post_state().rider_actor_id))): return
	await capture("reopened-mounted")
	await finish("mount-rider")

func assert_seated_rider(actor: Node) -> bool:
	if actor == null or actor.get_child_count() != 1: fail("Rider actor missing current player"); return false
	var animation := actor.get_child(0).get_node_or_null("M2Animation")
	if animation == null or int(animation.current_animation_id()) != 91: fail("Rider did not use seated animation91"); return false
	if actor.get_parent().name != "Attachment0": fail("Rider did not attach to saddle0"); return false
	return true

func rotate_control_proof() -> void:
	if not await choose_category(2): return
	if not await select_first_available(): return
	if not await wait_until(func(): return client.trading_post_state().preview_ready, "mount preview ready"): return
	await capture("rotate-default")
	for direction in ["Left", "Right"]:
		var name: String = "PerksProgramRotate" + direction + "Button"
		var target := control(name)
		if target == null: fail("Missing Retail rotation button " + name); return
		var point: Vector2 = target.get_global_rect().get_center()
		var before := float(client.trading_post_state().camera_yaw)
		await motion(point, false)
		await button_event(point, MOUSE_BUTTON_LEFT, true)
		await wait_frames(10)
		var held := float(client.trading_post_state().camera_yaw)
		if not assert_rotate_icon(name, true): return
		var sign := 1.0 if direction == "Left" else -1.0
		if sign * (held - before) < 0.5: fail("Held %s did not continuously rotate sourced direction: %s -> %s" % [direction, before, held]); return
		await capture("rotate-held-" + direction.to_lower())
		await motion(Vector2(1800, 500), true, Vector2(1, 0))
		await button_event(Vector2(1800, 500), MOUSE_BUTTON_LEFT, false)
		await wait_frames(3)
		var released := float(client.trading_post_state().camera_yaw)
		if not assert_rotate_icon(name, false): return
		await wait_frames(10)
		if absf(float(client.trading_post_state().camera_yaw) - released) > 0.0001: fail("Release outside did not stop rotation"); return
		await capture("rotate-released-" + direction.to_lower())
	if not await choose_category(5): return
	if not await select_first_available(): return
	if control("PerksProgramRotateLeftButton") != null: fail("Toy retained model-scene rotation controls"); return
	await capture("toy-no-rotate-controls")
	await finish("rotate-controls")

func assert_rotate_icon(name: String, pressed: bool) -> bool:
	var button := control(name)
	var icon := control(name + "Icon")
	if button == null or icon == null: fail("Missing sourced rotate icon"); return false
	var rect: Rect2 = button.get_global_rect()
	var offset := 1.0 if pressed else 0.0
	var expected := rect.position + Vector2(11.0 + offset, 11.5 + offset) * rect.size.x / 48.0
	if icon.get_global_rect().position.distance_to(expected) > 0.51:
		fail("Rotation pressed icon offset differs from Retail: %s != %s" % [icon.get_global_rect().position, expected]); return false
	return true

func rotate_model() -> bool:
	var start := control("PerksProgramMainModelScene").get_global_rect().get_center()
	var before := await pixels("PerksProgramMainModelScene")
	await motion(start, false)
	await button_event(start, MOUSE_BUTTON_LEFT, true)
	for step in range(6):
		await motion(start + Vector2(20 * (step + 1), 0), true, Vector2(20, 0))
	await button_event(start + Vector2(120, 0), MOUSE_BUTTON_LEFT, false)
	await wait_frames(20)
	var after := await pixels("PerksProgramMainModelScene")
	if before == after: fail("Model drag did not change the rendered preview"); return false
	await capture("rotated")
	return true

func drag_to_frozen(name: String) -> bool:
	var source := control(name)
	var target := control("PerksProgramFrozenSlot")
	if source == null or target == null: fail("Missing drag source or frozen slot"); return false
	var from := source.get_global_rect().get_center()
	var to := target.get_global_rect().get_center()
	await motion(from, false)
	await button_event(from, MOUSE_BUTTON_LEFT, true)
	for step in range(1, 11):
		await motion(from.lerp(to, step / 10.0), true, (to - from) / 10.0)
	await button_event(to, MOUSE_BUTTON_LEFT, false)
	return true

func assert_native_rect(name: String, expected: Rect2) -> bool:
	var node := control(name)
	if node == null: fail("Missing projected control " + name); return false
	var actual := node.get_global_rect()
	for delta in [actual.position.x-expected.position.x, actual.position.y-expected.position.y, actual.size.x-expected.size.x, actual.size.y-expected.size.y]:
		if absf(delta) > 0.51:
			fail("Source projected rect %s: %s != %s" % [name,actual,expected]); return false
	return true

func assert_vendor_rects() -> bool:
	for row in [
		["PerksProgramProductsScrollBoxContainer", Rect2(46,142.5,527,751)],
		["PerksProgramScrollBox", Rect2(46,200.5,503,573)],
		["PerksProgramHoldFrame", Rect2(46,773.5,527,120)],
		["PerksProgramFrozenSlot", Rect2(46,791.5,527,84)],
		["PerksProgramFilter", Rect2(55,95.5,145,25)],
		["PerksProgramFooterFrame", Rect2(0,980,1920,100)],
		["PerksProgramLeaveButton", Rect2(46,1014,180,46)],
		["PerksProgramPurchaseButton", Rect2(1694,1014,180,46)],
		["PerksProgramLeftGradient", Rect2(0,0,950,1080)],
		["PerksProgramRightGradient", Rect2(1320,0,600,1080)],
	]:
		if not assert_native_rect(row[0],row[1]): return false
	var price := control("PerksProgramPriceSortButton").get_global_rect()
	var time := control("PerksProgramTimeSortButton").get_global_rect()
	if absf(price.get_center().y-time.get_center().y) > 0.51 or absf(price.end.x+48-time.position.x) > 0.51:
		fail("Price RIGHT-to-Time.LEFT centering diverged"); return false
	return true

func assert_header_hover() -> bool:
	await motion(control("PerksProgramMainModelScene").get_global_rect().get_center(),false)
	await wait_frames(5)
	if not assert_header_color(Color(1,0.82,0,1), "normal"): return false
	var before := await pixels("PerksProgramNameSortLabel")
	await motion(control("PerksProgramNameSortButton").get_global_rect().get_center(),false)
	await wait_frames(5)
	if not assert_header_color(Color.WHITE, "hover"): return false
	if before == await pixels("PerksProgramNameSortLabel"):
		fail("Header hover did not change rendered pixels"); return false
	await capture("headers-hover")
	await motion(control("PerksProgramMainModelScene").get_global_rect().get_center(),false)
	await wait_frames(5)
	if not assert_header_color(Color(1,0.82,0,1), "leave"): return false
	return true

func assert_header_color(expected: Color, boundary: String) -> bool:
	# Async snapshots can rebuild the projection while the fixture yields.
	var label := control("PerksProgramNameSortLabel")
	if label == null:
		fail("Missing projected header after " + boundary); return false
	if not label.get_theme_color("font_color").is_equal_approx(expected):
		fail("Header color diverged at " + boundary); return false
	return true

func texture_ready(name: String) -> bool:
	var node := control(name)
	if node == null: return false
	var part := node.get_node_or_null("Parts/Part0") as TextureRect
	return part != null and part.texture != null

func assert_texture_pixels(name: String) -> bool:
	var node := control(name)
	var part := node.get_node_or_null("Parts/Part0") as TextureRect if node != null else null
	if part == null or part.texture == null:
		fail("Source art is absent on " + name); return false
	var image := part.texture.get_image()
	if image == null or image.is_empty(): fail("Source image is empty on " + name); return false
	var covered := 0
	for y in range(image.get_height()):
		for x in range(image.get_width()):
			if image.get_pixel(x,y).a > 0.1: covered += 1
	if covered == 0: fail("Source image has no visible pixels on " + name); return false
	return true

func assert_masked_icon(name: String) -> bool:
	if not assert_texture_pixels(name): return false
	var image: Image = control(name).get_node("Parts/Part0").texture.get_image()
	if image.get_pixel(0,0).a > 0.01 or image.get_pixel(floori(image.get_width()*0.5),floori(image.get_height()*0.5)).a < 0.75:
		fail("Authored mask did not cut corners/preserve icon on " + name); return false
	# Exact48:32 sampling is gated by native RGBA comparisons against each
	# decoded source icon; mount icons can have authored transparent borders.
	return true

func assert_dialog_rects_and_art() -> bool:
	var item := control("PerksProgramConfirmItem").get_global_rect()
	var text := control("PerksProgramConfirmItemText").get_global_rect()
	var button := control("PerksProgramConfirmButton").get_global_rect()
	if item.size != Vector2(37,37) or text.size != Vector2(103,38) or button.size != Vector2(128,21):
		fail("StaticPopup ItemFrame/button dimensions differ from source"); return false
	if absf(item.end.x+10-text.position.x)>0.51:
		fail("StaticPopup item is not RIGHT-to-text.LEFT-10"); return false
	if not assert_native_rect("PerksProgramConfirmCover", Rect2(0,0,1920,1080)): return false
	for name in ["PerksProgramConfirmItem", "PerksProgramConfirmItemNameFrame", "PerksProgramConfirmButtonNormalArt", "PerksProgramConfirmBorder"]:
		if not assert_texture_pixels(name): return false
	return true

func pixels(name: String) -> PackedByteArray:
	await RenderingServer.frame_post_draw
	var rect := Rect2i(control(name).get_global_rect())
	return root.get_texture().get_image().get_region(rect).get_data()

func ipc(arguments: Array) -> String:
	var worker := Thread.new()
	var argv := ["--signal=KILL", "15", OS.get_environment("GAME_ENGINE_CLI"), "--socket", "/tmp/game-engine-%d.sock" % OS.get_process_id()] + arguments
	var error := worker.start(func():
		var output := []
		var code := OS.execute("/usr/bin/timeout", argv, output, true)
		return {"code": code, "text": "\n".join(output)})
	if error != OK: fail("Cannot start IPC setup worker"); return ""
	while worker.is_alive(): await process_frame
	var result: Dictionary = worker.wait_to_finish()
	print("TP_SETUP_IPC ", arguments, " code=", result.code, " text=", result.text)
	if result.code != 0: fail("Bounded NPC setup failed"); return ""
	return result.text

func finish(label: String) -> void:
	if int(client.trading_post_state().hidden_layers) == 0: fail("HUD stayed visible under the full-screen vendor"); return
	var chat: CanvasLayer = client.get_node_or_null("ChatFrameUI")
	if chat == null or chat.visible: fail("Chat stayed visible under the full-screen vendor"); return
	await click("PerksProgramLeaveButton")
	if not await wait_until(func(): return not client.trading_post_state().open and control("MinimapLauncherButton") != null and chat.visible, "Leave restores the HUD and chat"): return
	print("TRADING_POST_LIVE PASS skin=", skin, " phase=", label, " state=", client.trading_post_state())
	client.free()
	quit(0)

func control(name: String) -> Control:
	for node in client.find_children(name, "Control", true, false):
		if node.is_visible_in_tree(): return node
	return null

func wait_until(predicate: Callable, label: String, timeout: int = 30000) -> bool:
	var deadline := Time.get_ticks_msec() + timeout
	while Time.get_ticks_msec() < deadline:
		if predicate.call(): return true
		await process_frame
	fail("timeout: " + label)
	return false

func click(name: String, button: int = MOUSE_BUTTON_LEFT) -> void:
	var node := control(name)
	if node == null: fail("Missing visible control " + name); return
	var point := node.get_global_rect().get_center()
	await motion(point, false)
	await button_event(point, button, true)
	await button_event(point, button, false)
	await wait_frames(5)

func motion(point: Vector2, down: bool, relative: Vector2 = Vector2.ZERO) -> void:
	var event := InputEventMouseMotion.new()
	event.position = point
	event.global_position = point
	event.relative = relative
	event.button_mask = MOUSE_BUTTON_MASK_LEFT if down else 0
	if phase in ["cart-tooltips", "row-cart-toggle", "refund-icon-tooltip"]:
		Input.parse_input_event(event)
	else:
		root.push_input(event, true)
	await process_frame

func button_event(point: Vector2, button: int, down: bool) -> void:
	var event := InputEventMouseButton.new()
	event.position = point
	event.global_position = point
	event.button_index = button
	event.button_mask = MOUSE_BUTTON_MASK_LEFT if down and button == MOUSE_BUTTON_LEFT else 0
	event.pressed = down
	root.push_input(event, true)
	await process_frame

func tap(code: Key) -> void:
	for down in [true, false]:
		var event := InputEventKey.new()
		event.keycode = code
		event.physical_keycode = code
		event.pressed = down
		root.push_input(event, true)
		await process_frame
	await wait_frames(5)

func capture(label: String) -> void:
	await wait_frames(30)
	await RenderingServer.frame_post_draw
	var stem := skin + "-" + phase + "-" + label
	var image_path := shots.path_join(stem + ".png")
	if root.get_texture().get_image().save_png(image_path) != OK:
		fail("Cannot save Trading Post evidence PNG: " + image_path)
		return
	var file := FileAccess.open(receipts.path_join(stem + ".json"), FileAccess.WRITE)
	if file == null:
		fail("Cannot save Trading Post evidence receipt: " + stem)
		return
	file.store_string(JSON.stringify({"trading_post": client.trading_post_state(), "tooltip": client.tooltip_state(), "journal": client.adventure_guide_state(), "account": client.account_state()}, "\t"))
	file.close()
	print("TP_CAPTURE ", stem)

func wait_frames(count: int) -> void:
	for frame in range(count): await process_frame

func fail(message: String) -> void:
	resume_private_reply()
	if client != null and not receipts.is_empty():
		var file := FileAccess.open(receipts.path_join(skin + "-" + phase + "-failure.json"), FileAccess.WRITE)
		file.store_string(JSON.stringify({"failure": message, "tooltip": client.tooltip_state(), "trading_post": client.trading_post_state(), "journal": client.adventure_guide_state(), "account": client.account_state()}, "\t"))
		file.close()
	push_error("Trading Post live: " + message)
	quit(1)
