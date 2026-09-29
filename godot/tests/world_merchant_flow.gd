extends SceneTree

# Vendor interaction against the dev server (docs/specs/merchant-frame.md): hovering
# Brother Danil shows the Buy cursor; a real right-click targets him and opens the
# MerchantFrame with his authored items and the backpack; right-clicking the bread buys
# one purchase (money down 25, 5 bread in the bags); right-clicking the Linen Cloth in
# the backpack sells it into buyback; Shift-click on the water opens the split frame and
# typing 2 + Enter buys 2 purchases; Escape, the close button and walking out of range
# each close the frame.
#
# Setup while Fbworldmap is offline (game-server-admin): teleport Fbworldmap 0 -8904 -110.5 82.1,
# set-gold Fbworldmap 1000, grant-item Fbworldmap 2589 5.

const ACCOUNT := "fb_worldmap"
const PASSWORD := "fbtest"
const CHARACTER := "Fbworldmap"
const VENDOR := "Brother Danil"
const VENDOR_ITEMS := ["Tough Hunk of Bread", "Refreshing Spring Water", "Small Brown Pouch"]
const WORLD_WAIT_MS := 120000
const NPC_WAIT_MS := 60000
const REPLY_WAIT_MS := 8000
const SHOTS := "/tmp/claude/merchant/"

var client: Node

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	DirAccess.make_dir_recursive_absolute(SHOTS)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if server != "127.0.0.1:5000":
		fail("GODOT_TEST_SERVER must explicitly select 127.0.0.1:5000")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, ACCOUNT, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world():
		return
	var npc: Dictionary = await find_vendor()
	if npc.is_empty():
		return
	print("FIXTURE VENDOR ", npc.name, " id ", npc.id, " at ", npc.point)

	await hover_point(npc.point)
	await frames(2)
	if client.merchant_state().cursor != "Buy":
		fail("Hovering the vendor shows cursor '%s', expected Buy" % client.merchant_state().cursor)
		return
	print("FIXTURE HOVER_CURSOR Buy")

	if not await open_vendor(npc):
		return
	await capture("01-merchant-open.png")
	if client.target_state().target != npc.id:
		fail("Right-click did not target the vendor: " + str(client.target_state()))
		return
	if merchant_text("MerchantItem1Name") != VENDOR_ITEMS[0] or merchant_text("MerchantFrameTitleText") != VENDOR:
		fail("Frame shows '%s' / '%s'" % [merchant_text("MerchantFrameTitleText"), merchant_text("MerchantItem1Name")])
		return
	if not merchant_visible("ContainerFrame0"):
		fail("The backpack did not open with the vendor")
		return

	# Buy: right-click the bread cell.
	var before: Dictionary = client.merchant_state()
	var bread_before := bag_count(before, 4540)
	await click_control(merchant_control("MerchantItem1"), MOUSE_BUTTON_RIGHT)
	if not await wait_for(func(s): return s.money == before.money - 25 and bag_count(s, 4540) == bread_before + 5, "bread purchase"):
		return
	print("FIXTURE BOUGHT bread: money ", before.money, " -> ", client.merchant_state().money)
	await capture("02-bought-bread.png")

	# Sell: right-click the Linen Cloth in the backpack.
	before = client.merchant_state()
	var linen := bag_slot(before, 2589)
	if linen.is_empty():
		fail("No Linen Cloth in the bags: " + str(before.bags))
		return
	await click_control(merchant_control("ContainerFrame%dSlot%d" % [linen.bag, linen.slot]), MOUSE_BUTTON_RIGHT)
	if not await wait_for(func(s): return bag_count(s, 2589) == 0 and s.buyback.has("Linen Cloth") and s.money > before.money, "linen sale"):
		return
	print("FIXTURE SOLD linen: money ", before.money, " -> ", client.merchant_state().money, " buyback ", client.merchant_state().buyback)
	await capture("03-sold-linen.png")
	await click_control(merchant_control("MerchantFrameTab2"), MOUSE_BUTTON_LEFT)
	await frames(2)
	if merchant_text("MerchantFrameTitleText") != "Merchant Buyback" or merchant_text("MerchantItem1Name") != "Linen Cloth":
		fail("Buyback tab shows '%s' / '%s'" % [merchant_text("MerchantFrameTitleText"), merchant_text("MerchantItem1Name")])
		return
	await capture("04-buyback-tab.png")
	await click_control(merchant_control("MerchantFrameTab1"), MOUSE_BUTTON_LEFT)
	await frames(2)

	# Stack split: Shift-click the water, type 2, Enter.
	before = client.merchant_state()
	var water_before := bag_count(before, 159)
	await click_control(merchant_control("MerchantItem2"), MOUSE_BUTTON_LEFT, true)
	if not client.merchant_state().split_open or not merchant_visible("StackSplitFrame"):
		fail("Shift-click on the water did not open the split frame")
		return
	await tap(KEY_2)
	await capture("05-split-frame.png")
	await tap(KEY_ENTER)
	if not await wait_for(func(s): return s.money == before.money - 10 and bag_count(s, 159) == water_before + 10, "split purchase"):
		return
	print("FIXTURE SPLIT bought 10 water")

	# Escape closes the frame and ends the interaction.
	await tap(KEY_ESCAPE)
	if client.merchant_state().open or merchant_visible("MerchantFrame"):
		fail("Escape did not close the merchant frame")
		return
	if client.get_node_or_null("GameMenuUI") != null:
		fail("Escape closing the merchant also opened the game menu")
		return
	await capture("06-escape-closed.png")
	print("FIXTURE ESCAPE_CLOSED")

	# The close button.
	npc = await find_vendor()
	if npc.is_empty() or not await open_vendor(npc):
		return
	await click_control(merchant_control("MerchantFrameCloseButton"), MOUSE_BUTTON_LEFT)
	if client.merchant_state().open:
		fail("The close button did not close the merchant frame")
		return
	print("FIXTURE CLOSE_BUTTON_CLOSED")

	# Walking out of range: the server ends the interaction.
	npc = await find_vendor()
	if npc.is_empty() or not await open_vendor(npc):
		return
	push_key(KEY_S, true)
	var closed := await wait_for(func(s): return not s.open, "server close on walking away", 15000)
	push_key(KEY_S, false)
	if not closed:
		return
	await capture("07-walked-away.png")
	print("FIXTURE RANGE_CLOSED")
	print("FIXTURE WORLD_MERCHANT_DONE")
	client.free()
	quit(0)

func open_vendor(npc: Dictionary) -> bool:
	await click(npc.point, MOUSE_BUTTON_RIGHT)
	if not await wait_for(func(s): return s.open and s.vendor_name == VENDOR, "vendor frame"):
		return false
	var state: Dictionary = client.merchant_state()
	if state.items != VENDOR_ITEMS:
		fail("Vendor items %s, expected %s" % [state.items, VENDOR_ITEMS])
		return false
	await frames(3)
	if not merchant_visible("MerchantFrame"):
		fail("MerchantFrame is not visible")
		return false
	print("FIXTURE OPEN ", state.vendor_name, " items ", state.items, " money ", state.money)
	return true

func wait_for(predicate: Callable, what: String, timeout_ms := REPLY_WAIT_MS) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call(client.merchant_state()):
			await frames(2)
			return true
	fail("Timed out waiting for %s: %s" % [what, client.merchant_state()])
	return false

func bag_slot(state: Dictionary, item_id: int) -> Dictionary:
	for item in state.bags:
		if item.item_id == item_id:
			return item
	return {}

func bag_count(state: Dictionary, item_id: int) -> int:
	var count := 0
	for item in state.bags:
		if item.item_id == item_id:
			count += item.count
	return count

func merchant_control(name: String) -> Control:
	var ui = client.get_node_or_null("MerchantUI")
	return ui.find_child(name, true, false) if ui != null else null

func merchant_visible(name: String) -> bool:
	var control := merchant_control(name)
	return control != null and control.is_visible_in_tree()

func merchant_text(name: String) -> String:
	var control := merchant_control(name)
	return control.text if control is Label and control.is_visible_in_tree() else ""

func enter_world() -> bool:
	var deadline := Time.get_ticks_msec() + 15000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.reply_received and state.screen == "CharacterSelect" and state.character_count >= 1:
			break
	var ui = client.get_node_or_null("CharacterSelectUI")
	if ui == null:
		fail("No character select: " + str(client.account_state()))
		return false
	var card := roster_card(ui)
	if card == null:
		fail("Roster has no " + CHARACTER)
		return false
	await click_control(card, MOUSE_BUTTON_LEFT)
	await click_control(ui.find_child("EnterWorld", true, false), MOUSE_BUTTON_LEFT)
	deadline = Time.get_ticks_msec() + WORLD_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "InWorld" and state.local_player_position != null:
			print("FIXTURE IN_WORLD ", state.selected_character_name, " at ", state.local_player_position)
			return true
	fail("Timed out entering the world: " + str(client.account_state()))
	return false

func roster_card(ui: Node) -> Control:
	var index := 0
	while true:
		var card = ui.find_child("CharCard_%d" % index, true, false)
		if not card is Control:
			return null
		for label in card.find_children("*", "Label", true, false):
			if label.text == CHARACTER:
				return card
		index += 1
	return null

func camera() -> Camera3D:
	return root.get_viewport().get_camera_3d()

# The vendor's pick shape centre on screen, selected by the native ray; turn until seen.
func find_vendor() -> Dictionary:
	var deadline := Time.get_ticks_msec() + NPC_WAIT_MS
	var turned := 0
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var units := client.get_node_or_null("WorldUnits")
		if units == null or camera() == null:
			continue
		for unit in units.get_children():
			if str(unit.name) != VENDOR:
				continue
			var area := unit.find_child("UnitPick", true, false) as Area3D
			if area == null:
				continue
			var world_point := (area.get_child(0) as Node3D).global_position
			if not camera().is_position_in_frustum(world_point):
				continue
			var point := camera().unproject_position(world_point)
			var id = area.get_meta("unit_server_id")
			if UnitPicker.pick(camera(), point) == id:
				return {"id": id, "name": VENDOR, "point": point}
		# Turn in place (TurnRight) until the vendor is in view.
		if turned < 60:
			push_key(KEY_RIGHT, true)
			await frames(3)
			push_key(KEY_RIGHT, false)
			turned += 1
	fail("%s is not visible and unoccluded" % VENDOR)
	return {}

func frames(count: int) -> void:
	for frame in range(count):
		await process_frame

func click(point: Vector2, button: MouseButton, shift := false) -> void:
	await hover_point(point)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = button
		event.button_mask = (1 << (button - 1)) if pressed else 0
		event.shift_pressed = shift
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await frames(3)

func hover_point(point: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await process_frame

func click_control(control: Control, button: MouseButton, shift := false) -> void:
	if control == null:
		fail("Missing control to click")
		return
	await click(control.get_global_rect().get_center(), button, shift)

func capture(file: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var error := image.save_png(SHOTS + file)
	if error != OK:
		fail("Could not save " + file + ": " + str(error))

func tap(code: Key) -> void:
	push_key(code, true)
	await process_frame
	push_key(code, false)
	await frames(2)

func push_key(code: Key, pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.pressed = pressed
	root.push_input(event, true)

func fail(message: String) -> void:
	push_error(message)
	quit(1)
