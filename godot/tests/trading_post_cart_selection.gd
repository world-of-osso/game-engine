extends SceneTree
# Native cart-owner proof only. No purchase/refund/freeze acceptance or policy setup.
const Completion = preload("res://tests/completion.gd")
const EXPECTED_CASES := 7
var gate: RefCounted
var client: Node
var skin := OS.get_environment("TP_SKIN")
var shots := OS.get_environment("TP_SHOTS")
var receipts := OS.get_environment("TP_RECEIPTS")
var origin := Vector3.ZERO
var baseline: Dictionary
var appearances: Array
var audio: Array = []
var checkpoints: Array = []
var primary := 0
var neighbor := 0

func _initialize() -> void:
	Engine.max_fps = 60
	gate = Completion.new(self, EXPECTED_CASES)
	call_deferred("run_fixture")

func run_fixture() -> void:
	gate.finish(await run_cases())

func run_cases() -> bool:
	if not gate.step(await connect_private(), "private account/world/vendor"): return false
	baseline = client.trading_post_state().duplicate(true)
	appearances = Array(client.tooltip_state().appearance_collection)
	appearances.sort()
	origin = client.account_state().local_player_position
	if not gate.require(baseline.cart.is_empty() and int(baseline.frozen) > 0, "empty local cart and genuine reservation required"): return false
	for row in baseline.types:
		if not gate.require(str(row).split("|")[1] == "1", "initial provider must have all source types checked"): return false
	if not gate.step(await only_type(1), "type1 setup"): return false
	var ids: Array = []
	for row in client.trading_post_state().rows:
		var parts := str(row).split("|")
		if parts[1] == "1": ids.append(int(parts[0]))
	if not gate.require(ids.size() >= 3, "three observed available ordinary products required"): return false
	primary = int(ids[1])
	neighbor = int(ids[2])
	if not gate.step(await ordinary_cart_case(), "ordinary clear/return"): return false
	if not gate.case_done(): return false
	if not gate.step(await frozen_cart_case(), "frozen clear/return"): return false
	if not gate.case_done(): return false
	if not gate.step(await explicit_product_case(), "explicit product hides cart"): return false
	if not gate.case_done(): return false
	if not gate.step(await filter_case(), "silent provider selection"): return false
	if not gate.case_done(): return false
	if not gate.step(await last_removal_case(), "last local removal"): return false
	if not gate.case_done(): return false
	if not gate.step(await clear_case(), "clear modal ownership/restoration"): return false
	if not gate.case_done(): return false
	if not gate.step(await restore_and_leave(), "conservation and HUD"): return false
	return gate.case_done()

func connect_private() -> bool:
	root.size = Vector2i(1920,1080)
	var server := OS.get_environment("TP_SERVER")
	var account := OS.get_environment("TP_ACCOUNT")
	if not gate.require(server == "127.0.0.1:5791" and account.begins_with("fb_") and skin in ["modern","forever"], "explicit private account/server/skin required"): return false
	if not gate.require(not shots.is_empty() and not receipts.is_empty(), "evidence paths required"): return false
	if not gate.require(DisplayServer.screen_get_size() == Vector2i(1920,1080), "private full-HD compositor required"): return false
	for directory in [shots,receipts]:
		if not gate.require(DirAccess.make_dir_recursive_absolute(directory) == OK, "create evidence directory"): return false
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await until(func(): return not client.account_state().assets_starting, "assets",180000): return false
	if not gate.require(client.connect_account(server,account,OS.get_environment("TP_PASSWORD"),false) == "", "account connection"): return false
	if not await until(func(): return control("CharCard_0") != null,"character select",180000): return false
	if not await click_checked("CharCard_0"): return false
	if not await until(func(): return control("EnterWorld") != null,"enter world button",180000): return false
	if not await click_checked("EnterWorld"): return false
	if not await until(func(): return client.account_state().screen == "InWorld" and control("MinimapLauncherButton") != null,"world",180000): return false
	var expected := "Modern" if skin == "modern" else "Forever"
	if not gate.require(client.account_state().ui_skin == expected,"active skin"): return false
	if not await until(func(): return float(client.account_state().local_player_health) > 0.,"alive",120000): return false
	client.set_world_minutes(720.)
	await frames(120)
	origin = client.account_state().local_player_position
	var kits := client.find_child("UiSoundKits",true,false)
	if not gate.require(kits != null,"native sound owner required"): return false
	kits.child_entered_tree.connect(observe_sound)
	if not await interact_vendor(): return false
	return await checkpoint("initial-vendor",sound_count())

func ordinary_cart_case() -> bool:
	if not await click_checked("PerksProgramProduct%d" % primary): return false
	if not await until(func(): return int(client.trading_post_state().selected) == primary,"primary selection"): return false
	if not gate.require(selected_area("PerksProgramSelected%d" % primary) > 0.,"ordinary selected art before cart"): return false
	if not await click_checked("PerksProgramAddToCartButton"): return false
	var count := sound_count()
	if not await click_checked("PerksProgramViewCartButton"): return false
	if not await assert_cart_owner(primary,"ordinary-cart-cleared",count): return false
	var cart_before: Dictionary = client.trading_post_state().duplicate(true)
	for key in [KEY_UP,KEY_DOWN]: await tap(key)
	if not gate.require(client.trading_post_state().cart == cart_before.cart and int(client.trading_post_state().selected) == -1,"cart owns keyboard"): return false
	if not await click_checked("PerksProgramCartCloseButton"): return false
	return await assert_product(primary,"ordinary-return",count+1)

func frozen_cart_case() -> bool:
	if not await click_checked("PerksProgramFrozenSlot"): return false
	if not gate.require(selected_area("PerksProgramFrozenSelected") > 0.,"frozen selected art before cart"): return false
	var count := sound_count()
	if not await click_checked("PerksProgramViewCartButton"): return false
	if not await assert_cart_owner(primary,"frozen-cart-cleared",count): return false
	if not await click_checked("PerksProgramCartCloseButton"): return false
	return await assert_product(int(baseline.frozen),"frozen-return",count+1)

func explicit_product_case() -> bool:
	if not await click_checked("PerksProgramViewCartButton"): return false
	var count := sound_count()
	if not await click_checked("PerksProgramProduct%d" % neighbor): return false
	if not await assert_product(neighbor,"explicit-product-hides-cart",count+1): return false
	return gate.require(client.trading_post_state().cart == PackedInt64Array([primary]),"explicit product retains local cart")

func filter_case() -> bool:
	if not await click_checked("PerksProgramViewCartButton"): return false
	var count := sound_count()
	if not await only_type(2): return false
	var rows: PackedStringArray = client.trading_post_state().rows
	if not gate.require(not rows.is_empty(),"type2 provider has ordinary members"): return false
	var first := int(str(rows[0]).split("|")[0])
	if not await assert_product(first,"filter-hides-cart-silent",count): return false
	if not gate.require(client.trading_post_state().cart == PackedInt64Array([primary]),"filter retains local cart"): return false
	return await only_type(1)

func last_removal_case() -> bool:
	if not await click_checked("PerksProgramProduct%d" % primary): return false
	if not await click_checked("PerksProgramViewCartButton"): return false
	var count := sound_count()
	var row := control("PerksProgramCartItem%d" % primary)
	if not gate.require(row != null,"existing cart item"): return false
	await motion(row.get_global_rect().get_center())
	await frames(5)
	if not await click_checked("PerksProgramCartItem%dRemove" % primary): return false
	if not await assert_product(primary,"last-removal-return",count+1): return false
	return gate.require(client.trading_post_state().cart.is_empty(),"last local item removed")

func clear_case() -> bool:
	if not await click_checked("PerksProgramAddToCartButton"): return false
	if not await click_checked("PerksProgramViewCartButton"): return false
	var count := sound_count()
	if not await click_checked("PerksProgramCartClearButton"): return false
	if not await until(func(): return control("PerksProgramConfirmDialog") != null,"clear confirmation"): return false
	for key in [KEY_UP,KEY_DOWN]: await tap(key)
	if not await assert_cart_owner(primary,"clear-modal-owner",count): return false
	if not await click_checked("PerksProgramConfirmCancelButton"): return false
	if not await assert_cart_owner(primary,"clear-cancel-owner",count): return false
	if not await click_checked("PerksProgramCartClearButton"): return false
	if not await until(func(): return control("PerksProgramConfirmDialog") != null,"second clear confirmation"): return false
	if not await click_checked("PerksProgramConfirmButton"): return false
	if not await assert_product(primary,"clear-return",count+1): return false
	return gate.require(client.trading_post_state().cart.is_empty(),"accepted local Clear empties cart")

func restore_and_leave() -> bool:
	if not await only_type(0): return false
	if not await click_checked("PerksProgramProduct%d" % int(baseline.selected)): return false
	if not await until(func(): return int(client.trading_post_state().selected) == int(baseline.selected),"original selection"): return false
	var now: Dictionary = client.trading_post_state()
	var final_appearances: Array = Array(client.tooltip_state().appearance_collection)
	final_appearances.sort()
	if not gate.require(int(now.tender) == int(baseline.tender) and int(now.frozen) == int(baseline.frozen),"Tender/reservation conserved"): return false
	if not gate.require(final_appearances == appearances and now.cart.is_empty() and int(now.items) == int(baseline.items),"exact collection/cart/catalog conserved"): return false
	if not await checkpoint("restored-before-verdict",sound_count()): return false
	var chat := client.get_node_or_null("ChatFrameUI") as CanvasLayer
	if not gate.require(chat != null and not chat.visible and int(now.hidden_layers) > 0,"fullscreen HUD owner"): return false
	if not await click_checked("PerksProgramLeaveButton"): return false
	if not await until(func(): return not client.trading_post_state().open and chat.visible and control("MinimapLauncherButton") != null,"Leave restores HUD"): return false
	for event in audio:
		if not gate.require(float(event.length) > 0. and (event.playing or event.finished),"decoded selection audio playback"): return false
	return write_receipt("result",{"skin":skin,"baseline":baseline,"appearances":appearances,"checkpoints":checkpoints,"audio":audio,"final":client.trading_post_state(),"cases":EXPECTED_CASES})

func assert_cart_owner(item: int, label: String, expected: int) -> bool:
	if not await until(func(): return client.trading_post_state().cart_visible and int(client.trading_post_state().selected) == -1,label): return false
	var now: Dictionary = client.trading_post_state()
	if not gate.require(now.cart == PackedInt64Array([item]) and not now.cart_preview_items.is_empty(),"cart item/preview retained"): return false
	if not gate.require(selected_area("PerksProgramFrozenSelected") == 0.,"frozen art deselected"): return false
	for id in [primary,neighbor]:
		if not gate.require(selected_area("PerksProgramSelected%d" % id) == 0.,"ordinary art deselected"): return false
	return await checkpoint(label,expected)

func assert_product(id: int, label: String, expected: int) -> bool:
	if not await until(func(): return not client.trading_post_state().cart_visible and int(client.trading_post_state().selected) == id,label): return false
	var name := "PerksProgramFrozenSelected" if id == int(baseline.frozen) else "PerksProgramSelected%d" % id
	if not gate.require(selected_area(name) > 0.,"returned product selected art"): return false
	return await checkpoint(label,expected)

func checkpoint(label: String, expected: int) -> bool:
	await frames(45)
	var position: Vector3 = client.account_state().local_player_position
	if not gate.require(position.distance_to(origin) < 0.0001,"native input must not move world"): return false
	if not gate.require(sound_count() == expected,"selection sound count at " + label): return false
	checkpoints.append({"label":label,"sound_count":sound_count(),"state":client.trading_post_state(),"position":[position.x,position.y,position.z],"world_delta":position.distance_to(origin)})
	await RenderingServer.frame_post_draw
	var image_path := shots.path_join(skin+"-cart-selection-"+label+".png")
	if not gate.require(root.get_texture().get_image().save_png(image_path) == OK,"save full-HD PNG"): return false
	return write_receipt(label,{"state":client.trading_post_state(),"account":client.account_state(),"checkpoint":checkpoints.back()})

func only_type(id: int) -> bool:
	if control("PerksProgramFilterMenu") == null:
		if not await click_checked("PerksProgramFilter"): return false
	var types := control("PerksProgramFilterTypes")
	if not gate.require(types != null,"type menu owner"): return false
	await motion(types.get_global_rect().get_center())
	if not await until(func(): return control("PerksProgramTypeMenu") != null,"type submenu"): return false
	var entries: PackedStringArray = client.trading_post_state().types
	for row in entries:
		var parts := str(row).split("|")
		var wanted := id == 0 or int(parts[0]) == id
		if (parts[1] == "1") != wanted:
			if not await click_checked("PerksProgramFilterType"+parts[0]): return false
	if not await click_checked("PerksProgramFilter"): return false
	return await until(func(): return int(client.trading_post_state().category) == id and control("PerksProgramFilterMenu") == null,"source type filter")

func interact_vendor() -> bool:
	var worker := Thread.new()
	var argv := ["--signal=KILL","15",OS.get_environment("GAME_ENGINE_CLI"),"--socket","/tmp/game-engine-%d.sock" % OS.get_process_id(),"quest","interact","--npc","Tawny Seabraid"]
	if not gate.require(worker.start(func():
		var output := []
		var code := OS.execute("/usr/bin/timeout",argv,output,true)
		return {"code":code,"text":"\n".join(output)}) == OK,"NPC interaction worker"): return false
	while worker.is_alive(): await process_frame
	var reply: Dictionary = worker.wait_to_finish()
	if not gate.require(int(reply.code) == 0,"bounded NPC interaction"): return false
	if not await until(func(): return client.trading_post_state().open or control("GossipOption0Text") != null,"NPC greeting"): return false
	if not client.trading_post_state().open:
		if not await click_checked("GossipOption0Text"): return false
	return await until(func(): return client.trading_post_state().open and int(client.trading_post_state().get("items",0)) > 0,"authoritative vendor",180000)

func control(name: String) -> Control:
	for node in client.find_children(name,"Control",true,false):
		if node.is_visible_in_tree(): return node
	return null

func selected_area(name: String) -> float:
	var node := control(name)
	return node.get_global_rect().get_area() if node != null else 0.

func until(predicate: Callable, label: String, timeout: int = 30000) -> bool:
	var deadline := Time.get_ticks_msec()+timeout
	while Time.get_ticks_msec() < deadline:
		if gate.failed: return false
		if predicate.call(): return true
		await process_frame
	return gate.fail("timeout: "+label)

func click_checked(name: String) -> bool:
	var node := control(name)
	if not gate.require(node != null,"visible control "+name): return false
	var point := node.get_global_rect().get_center()
	await motion(point)
	for down in [true,false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.button_mask = MOUSE_BUTTON_MASK_LEFT if down else 0
		event.pressed = down
		root.push_input(event,true)
		await process_frame
	await frames(5)
	return not gate.failed

func motion(point: Vector2) -> void:
	var event := InputEventMouseMotion.new()
	event.position = point
	event.global_position = point
	Input.parse_input_event(event)
	await process_frame

func tap(key: Key) -> void:
	for down in [true,false]:
		var event := InputEventKey.new()
		event.keycode = key
		event.physical_keycode = key
		event.pressed = down
		root.push_input(event,true)
		await process_frame
	await frames(5)

func frames(count: int) -> void:
	for frame in range(count): await process_frame

func sound_count() -> int:
	var count := 0
	for kit in client.merchant_state().ui_sounds:
		if int(kit) == 218230: count += 1
	return count

func observe_sound(node: Node) -> void:
	var player := node as AudioStreamPlayer
	if player == null or player.stream == null or int(player.stream.resource_name) != 4906955: return
	var event := {"length":player.stream.get_length(),"gain":player.volume_linear,"pitch":player.pitch_scale,"playing":false,"finished":false}
	audio.append(event)
	player.finished.connect(func(): event.finished = true)
	call_deferred("observe_playing",player,event)

func observe_playing(player: AudioStreamPlayer, event: Dictionary) -> void:
	if is_instance_valid(player): event.playing = player.playing

func write_receipt(label: String, value: Dictionary) -> bool:
	var file := FileAccess.open(receipts.path_join(skin+"-cart-selection-"+label+".json"),FileAccess.WRITE)
	if not gate.require(file != null,"write receipt "+label): return false
	file.store_string(JSON.stringify(value,"\t"))
	file.close()
	return true
