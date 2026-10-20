extends SceneTree
## SLOW_PURCHASE native proof: private feature-held Purchase/Checkout persistence, natural 10s notice.

const Completion = preload("res://tests/completion.gd")
const EXPECTED_CASES := 5
const SLOW_TEXT := "It's taking longer than usual to process your purchase. Processing will continue in the background."
const RETURN_TEXT := "Return to Trading Post"
const SLOW_SECONDS := 10.0
var gate: RefCounted
var client: Node
var skin := OS.get_environment("TP_SKIN")
var shots := OS.get_environment("TP_SHOTS")
var receipts := OS.get_environment("TP_RECEIPTS")
var hold := OS.get_environment("TP_HOLD_DIR")
var origin := Vector3.ZERO
var baseline: Dictionary
var appearances: Array
var checkpoints: Array = []
var primary := 0
var neighbor := 0
var prices: Dictionary = {}

func _initialize() -> void:
	Engine.max_fps = 60
	gate = Completion.new(self, EXPECTED_CASES)
	call_deferred("run_fixture")

func run_fixture() -> void:
	gate.finish(await run_cases())

func run_cases() -> bool:
	if not gate.step(await connect_private(true), "private account/world/character frame/vendor"): return false
	baseline = client.trading_post_state().duplicate(true)
	appearances = sorted_appearances()
	if not gate.require(baseline.cart.is_empty() and not baseline.pending, "empty idle cart required"): return false
	if not gate.step(await choose_products(), "two affordable ordinary products"): return false
	if not gate.step(await purchase_return_case(), "Purchase natural slow notice and Return"): return false
	if not gate.case_done(): return false
	if not gate.step(await checkout_escape_case(), "Checkout natural slow notice and Escape"): return false
	if not gate.case_done(): return false
	if not gate.step(await early_success_case(), "matching success before deadline"): return false
	if not gate.case_done(): return false
	if not gate.step(await session_hide_case(), "private kick hides vendor before deadline"): return false
	if not gate.case_done(): return false
	if not gate.step(await restore_and_leave(), "conservation and HUD"): return false
	return gate.case_done()

func connect_private(first: bool) -> bool:
	var server := OS.get_environment("TP_SERVER")
	var account := OS.get_environment("TP_ACCOUNT")
	if first:
		root.size = Vector2i(1920,1080)
		if not gate.require(server == "127.0.0.1:5791" and account.begins_with("fb_") and skin in ["modern","forever"], "explicit private account/server/skin required"): return false
		if not gate.require(not shots.is_empty() and not receipts.is_empty(), "evidence paths required"): return false
		if not gate.require(hold.is_absolute_path() and DirAccess.dir_exists_absolute(hold) and hold_markers().is_empty(), "private empty hold directory required"): return false
		if not gate.require(OS.get_environment("TP_ADMIN").is_absolute_path(), "private admin binary required"): return false
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
	if first:
		# Real C-opened panel behind the fullscreen vendor; CloseAllWindows would close it.
		await tap(KEY_C)
		if not await until(func(): return client.character_frame_state().open,"character frame opened by C"): return false
	if not await interact_vendor(): return false
	if first and not gate.require(client.character_frame_state().open,"vendor show keeps hidden character frame"): return false
	return await checkpoint("vendor-" + ("initial" if first else "reconnected"))

func choose_products() -> bool:
	if not await only_type(1): return false
	var ids: Array = []
	for row in client.trading_post_state().rows:
		var parts := str(row).split("|")
		if parts[1] == "1" and int(parts[0]) != int(baseline.frozen): ids.append(int(parts[0]))
	if not gate.require(ids.size() >= 2, "two observed available ordinary products required"): return false
	primary = int(ids[0])
	neighbor = int(ids[1])
	for id in [primary,neighbor]:
		if not await select(id): return false
		var now: Dictionary = client.trading_post_state()
		if not gate.require(not now.purchased and not now.collected, "unowned product %d" % id): return false
		prices[id] = int(now.price)
	return gate.require(int(prices[primary])+int(prices[neighbor]) <= int(baseline.tender), "Tender covers two-item checkout")

func purchase_return_case() -> bool:
	if not await select(primary): return false
	var tender := int(client.trading_post_state().tender)
	var price := int(client.trading_post_state().price)
	if not await click_checked("PerksProgramPurchaseButton"): return false
	if not await until(func(): return control("PerksProgramConfirmDialog") != null,"purchase confirmation"): return false
	var timing: Dictionary = await held_confirm("purchase-return")
	if timing.is_empty(): return false
	if not await assert_slow_notice("purchase-slow", timing, tender, false): return false
	if not await click_checked("PerksProgramSlowPurchaseButton"): return false
	if not await until(func(): return control("PerksProgramSlowPurchaseDialog") == null and control("PerksProgramConfirmDialog") == null,"Return dismisses notice"): return false
	if not gate.require(client.trading_post_state().open and not client.trading_post_state().pending,"Return keeps vendor and releases modal"): return false
	if not gate.require(FileAccess.file_exists(hold.path_join("held")) and int(client.trading_post_state().tender) == tender,"Return does not cancel or commit server work"): return false
	if not await checkpoint("purchase-returned-held",2): return false
	if not await release_and_wait(): return false
	if not await until(func(): return client.trading_post_state().purchased and int(client.trading_post_state().tender) == tender-price,"released purchase commits",30000): return false
	if not await never_slow(0.5,"no notice after released purchase"): return false
	if not await checkpoint("purchase-committed"): return false
	return await refund(primary, tender, "purchase-return", timing)

func checkout_escape_case() -> bool:
	var tender := int(client.trading_post_state().tender)
	var total := int(prices[primary])+int(prices[neighbor])
	for id in [primary,neighbor]:
		if not await select(id): return false
		if not await click_checked("PerksProgramAddToCartButton"): return false
	if not await click_checked("PerksProgramViewCartButton"): return false
	if not await click_checked("PerksProgramCartPurchaseButton"): return false
	if not await until(func(): return control("PerksProgramConfirmDialog") != null,"checkout confirmation"): return false
	var timing: Dictionary = await held_confirm("checkout-escape")
	if timing.is_empty(): return false
	if not gate.require(client.trading_post_state().cart.is_empty(),"accepted checkout clears local cart at enqueue"): return false
	if not await assert_slow_notice("checkout-slow", timing, tender, false): return false
	await tap(KEY_ESCAPE)
	if not await until(func(): return control("PerksProgramSlowPurchaseDialog") == null,"Escape dismisses notice"): return false
	var now: Dictionary = client.trading_post_state()
	if not gate.require(now.open and not now.pending and int(now.tender) == tender,"Escape keeps vendor without commit"): return false
	if not gate.require(client.character_frame_state().open,"Escape does not close hidden character frame"): return false
	if not gate.require(FileAccess.file_exists(hold.path_join("held")),"Escape does not cancel server checkout"): return false
	if not await checkpoint("checkout-escaped-held",2): return false
	if not await release_and_wait(): return false
	if not await until(func(): return int(client.trading_post_state().tender) == tender-total,"released checkout commits",30000): return false
	if not await never_slow(0.5,"no notice after released checkout"): return false
	if not await checkpoint("checkout-committed"): return false
	if not await refund(primary, tender-int(prices[neighbor]), "checkout-primary", timing): return false
	return await refund(neighbor, tender, "checkout-neighbor", timing)

func early_success_case() -> bool:
	if not await select(primary): return false
	var tender := int(client.trading_post_state().tender)
	var price := int(client.trading_post_state().price)
	if not await click_checked("PerksProgramPurchaseButton"): return false
	if not await until(func(): return control("PerksProgramConfirmDialog") != null,"early purchase confirmation"): return false
	var timing: Dictionary = await held_confirm("early-success")
	if timing.is_empty(): return false
	if not await never_slow_until(int(timing.before)+2000000,"held before early release"): return false
	if not await release_and_wait(): return false
	if not await until(func(): return client.trading_post_state().purchased and not client.trading_post_state().pending and control("PerksProgramConfirmDialog") == null and int(client.trading_post_state().tender) == tender-price,"matching success closes modal",30000): return false
	timing["success_observed"] = Time.get_ticks_usec()
	if not gate.require(int(timing.success_observed) < int(timing.before)+int(SLOW_SECONDS*1000000.),"success precedes deadline"): return false
	if not await never_slow_until(int(timing.after)+int((SLOW_SECONDS+1.5)*1000000.),"matching success cancels timer"): return false
	if not await checkpoint("early-success-past-deadline"): return false
	return await refund(primary, tender, "early-success", timing)

func session_hide_case() -> bool:
	if not await select(primary): return false
	var tender := int(client.trading_post_state().tender)
	var price := int(client.trading_post_state().price)
	if not await click_checked("PerksProgramPurchaseButton"): return false
	if not await until(func(): return control("PerksProgramConfirmDialog") != null,"hide purchase confirmation"): return false
	var timing: Dictionary = await held_confirm("session-hide")
	if timing.is_empty(): return false
	if not await kick_owned_account(): return false
	if not await until(func(): return client.account_state().screen == "Login" and not client.trading_post_state().open,"private kick hides vendor",8000): return false
	timing["hidden_observed"] = Time.get_ticks_usec()
	if not gate.require(int(timing.hidden_observed) < int(timing.before)+int(SLOW_SECONDS*1000000.),"hide precedes deadline"): return false
	if not await never_slow_until(int(timing.after)+int((SLOW_SECONDS+1.5)*1000000.),"vendor hide cancels timer"): return false
	await RenderingServer.frame_post_draw
	if not gate.require(root.get_texture().get_image().save_png(shots.path_join(skin+"-slow-purchase-hidden-past-deadline.png")) == OK,"save hidden PNG"): return false
	if not await release_and_wait(): return false
	if not await connect_private(false): return false
	if not await only_type(1): return false
	if not await select(primary): return false
	if not gate.require(client.trading_post_state().purchased and int(client.trading_post_state().tender) == tender-price,"held purchase committed after hide"): return false
	return await refund(primary, tender, "session-hide", timing)

func restore_and_leave() -> bool:
	if not await only_type(0): return false
	if not await select(int(baseline.selected)): return false
	var now: Dictionary = client.trading_post_state()
	if not gate.require(int(now.tender) == int(baseline.tender) and int(now.frozen) == int(baseline.frozen),"Tender/reservation conserved"): return false
	if not gate.require(sorted_appearances() == appearances and now.cart.is_empty() and int(now.items) == int(baseline.items),"exact collection/cart/catalog conserved"): return false
	if not gate.require(hold_markers().is_empty(),"no hold marker remains"): return false
	if not await checkpoint("restored-before-verdict"): return false
	var chat := client.get_node_or_null("ChatFrameUI") as CanvasLayer
	if not gate.require(chat != null and not chat.visible and int(now.hidden_layers) > 0,"fullscreen HUD owner"): return false
	if not await click_checked("PerksProgramLeaveButton"): return false
	if not await until(func(): return not client.trading_post_state().open and chat.visible and control("MinimapLauncherButton") != null,"Leave restores HUD"): return false
	return write_receipt("result",{"skin":skin,"baseline":baseline,"appearances":appearances,"checkpoints":checkpoints,"final":client.trading_post_state(),"cases":EXPECTED_CASES})

## Arms one hold, accepts the open confirmation and returns dispatch bounds once the server claims it.
func held_confirm(label: String) -> Dictionary:
	if not gate.require(hold_markers().is_empty(),"clean hold before "+label): return {}
	var temporary := hold.path_join("arm.tmp")
	var file := FileAccess.open(temporary,FileAccess.WRITE)
	if not gate.require(file != null,"write arm"): return {}
	file.store_string(label)
	file.close()
	if not gate.require(DirAccess.rename_absolute(temporary,hold.path_join("arm")) == OK,"atomic arm"): return {}
	var button := control("PerksProgramConfirmButton")
	if not gate.require(button != null,"confirmation accept button"): return {}
	var point := button.get_global_rect().get_center()
	await motion(point)
	var timing := {"label":label,"before":Time.get_ticks_usec()}
	for down in [true,false]:
		push_button(point,down)
		await process_frame
	timing["after"] = Time.get_ticks_usec()
	if not await until(func(): return client.trading_post_state().pending,label+" pending",3000): return {}
	if not await until(func(): return FileAccess.file_exists(hold.path_join("held")),label+" server claimed hold",5000): return {}
	timing["held_observed"] = Time.get_ticks_usec()
	if not gate.require(int(timing.held_observed) < int(timing.before)+2000000,label+" hold claimed promptly"): return {}
	return timing

## Last original and first slow rendered frames bracket the real 10s request deadline.
func assert_slow_notice(label: String, request: Dictionary, tender: int, purchased: bool) -> bool:
	var last_original := 0
	var first_slow := 0
	var previous := Time.get_ticks_usec()
	var gap := 0
	var deadline := previous+int((SLOW_SECONDS+3.)*1000000.)
	while Time.get_ticks_usec() < deadline:
		await RenderingServer.frame_post_draw
		var now := Time.get_ticks_usec()
		gap = maxi(gap,now-previous)
		previous = now
		if control("PerksProgramSlowPurchaseDialog") != null:
			first_slow = now
			break
		if not gate.require(control("PerksProgramConfirmDialog") != null and control("PerksProgramConfirmSpinner") != null,label+" original pending dialog until notice"): return false
		last_original = now
	if not gate.require(first_slow > 0 and last_original > 0,label+" natural notice appeared"): return false
	if not gate.require(first_slow >= int(request.before)+int(SLOW_SECONDS*1000000.),label+" notice not early"): return false
	if not gate.require(last_original <= int(request.after)+int(SLOW_SECONDS*1000000.),label+" notice not late"): return false
	# Precision is reported, not asserted: the deadline lies inside [last_original, first_slow].
	var timing := {"last_original":last_original,"first_slow":first_slow,"bracket_usec":first_slow-last_original,"max_frame_gap":gap,"request":request}
	var text := control("PerksProgramSlowPurchaseTextBefore")
	var caption := control("PerksProgramSlowPurchaseButtonDialogButtonText")
	if not gate.require(text != null and str(text.get("text")) == SLOW_TEXT,label+" source notice text"): return false
	if not gate.require(caption != null and str(caption.get("text")) == RETURN_TEXT,label+" source Return caption"): return false
	if not gate.require(control("PerksProgramSlowPurchaseCancelButton") == null and control("PerksProgramConfirmSpinner") == null,label+" one button, no spinner"): return false
	var now: Dictionary = client.trading_post_state()
	if not gate.require(now.pending and int(now.tender) == tender and bool(now.purchased) == purchased and FileAccess.file_exists(hold.path_join("held")),label+" notice leaves authority held"): return false
	if not write_receipt(label+"-timing",timing): return false
	return await checkpoint(label,2)

func release_and_wait() -> bool:
	var file := FileAccess.open(hold.path_join("release"),FileAccess.WRITE)
	if not gate.require(file != null,"write release"): return false
	file.close()
	return await until(func(): return hold_markers().is_empty(),"server consumed release",10000)

func refund(id: int, tender: int, label: String, timing: Dictionary) -> bool:
	if not await select(id): return false
	if not gate.require(hold_markers().is_empty() and client.trading_post_state().purchased and int(client.trading_post_state().refund_seconds) > 0,"private refund window for "+label): return false
	if not await click_checked("PerksProgramRefundButton"): return false
	if not await until(func(): return control("PerksProgramConfirmDialog") != null,label+" refund confirmation"): return false
	if not await click_checked("PerksProgramConfirmButton"): return false
	if not await until(func(): return not client.trading_post_state().pending and not client.trading_post_state().purchased and control("PerksProgramConfirmDialog") == null and int(client.trading_post_state().tender) == tender,label+" refund restores Tender",30000): return false
	return write_receipt(label+"-request",timing)

func never_slow(seconds: float, label: String) -> bool:
	return await never_slow_until(Time.get_ticks_usec()+int(seconds*1000000.),label)

func never_slow_until(usec: int, label: String) -> bool:
	while Time.get_ticks_usec() < usec:
		if not gate.require(control("PerksProgramSlowPurchaseDialog") == null,label): return false
		await process_frame
	return not gate.failed

func kick_owned_account() -> bool:
	var worker := Thread.new()
	var argv := ["--signal=KILL","15","/usr/bin/env","GAME_SERVER_ADMIN_SOCKET=/tmp/game-server-admin-5791.sock",OS.get_environment("TP_ADMIN"),"kick-account",OS.get_environment("TP_ACCOUNT"),"native-proof-hide"]
	if not gate.require(worker.start(func():
		var output := []
		var code := OS.execute("/usr/bin/timeout",argv,output,true)
		return {"code":code,"text":"\n".join(output)}) == OK,"private kick worker"): return false
	while worker.is_alive(): await process_frame
	var reply: Dictionary = worker.wait_to_finish()
	if not write_receipt("kick",reply): return false
	return gate.require(int(reply.code) == 0,"bounded private owned-account kick")

func select(id: int) -> bool:
	if int(client.trading_post_state().selected) == id: return true
	if client.trading_post_state().cart_visible:
		if not await click_checked("PerksProgramCartCloseButton"): return false
		if not await until(func(): return not client.trading_post_state().cart_visible,"cart closed"): return false
	if control("PerksProgramProduct%d" % id) != null:
		if not await click_checked("PerksProgramProduct%d" % id): return false
	else:
		# Purchased rows can sit outside the virtualized page; real Up/Down reveals them.
		for key in [KEY_DOWN,KEY_UP]:
			for step in range(40):
				var before := int(client.trading_post_state().selected)
				if before == id: break
				await tap(key,1)
				if int(client.trading_post_state().selected) == before: break
	return await until(func(): return int(client.trading_post_state().selected) == id,"select %d" % id)

func hold_markers() -> PackedStringArray:
	var found := PackedStringArray()
	for name in ["arm","arm.tmp","held","release"]:
		if FileAccess.file_exists(hold.path_join(name)): found.append(name)
	return found

func sorted_appearances() -> Array:
	var values := Array(client.tooltip_state().appearance_collection)
	values.sort()
	return values

## Held-state captures settle briefly so release stays inside the server's 30s hold bound.
func checkpoint(label: String, settle: int = 30) -> bool:
	await frames(settle)
	var position: Vector3 = client.account_state().local_player_position
	if not gate.require(position.distance_to(origin) < 0.0001,"native input must not move world"): return false
	checkpoints.append({"label":label,"at":Time.get_ticks_usec(),"state":client.trading_post_state(),"character_frame":client.character_frame_state().open,"hold":hold_markers(),"position":[position.x,position.y,position.z],"world_delta":position.distance_to(origin)})
	await RenderingServer.frame_post_draw
	if not gate.require(root.get_texture().get_image().save_png(shots.path_join(skin+"-slow-purchase-"+label+".png")) == OK,"save full-HD PNG"): return false
	return write_receipt(label,{"state":client.trading_post_state(),"account":client.account_state(),"checkpoint":checkpoints.back()})

func only_type(id: int) -> bool:
	if control("PerksProgramFilterMenu") == null:
		if not await click_checked("PerksProgramFilter"): return false
	var types := control("PerksProgramFilterTypes")
	if not gate.require(types != null,"type menu owner"): return false
	await motion(types.get_global_rect().get_center())
	if not await until(func(): return control("PerksProgramTypeMenu") != null,"type submenu"): return false
	for row in client.trading_post_state().types:
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

func until(predicate: Callable, label: String, timeout: int = 30000) -> bool:
	var deadline := Time.get_ticks_msec()+timeout
	while Time.get_ticks_msec() < deadline:
		if gate.failed: return false
		if predicate.call(): return true
		await process_frame
	if client != null:
		if not write_receipt("timeout",{"label":label,"state":client.trading_post_state(),"hold":hold_markers()}): return false
	return gate.fail("timeout: "+label)

func push_button(point: Vector2, down: bool) -> void:
	var event := InputEventMouseButton.new()
	event.position = point
	event.global_position = point
	event.button_index = MOUSE_BUTTON_LEFT
	event.button_mask = MOUSE_BUTTON_MASK_LEFT if down else 0
	event.pressed = down
	root.push_input(event,true)

func click_checked(name: String) -> bool:
	var node := control(name)
	if not gate.require(node != null,"visible control "+name): return false
	var point := node.get_global_rect().get_center()
	await motion(point)
	for down in [true,false]:
		push_button(point,down)
		await process_frame
	await frames(5)
	return not gate.failed

func motion(point: Vector2) -> void:
	var event := InputEventMouseMotion.new()
	event.position = point
	event.global_position = point
	Input.parse_input_event(event)
	await process_frame

func tap(key: Key, settle: int = 5) -> void:
	for down in [true,false]:
		var event := InputEventKey.new()
		event.keycode = key
		event.physical_keycode = key
		event.pressed = down
		root.push_input(event,true)
		await process_frame
	await frames(settle)

func frames(count: int) -> void:
	for frame in range(count): await process_frame

func write_receipt(label: String, value: Dictionary) -> bool:
	var file := FileAccess.open(receipts.path_join(skin+"-slow-purchase-"+label+".json"),FileAccess.WRITE)
	if not gate.require(file != null,"write receipt "+label): return false
	file.store_string(JSON.stringify(value,"\t"))
	file.close()
	return true
