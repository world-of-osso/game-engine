extends SceneTree

## Two-client bank and guild bank against a private server (docs/specs/bank-frame.md,
## guild-bank-frame.md). Run once per side; the sides meet at barrier files. Environment:
##   GODOT_TEST_SERVER   private server address
##   BANK_ROLE           "a" (guild master: bank, Warband, guild money/tab/deposit)
##                       or "b" (member: sees the deposit, withdraws it)
##   BANK_ACCOUNT / BANK_CHARACTER   (password fbtest)
##   BANK_SYNC           shared barrier directory
##   BANK_SHOTS          screenshot directory
##   BANK_ADMIN          game-server-admin binary (moves the character to the vault)
## Real input only: banker and vault clicks, frame buttons, bag/slot right-clicks, typed
## money, popup Yes, Escape. Every result is the server's.

const PASSWORD := "fbtest"
const LINEN := 2589
const BANKER := "Olivia Burnside"
const VAULT := "Guild Vault"
const WAIT_MS := 60000
const VAULT_SPOT := {"a": ["-8928.5", "614.0", "100.6"], "b": ["-8928.5", "617.5", "100.6"]}

var client: Node
var role := ""
var sync_dir := ""
var shots := ""

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	role = OS.get_environment("BANK_ROLE")
	sync_dir = OS.get_environment("BANK_SYNC")
	shots = OS.get_environment("BANK_SHOTS")
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(OS.get_environment("GODOT_TEST_SERVER"), OS.get_environment("BANK_ACCOUNT"), PASSWORD, false)
	if error != "":
		fail("connect: " + error)
		return
	if not await enter_world(OS.get_environment("BANK_CHARACTER")):
		return
	var steps := [Callable(self, "character_and_warband_bank")] if role == "a" else []
	steps.append_array([Callable(self, "open_vault"), Callable(self, "guild_bank_shared")])
	for step in steps:
		if not await step.call():
			return
	print("FIXTURE BANK_LIVE_DONE role=", role)
	client.free()
	quit(0)

# --- scenarios -------------------------------------------------------------

## A new character has no bank tab: buy one (1g), deposit and withdraw Linen, then buy
## the first Warband tab, deposit Linen and 1g there, take the Linen back, and close
## with Escape.
func character_and_warband_bank() -> bool:
	if not await use_banker():
		return false
	await capture("1-bank-purchase-prompt")
	var gold := money()
	if not await buy_bank_tab("Do you want to purchase a Bank tab for:\n1g", gold - 10000):
		return false
	if not await deposit_from_bags():
		return false
	await capture("2-bank-deposit")
	await click_at(control("BankFrameItem1").get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
	if not await wait_until(func(): return bag_count(LINEN) == 10 and text("BankFrameItem1Count") == "", "character withdraw"):
		return false
	await press("BankFrameTabSystemTab2")
	gold = money()
	if not await buy_bank_tab("Do you want to purchase a Warband Bank tab for:\n1000g", gold - 10000000):
		return false
	if not await deposit_from_bags():
		return false
	if not await money_prompt("BankFrameMoneyFrameDepositButton", "BankMoney", "BankMoneyPopupAccept", 1, money() - 10000):
		return false
	await capture("3-warband-deposit")
	await click_at(control("BankFrameItem1").get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
	if not await wait_until(func(): return bag_count(LINEN) == 10 and text("BankFrameItem1Count") == "", "Warband withdraw"):
		return false
	await tap(KEY_ESCAPE)
	return await wait_until(func(): return not shown("BankFrame") and not shown("ContainerFrame0") and client.get_node_or_null("GameMenuUI") == null, "Escape closes bank and bags")

func open_vault() -> bool:
	var admin := OS.get_environment("BANK_ADMIN")
	var spot: Array = VAULT_SPOT[role]
	var output := []
	var code := OS.execute(admin, ["set-position", OS.get_environment("BANK_CHARACTER")] + spot, output, true)
	print("FIXTURE MOVE ", code, " ", output)
	if code != 0:
		return fail("set-position failed")
	await wait_frames(120)
	if not await barrier("at-vault"):
		return false
	if not await click_pickable(func(node): return node.has_meta("game_object_name") and str(node.get_meta("game_object_name")) == VAULT, "game_object_server_id"):
		return false
	if not await wait_until(func(): return shown("GuildBankFrame") and text("GuildBankFrameTitleText") == "Vault Keepers" and embedded_backpack_shown(), "guild bank with embedded backpack"):
		return false
	await capture("4-guild-bank-open")
	return await barrier("vault-open")

## A funds the guild (150g), buys its first tab (100g) and deposits Linen; B, with the
## vault open, sees it arrive and withdraws it; A sees it leave and reads both logs.
func guild_bank_shared() -> bool:
	if role == "a":
		if not await money_prompt("GuildBankFrameDepositButton", "GuildBankMoney", "GuildBankMoneyPopupAccept", 150, money() - 1500000):
			return false
		if not await wait_until(func(): return shown("GuildBankFrameBuyInfoPurchaseButton"), "affordable guild tab"):
			return false
		await capture("5-guild-buy-tab")
		await press("GuildBankFrameBuyInfoPurchaseButton")
		if not await wait_until(func(): return popup_text() != "", "guild tab confirmation"):
			return false
		print("FIXTURE GUILD_TAB_POPUP ", popup_text())
		await press("StaticPopup1Button1")
		if not await wait_until(func(): return shown("GuildBankFrameItem1") and shown("GuildBankTab1"), "purchased guild tab"):
			return false
		if not await barrier("tab-bought"):
			return false
		if not await deposit_from_bags():
			return false
		await capture("6-guild-deposit")
		if not await barrier("deposited"):
			return false
		if not await barrier("withdrawn"):
			return false
		if not await wait_until(func(): return text("GuildBankFrameItem1Count") == "" and bag_count(LINEN) == 0, "member withdraw broadcast"):
			return false
		await capture("8-guild-after-member-withdraw")
		await press("GuildBankFrameTab2")
		if not await wait_until(func(): return log_lines().size() >= 2, "item log"):
			return false
		print("FIXTURE GUILD_LOG ", log_lines())
		await capture("9-guild-item-log")
		await press("GuildBankFrameTab3")
		if not await wait_until(func(): return log_lines().size() >= 1, "money log"):
			return false
		print("FIXTURE GUILD_MONEY_LOG ", log_lines())
		await capture("10-guild-money-log")
	else:
		if not await barrier("tab-bought"):
			return false
		if not await wait_until(func(): return shown("GuildBankTab1"), "purchased tab broadcast"):
			return false
		await press("GuildBankTab1")
		if not await barrier("deposited"):
			return false
		if not await wait_until(func(): return text("GuildBankFrameItem1Count") == "10", "deposit broadcast"):
			return false
		await capture("7-member-sees-deposit")
		var before := bag_count(LINEN)
		await click_at(control("GuildBankFrameItem1").get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
		if not await wait_until(func(): return bag_count(LINEN) == before + 10 and text("GuildBankFrameItem1Count") == "", "member withdraw"):
			return false
		await capture("8-member-withdrew")
		# Rank 3 has no gold allowance (set-guild-bank-rights ... gold 0 0).
		print("FIXTURE MEMBER_WITHDRAW_BUTTON shown=", shown("GuildBankFrameWithdrawButton"))
		if not await barrier("withdrawn"):
			return false
	return await barrier("done")

# --- steps -----------------------------------------------------------------

func use_banker() -> bool:
	if not await click_pickable(func(node): return node.get_parent() != null and node.get_parent().name == "WorldUnits" and str(node.name) == BANKER, "unit_server_id"):
		return false
	# Gossip menu 699: "I would like to check my deposit box." (option type 9).
	if not await wait_until(func(): return shown("AuctionGossip") and not client.find_children("AuctionGossipOption*", "", true, false).is_empty(), "banker gossip"):
		return false
	await capture("0-banker-gossip")
	await click(client.find_children("AuctionGossipOption*", "", true, false)[0])
	return await wait_until(func(): return shown("BankFrame") and shown("ContainerFrame0") and not shown("AuctionGossip"), "BankFrame and bags")

func buy_bank_tab(prompt: String, gold_after: int) -> bool:
	if not await wait_until(func(): return shown("BankFramePurchasePromptButton"), "bank purchase prompt"):
		return false
	await press("BankFramePurchasePromptButton")
	if not await wait_until(func(): return popup_text() == prompt, "confirmation '%s' (shown '%s')" % [prompt, popup_text()]):
		return false
	await press("StaticPopup1Button1")
	return await wait_until(func(): return money() == gold_after and shown("BankFrameItem1"), "tab bought for %d" % gold_after)

## Right-click the backpack Linen: with BankFrame/GuildBankFrame shown it is deposited.
func deposit_from_bags() -> bool:
	for entry in client.merchant_state().bags:
		if entry.item_id == LINEN and entry.bag == 0:
			var slot := "ContainerFrame0Slot%d" % entry.slot
			await click_at(control(slot).get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
			var count := "GuildBankFrameItem1Count" if shown("GuildBankFrame") else "BankFrameItem1Count"
			return await wait_until(func(): return bag_count(LINEN) == 0 and text(count) == "10", "deposit into " + count)
	return fail("no Linen in the backpack: %s" % [client.merchant_state().bags])

func money_prompt(button: String, boxes: String, accept: String, gold: int, money_after: int) -> bool:
	await press(button)
	if not await wait_until(func(): return shown(boxes + "Gold"), button + " prompt"):
		return false
	await press(boxes + "Gold")
	await tap(KEY_END)
	for index in range(8):
		await tap(KEY_BACKSPACE)
	await type_text(str(gold))
	await press(accept)
	return await wait_until(func(): return money() == money_after, "%dg moved" % gold)

## Right-click the nearest matching target once it is pickable on screen.
func click_pickable(matches: Callable, id_meta: String) -> bool:
	for turn in range(48):
		var camera := root.get_viewport().get_camera_3d()
		if camera != null:
			for node in nearest_first(client.find_children("*", "Node3D", true, false).filter(matches)):
				var area := node.find_child("UnitPick", true, false) as Area3D
				if area == null or area.get_child_count() == 0:
					continue
				var center := (area.get_child(0) as Node3D).global_position
				if not camera.is_position_in_frustum(center):
					continue
				var point := camera.unproject_position(center)
				if UnitPicker.pick(camera, point) == int(area.get_meta(id_meta) if area.has_meta(id_meta) else node.get_meta(id_meta)):
					await click_at(point, MOUSE_BUTTON_RIGHT)
					return true
		# Turn Left (ArrowLeft) until the target is in view.
		push_key(KEY_LEFT, 0, true)
		await wait_frames(8)
		push_key(KEY_LEFT, 0, false)
		await wait_frames(4)
	await capture("fail-pick")
	return fail("no pickable target in view")

# --- observation -----------------------------------------------------------

func nearest_first(nodes: Array) -> Array:
	var player := client.get_node("WorldUnits/" + OS.get_environment("BANK_CHARACTER")) as Node3D
	nodes.sort_custom(func(a, b): return a.global_position.distance_to(player.global_position) < b.global_position.distance_to(player.global_position))
	return nodes

## GuildBankFrame draws its own backpack; the standalone ContainerFrame0 stays hidden.
func embedded_backpack_shown() -> bool:
	var host := client.get_node_or_null("GuildBankUI")
	var bag := host.find_child("ContainerFrame0", true, false) as Control if host != null else null
	return bag != null and bag.is_visible_in_tree()

func bag_count(item_id: int) -> int:
	var total := 0
	for entry in client.merchant_state().bags:
		if entry.item_id == item_id:
			total += entry.count
	return total

func money() -> int:
	return client.merchant_state().money

func popup_text() -> String:
	return text("StaticPopup1Text") if shown("StaticPopup1") else ""

func log_lines() -> Array:
	var lines := []
	for index in range(25):
		var line := text("GuildBankFrameLogLine%d" % (index + 1))
		if line != "":
			lines.append(line)
	return lines

## The visible control of that name: the standalone and embedded backpacks share names.
func control(name: String) -> Control:
	var first: Control = null
	for node in client.find_children(name, "Control", true, false):
		if node.is_visible_in_tree():
			return node
		if first == null:
			first = node
	return first

func shown(name: String) -> bool:
	var node := control(name)
	return node != null and node.is_visible_in_tree()

func text(name: String) -> String:
	var node := control(name)
	return node.text if node is Label and node.is_visible_in_tree() else ""

func wait_until(predicate: Callable, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call():
			await wait_frames(10)
			return true
	await capture("fail")
	return fail("%s timed out waiting for %s: bank=%s" % [role, what, client.bank_state()])

func barrier(name: String) -> bool:
	FileAccess.open("%s/%s-%s" % [sync_dir, role, name], FileAccess.WRITE).store_string("1")
	var other := "%s/%s-%s" % [sync_dir, "b" if role == "a" else "a", name]
	var deadline := Time.get_ticks_msec() + 600000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if FileAccess.file_exists(other):
			return true
	return fail("%s: partner never reached %s" % [role, name])

func enter_world(character: String) -> bool:
	var deadline := Time.get_ticks_msec() + 120000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.reply_received and state.screen == "CharacterSelect" and state.character_count >= 1 and client.get_node_or_null("CharacterSelectUI") != null and not state.assets_starting:
			break
	var ui = client.get_node_or_null("CharacterSelectUI")
	if ui == null:
		return fail("No character select: " + str(client.account_state()))
	await click(ui.find_child("CharCard_0", true, false))
	if ui.find_child("CharSelectCharacterName", true, false).text != character:
		return fail("Card 0 is not " + character)
	await click(ui.find_child("EnterWorld", true, false))
	deadline = Time.get_ticks_msec() + 180000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "InWorld" and state.local_player_position != null and state.terrain.pending_count == 0 and not state.terrain.parsed_tiles.is_empty():
			await wait_frames(120)
			return true
	return fail("Timed out entering the world: " + str(client.account_state()))

# --- input -----------------------------------------------------------------

func press(name: String) -> void:
	await click(control(name))

func click(target: Control) -> void:
	await click_at(target.get_global_rect().get_center(), MOUSE_BUTTON_LEFT)

func click_at(point: Vector2, button: int) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	Input.warp_mouse(point)
	await wait_frames(2)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = button
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await wait_frames(3)

func type_text(text: String) -> void:
	for character in text:
		var code := OS.find_keycode_from_string(character.to_upper())
		push_key(code, character.unicode_at(0), true)
		await process_frame
		push_key(code, 0, false)
		await process_frame

func tap(code: Key) -> void:
	push_key(code, 0, true)
	await process_frame
	push_key(code, 0, false)
	await wait_frames(2)

func push_key(code: Key, unicode: int, pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.unicode = unicode
	event.pressed = pressed
	root.push_input(event, true)

func capture(name: String) -> void:
	await RenderingServer.frame_post_draw
	var path := "%s/%s-%s.png" % [shots, role, name]
	root.get_texture().get_image().save_png(path)
	print("FIXTURE SHOT ", path)

func wait_frames(count: int) -> void:
	for frame in range(count):
		await process_frame

func fail(message: String) -> bool:
	push_error(message)
	quit(1)
	return false
