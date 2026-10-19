extends "res://tests/trading_post_cart_selection.gd"
# Explicit TP_DRAG_OPT_IN=1; seven checked native-input cases, no vendor transaction.
# Reuse only the landed strict account/input/capture helpers, not its cart cases.
var drag_observations: Array = []
var source_a := 0
var source_b := 0
# Historical169 conserved44268; the selected-texture slice now requires0 before accept.
const HELD_OBSERVATIONS := ["below-existing-threshold", "first-held-activation", "repeated-held-motion", "already-selected-held-silent", "cart-owner-held-selection", "occupied-drop-before-release"]

func run_cases() -> bool:
	if not gate.require(OS.get_environment("TP_DRAG_OPT_IN") == "1","explicit drag proof opt-in required"): return false
	if not gate.step(await connect_private(),"private account/world/vendor"): return false
	baseline = client.trading_post_state().duplicate(true)
	appearances = Array(client.tooltip_state().appearance_collection)
	appearances.sort()
	origin = client.account_state().local_player_position
	if not gate.require(baseline.cart.is_empty() and int(baseline.frozen) > 0,"empty cart/genuine occupied reservation required"): return false
	if not await only_type(1): return false
	var available: Array = []
	for row in client.trading_post_state().rows:
		var parts := str(row).split("|")
		if parts[1] == "1": available.append(int(parts[0]))
	if not gate.require(available.size() >= 2,"two observed eligible ordinary products required"): return false
	source_a = int(available[0])
	source_b = int(available[1])
	if not gate.step(await held_repeat_outside_cases(),"held/repeat/outside input collection"): return false
	if not gate.step(await already_selected_case(),"already-selected collection"): return false
	if not gate.step(await cart_activation_case(),"cart-owner collection"): return false
	if not gate.step(await modal_guard_case(),"modal collection"): return false
	if not gate.step(await occupied_drop_case(),"occupied-slot cancellation collection"): return false
	# Restore authority/local state before judging timing observations, including RED.
	if not gate.step(await restore_drag_state(),"restore before verdict"): return false
	if not write_receipt("result",{"skin":skin,"baseline":baseline,"appearances":appearances,"observations":drag_observations,"audio":audio,"final":client.trading_post_state(),"cases":7}): return false
	for case_id in range(1,8):
		var found := false
		for observation in drag_observations:
			if int(observation.case_id) != case_id: continue
			found = true
			if not assert_drag_observation(observation): return false
		if not gate.require(found,"missing independent drag case%d" % case_id): return false
		if not gate.case_done(): return false
	return true

func held_repeat_outside_cases() -> bool:
	if not await click_checked("PerksProgramProduct%d" % source_a): return false
	var count := sound_count()
	var start := row_center(source_b)
	if not gate.require(start != Vector2.INF,"source B row exists"): return false
	if not await press_product(start): return false
	# Existing native threshold is distance_squared>16. Keep it, do not tune it.
	await drag_motion(start+Vector2(2,0),Vector2(2,0))
	if not await observe_drag(1,"below-existing-threshold",source_a,count,false,false): return false
	await drag_motion(start+Vector2(12,0),Vector2(10,0))
	if not await observe_drag(1,"first-held-activation",source_b,count+1,false,false): return false
	await drag_motion(start+Vector2(18,0),Vector2(6,0))
	await drag_motion(start+Vector2(24,0),Vector2(6,0))
	if not await observe_drag(2,"repeated-held-motion",source_b,count+1,false,false): return false
	var outside := Vector2(1850,80)
	await drag_motion(outside,outside-start)
	await release_product(outside)
	return await observe_drag(3,"outside-release-retains-source",source_b,count+1,false,false)

func already_selected_case() -> bool:
	if not await click_checked("PerksProgramProduct%d" % source_b): return false
	var count := sound_count()
	var start := row_center(source_b)
	if not gate.require(start != Vector2.INF,"already-selected row exists"): return false
	if not await press_product(start): return false
	await drag_motion(start+Vector2(12,0),Vector2(12,0))
	if not await observe_drag(4,"already-selected-held-silent",source_b,count,false,false): return false
	var outside := Vector2(1850,80)
	await drag_motion(outside,outside-start)
	await release_product(outside)
	return await observe_drag(4,"already-selected-outside-silent",source_b,count,false,false)

func cart_activation_case() -> bool:
	if not await click_checked("PerksProgramProduct%d" % source_a): return false
	var add := control("PerksProgramAddToCartButton")
	if not gate.require(add != null and not bool(add.get("disabled")),"source A local-cart eligible"): return false
	if not await click_checked("PerksProgramAddToCartButton"): return false
	if not await click_checked("PerksProgramViewCartButton"): return false
	if not gate.require(client.trading_post_state().cart_visible and int(client.trading_post_state().selected) == -1,"landed cart owner setup"): return false
	var cart_before: Dictionary = client.trading_post_state().duplicate(true)
	var count := sound_count()
	var start := row_center(source_b)
	if not gate.require(start != Vector2.INF,"cart-visible ordinary source row exists"): return false
	if not await press_product(start): return false
	await drag_motion(start+Vector2(12,0),Vector2(12,0))
	if not await observe_drag(5,"cart-owner-held-selection",source_b,count+1,false,false): return false
	var outside := Vector2(1850,80)
	await drag_motion(outside,outside-start)
	await release_product(outside)
	if not await observe_drag(5,"cart-owner-outside-release",source_b,count+1,false,false): return false
	return gate.require(client.trading_post_state().cart == cart_before.cart and client.trading_post_state().cart_preview_items == cart_before.cart_preview_items,"drag selection preserves independent cart preview choices")

func modal_guard_case() -> bool:
	# Explicit setup click closes either the old or corrected cart owner before reopening.
	if not await click_checked("PerksProgramProduct%d" % source_a): return false
	if not await click_checked("PerksProgramViewCartButton"): return false
	if not await click_checked("PerksProgramCartClearButton"): return false
	if not await until(func(): return control("PerksProgramConfirmDialog") != null,"local Clear dialog"): return false
	var count := sound_count()
	var start := row_center(source_b)
	if not gate.require(start != Vector2.INF,"covered source row exists"): return false
	if not await press_product(start): return false
	await drag_motion(start+Vector2(12,0),Vector2(12,0))
	await release_product(start+Vector2(12,0))
	if not await observe_drag(6,"modal-retains-cart-owner",-1,count,true,true): return false
	if not await click_checked("PerksProgramConfirmCancelButton"): return false
	if not await click_checked("PerksProgramProduct%d" % source_a): return false
	if not await click_checked("PerksProgramRemoveFromCartButton"): return false
	return gate.require(client.trading_post_state().cart.is_empty(),"local cart cleanup without Clear acceptance")

func occupied_drop_case() -> bool:
	if not await click_checked("PerksProgramProduct%d" % source_a): return false
	var count := sound_count()
	var start := row_center(source_b)
	var slot := control("PerksProgramFrozenSlot")
	if not gate.require(start != Vector2.INF and slot != null,"source and genuine occupied freeze target"): return false
	if not await press_product(start): return false
	await drag_motion(start+Vector2(12,0),Vector2(12,0))
	if not await observe_drag(7,"occupied-drop-before-release",source_b,count+1,false,false): return false
	var target := slot.get_global_rect().get_center()
	await drag_motion(target,target-start)
	await release_product(target)
	if not await until(func(): return control("PerksProgramConfirmDialog") != null,"unaccepted freeze replacement dialog"): return false
	if not await observe_drag(7,"occupied-drop-no-duplicate",source_b,count+1,false,true): return false
	if not await click_checked("PerksProgramConfirmCancelButton"): return false
	if not gate.require(not client.trading_post_state().pending and int(client.trading_post_state().frozen) == int(baseline.frozen),"cancel never changes authority"): return false
	if not await click_checked("PerksProgramFrozenSlot"): return false
	return await observe_drag(7,"committed-frozen-after-cancel",int(baseline.frozen),count+2,false,false)

func restore_drag_state() -> bool:
	if not await only_type(0): return false
	if not await click_checked("PerksProgramProduct%d" % int(baseline.selected)): return false
	if not await until(func(): return int(client.trading_post_state().selected) == int(baseline.selected),"original selection"): return false
	var now: Dictionary = client.trading_post_state()
	var final_appearances := Array(client.tooltip_state().appearance_collection)
	final_appearances.sort()
	if not gate.require(int(now.tender) == int(baseline.tender) and int(now.frozen) == int(baseline.frozen),"Tender/reservation conserved"): return false
	if not gate.require(final_appearances == appearances and now.cart.is_empty() and int(now.items) == int(baseline.items),"exact collection/cart/catalog conserved"): return false
	if not await checkpoint("restored-before-verdict",sound_count()): return false
	var chat := client.get_node_or_null("ChatFrameUI") as CanvasLayer
	if not gate.require(chat != null and not chat.visible,"fullscreen HUD ownership"): return false
	if not await click_checked("PerksProgramLeaveButton"): return false
	if not await until(func(): return not client.trading_post_state().open and chat.visible,"Leave restores HUD"): return false
	if not gate.require(audio.size() == sound_count(),"one decoded play per selection callback"): return false
	for event in audio:
		if not gate.require(float(event.length) > 0. and (event.playing or event.finished),"decoded playback observed"): return false
	return true

func observe_drag(case_id: int, label: String, selected: int, count: int, cart_visible: bool, dialog: bool) -> bool:
	await frames(12)
	var position: Vector3 = client.account_state().local_player_position
	var data := {"case_id":case_id,"label":label,"expected_selected":selected,"expected_sound_count":count,"expected_cart_visible":cart_visible,"expected_dialog":dialog,"state":client.trading_post_state().duplicate(true),"sound_count":sound_count(),"source_a":source_a,"source_b":source_b,"art_a":selected_area("PerksProgramSelected%d" % source_a),"art_b":selected_area("PerksProgramSelected%d" % source_b),"frozen_art":selected_area("PerksProgramFrozenSelected"),"dialog":control("PerksProgramConfirmDialog") != null,"world_delta":position.distance_to(origin)}
	drag_observations.append(data)
	return await capture_drag(label,data)

func assert_drag_observation(data: Dictionary) -> bool:
	var message := "%s expected selected%d/sounds%d, actual%d/%d" % [data.label,data.expected_selected,data.expected_sound_count,data.state.selected,data.sound_count]
	if not gate.require(int(data.state.selected) == int(data.expected_selected),message): return false
	if not gate.require(int(data.sound_count) == int(data.expected_sound_count),message): return false
	if not gate.require(bool(data.state.cart_visible) == bool(data.expected_cart_visible),"cart owner at "+str(data.label)): return false
	if not gate.require(bool(data.dialog) == bool(data.expected_dialog),"modal owner at "+str(data.label)): return false
	if not gate.require(not data.state.pending and int(data.state.frozen) == int(baseline.frozen),"no authority request at "+str(data.label)): return false
	if not gate.require(int(data.state.tender) == int(baseline.tender) and float(data.world_delta) < 0.0001,"authority/world conserved at "+str(data.label)): return false
	if int(data.expected_selected) == source_b:
		if not gate.require(float(data.art_b) > 0. and float(data.art_a) == 0.,"source B selected art at "+str(data.label)): return false
	elif int(data.expected_selected) == source_a:
		if not gate.require(float(data.art_a) > 0. and float(data.art_b) == 0.,"source A art below threshold"): return false
	else:
		if not gate.require(float(data.art_a) == 0. and float(data.art_b) == 0.,"ordinary product art remains clear"): return false
	if str(data.label) in HELD_OBSERVATIONS:
		if not gate.require(float(data.frozen_art) == 0.,"frozen art clear before release at "+str(data.label)): return false
	elif str(data.label) == "occupied-drop-no-duplicate":
		if not gate.require(float(data.frozen_art) == 0.,"unaccepted pending preview is not a frozen selection"): return false
	elif str(data.label) == "committed-frozen-after-cancel":
		if not gate.require(float(data.frozen_art) > 0.,"explicit committed selection remains visible after cancel"): return false
	return true

func row_center(id: int) -> Vector2:
	var row := control("PerksProgramProduct%d" % id)
	return row.get_global_rect().get_center() if row != null else Vector2.INF

func press_product(point: Vector2) -> bool:
	await motion(point)
	await drag_button(point,true)
	return not gate.failed

func release_product(point: Vector2) -> void:
	await drag_button(point,false)
	await frames(5)

func drag_button(point: Vector2, down: bool) -> void:
	var event := InputEventMouseButton.new()
	event.position = point
	event.global_position = point
	event.button_index = MOUSE_BUTTON_LEFT
	event.button_mask = MOUSE_BUTTON_MASK_LEFT if down else 0
	event.pressed = down
	root.push_input(event,true)
	await process_frame

func drag_motion(point: Vector2, relative: Vector2) -> void:
	var event := InputEventMouseMotion.new()
	event.position = point
	event.global_position = point
	event.relative = relative
	event.button_mask = MOUSE_BUTTON_MASK_LEFT
	Input.parse_input_event(event)
	await process_frame

func checkpoint(label: String, expected: int) -> bool:
	await frames(30)
	return await capture_drag(label,{"state":client.trading_post_state(),"sound_count":sound_count(),"expected_sound_count":expected})

func capture_drag(label: String, value: Dictionary) -> bool:
	await RenderingServer.frame_post_draw
	var path := shots.path_join(skin+"-drag-selection-"+label+".png")
	if not gate.require(root.get_texture().get_image().save_png(path) == OK,"save drag PNG"): return false
	return write_receipt(label,value)

func write_receipt(label: String, value: Dictionary) -> bool:
	var file := FileAccess.open(receipts.path_join(skin+"-drag-selection-"+label+".json"),FileAccess.WRITE)
	if not gate.require(file != null,"write drag receipt "+label): return false
	file.store_string(JSON.stringify(value,"\t"))
	file.close()
	return true
