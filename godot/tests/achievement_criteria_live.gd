extends SceneTree

# Actual private UDP5898 purchases, item uses, criteria pages and online comparison.
# Admin setup may set level/gold/position; never achievement counters or known spells.
var client: Node
var evidence: String
var skin: String
var character: String

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	evidence = OS.get_environment("ACHIEVEMENTS_RUN")
	skin = OS.get_environment("ACHIEVEMENTS_SKIN")
	character = OS.get_environment("ACHIEVEMENTS_CHARACTER")
	var account := OS.get_environment("ACHIEVEMENTS_ACCOUNT")
	if OS.get_environment("GODOT_TEST_SERVER") != "127.0.0.1:5898" or not account.begins_with("fb_achcrit_"):
		fail("Only private5898 and criteria fixture accounts are allowed")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	# Cold catalog loading blocks the main thread; finish it before starting the handshake.
	if not await wait_for(func(): return client.merchant_state().item_catalog_loaded, "startup item catalog", 90000):
		return
	var error = client.connect_account("127.0.0.1:5898", account, "fbtest", false)
	if error != "":
		fail("Login: " + error)
		return
	if not await wait_for(func(): return client.account_state().screen == "CharacterSelect" and client.get_node_or_null("CharacterSelectUI") != null, "character select frame", 30000):
		return
	var ui = client.get_node("CharacterSelectUI")
	await click_control(ui.find_child("CharCard_0", true, false))
	await click_control(ui.find_child("EnterWorld", true, false))
	if not await wait_for(func(): return client.account_state().screen == "InWorld" and client.account_state().local_player_position != null, "world entry", 120000):
		return
	if client.account_state().ui_skin != skin:
		fail("Expected skin %s, got %s" % [skin, client.account_state().ui_skin])
		return
	print("CRITERIA_WORLD ", character, " ", client.account_state())
	await key(KEY_Y)
	# Initial authoritative criteria snapshots precede catalog replies on the ordered channel.
	if not await wait_for(func(): return label("AchievementSummaryTotalText") != "" and find_control("AchievementCategory92") != null, "initial achievement authority and categories", 150000):
		return
	if not await show_achievement(2141, "Stable Keeper", false):
		return
	var initial_mounts := mount_counter()
	print("CRITERIA_MOUNT_BASELINE ", initial_mounts)
	if not await show_achievement(2076, "Armored Brown Bear", false):
		return
	await capture("before-bear")
	await close_achievement()
	if not await buy("Harold Winston", "Band of the Kirin Tor", 40586):
		return
	await key(KEY_ESCAPE)
	await key(KEY_Y)
	if not await show_achievement(2084, "Ring of the Kirin Tor", true):
		return
	await capture("earned-ring")
	await close_achievement()
	if not await buy("Mei Francis", "Reins of the Armored Brown Bear", 44225):
		return
	await use_bag_item(44225)
	await key(KEY_ESCAPE)
	await key(KEY_Y)
	if not await show_achievement(2076, "Armored Brown Bear", true):
		return
	await capture("earned-bear")
	if not await show_achievement(2141, "Stable Keeper", false):
		return
	if mount_counter() != initial_mounts + 1:
		fail("Known mount criteria did not increase by one: " + str(criteria_labels(2141)))
		return
	await capture("mount-progress-one")
	await close_achievement()
	if not await buy("Mei Francis", "Reins of the Traveler's Tundra Mammoth", 44235):
		return
	await use_bag_item(44235)
	await key(KEY_ESCAPE)
	await key(KEY_Y)
	if not await show_achievement(2078, "Traveler's Tundra Mammoth", true):
		return
	await capture("earned-mammoth")
	if not await show_achievement(2141, "Stable Keeper", false):
		return
	if mount_counter() != initial_mounts + 2:
		fail("Known mount criteria did not increase by two: " + str(criteria_labels(2141)))
		return
	await capture("mount-progress-two")
	await close_achievement()
	var peer = JSON.parse_string(FileAccess.get_file_as_string(evidence + "/peer-ready.json"))
	var target := await find_unit(int(peer.player_entity))
	if target.is_empty():
		return
	await click(target.point, MOUSE_BUTTON_LEFT)
	if not await wait_for(func(): return client.target_state().target == int(peer.player_entity), "comparison target"):
		return
	await click_control(find_control("TargetFrame"), MOUSE_BUTTON_RIGHT)
	if not await wait_for(func(): return find_control("UnitFrameContextMenuCompareAchievements") != null, "Retail comparison menu"):
		return
	await click_control(find_control("UnitFrameContextMenuCompareAchievements"))
	if not await wait_for(func(): return label("AchievementComparisonName") != "" and label("AchievementComparisonName") != "Loading...", "comparison reply"):
		return
	await capture("comparison-summary")
	await search("Ring of the Kirin Tor")
	if not await wait_for(func(): return label("Achievement2084ComparisonStatus") == "Incomplete" and label("Achievement2084OwnStatus") == "Completed", "isolated comparison status"):
		return
	await click_control(find_control("Achievement2084Row"))
	await capture("comparison-ring")
	print("CRITERIA_LIVE_PASS ", skin, " earned2084/2076/2078; actual mount count ", initial_mounts, " -> ", initial_mounts + 2, "; online peer isolated")
	client.free()
	quit(0)

func close_achievement() -> void:
	await click_control(find_control("AchievementFrameCloseButton"))
	await wait_for(func(): return client.get_node_or_null("AchievementsUI") == null, "achievement close")

func show_achievement(id: int, title: String, earned: bool) -> bool:
	await search(title)
	var prefix := "Achievement%d" % id
	var expected := "Earned" if earned else "Not earned"
	if not await wait_for(func(): return label(prefix + "Date").begins_with(expected), "authoritative %s %s" % [title, expected], 60000):
		return false
	await click_control(find_control(prefix + "Row"))
	if not await wait_for(func(): return not criteria_labels(id).is_empty(), "criteria expansion", 60000):
		return false
	print("CRITERIA_PAGE ", id, " ", label(prefix + "Date"), " ", criteria_labels(id))
	return true

func mount_counter() -> int:
	var labels := criteria_labels(2141)
	if labels.size() != 1:
		fail("Stable Keeper lacks its authoritative mount count: " + str(labels))
		return -1
	var quantity := str(labels[0]).split("/")
	if quantity.size() != 2 or not quantity[0].is_valid_int() or not quantity[1].is_valid_int():
		fail("Stable Keeper quantity caption is invalid: " + str(labels))
		return -1
	return quantity[0].to_int()

func criteria_labels(id: int) -> Array:
	var result := []
	for node in client.find_children("Achievement%dCriterion*" % id, "Label", true, false):
		if node.is_visible_in_tree():
			result.append(node.text)
	return result

func search(title: String) -> void:
	await click_control(find_control("AchievementSearchBox"))
	var event := InputEventKey.new()
	event.keycode = KEY_A
	event.physical_keycode = KEY_A
	event.ctrl_pressed = true
	event.pressed = true
	root.push_input(event, true)
	await process_frame
	event = event.duplicate()
	event.pressed = false
	root.push_input(event, true)
	await key(KEY_BACKSPACE)
	for i in title.length():
		event = InputEventKey.new()
		event.unicode = title.unicode_at(i)
		event.pressed = true
		root.push_input(event, true)
		await process_frame
	await frames(5)

func buy(vendor: String, title: String, item_id: int) -> bool:
	var npc := await find_unit(0, vendor)
	if npc.is_empty():
		return false
	await click(npc.point, MOUSE_BUTTON_RIGHT)
	print("CRITERIA_VENDOR_CLICK ", npc, " target ", client.target_state(), " merchant ", client.merchant_state())
	if not await wait_for(func(): return client.merchant_state().open and client.merchant_state().vendor_name == vendor, "vendor " + vendor):
		return false
	var cell: Control
	for page in range(12):
		for index in range(1, 13):
			if label("MerchantItem%dName" % index) == title:
				cell = find_control("MerchantItem%d" % index)
				break
		if cell != null:
			break
		var next = find_control("MerchantNextPageButton")
		if next == null or not next.is_visible_in_tree():
			break
		await click_control(next)
		await frames(3)
	if cell == null:
		fail("Vendor %s lacks %s; %s" % [vendor, title, client.merchant_state()])
		return false
	var money = client.merchant_state().money
	await click_control(cell, MOUSE_BUTTON_RIGHT)
	if not await wait_for(func(): return not bag_item(item_id).is_empty() and client.merchant_state().money < money, "paid purchase " + title):
		return false
	print("CRITERIA_PAID_PURCHASE ", item_id, " money ", money, " -> ", client.merchant_state().money)
	return true

func use_bag_item(id: int) -> void:
	# Retail bag right-click sells while a merchant is open; close it before actual use.
	await key(KEY_ESCAPE)
	if not await wait_for(func(): return not client.merchant_state().open, "merchant close before item use"):
		return
	var bag := find_control("ContainerFrame0")
	if bag == null or not bag.is_visible_in_tree():
		await click_control(find_control("MainMenuBarBackpackButton"))
	if not await wait_for(func(): return find_control("ContainerFrame0") != null and find_control("ContainerFrame0").is_visible_in_tree(), "backpack for item use"):
		return
	var item := bag_item(id)
	if item.is_empty():
		fail("Missing purchased bag item " + str(id))
		return
	var before := bag_count(id)
	await click_control(find_control("ContainerFrame%dSlot%d" % [item.bag, item.slot]), MOUSE_BUTTON_RIGHT)
	await wait_for(func(): return bag_count(id) == before - 1, "consumed one mount item")
	await frames(20)

func bag_count(id: int) -> int:
	var count := 0
	for item in client.merchant_state().bags:
		if item.item_id == id:
			count += int(item.count)
	return count

func bag_item(id: int) -> Dictionary:
	for item in client.merchant_state().bags:
		if item.item_id == id:
			return item
	return {}

func find_unit(id: int, unit_name := "") -> Dictionary:
	var deadline := Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		await frames(3)
		var camera := root.get_camera_3d()
		var units = client.get_node_or_null("WorldUnits")
		if camera == null or units == null:
			continue
		for unit in units.get_children():
			var area := unit.find_child("UnitPick", true, false) as Area3D
			if area == null:
				continue
			var server_id = area.get_meta("unit_server_id")
			if (id != 0 and server_id != id) or (id == 0 and str(unit.name) != unit_name):
				continue
			var point3 := (area.get_child(0) as Node3D).global_position
			if not camera.is_position_in_frustum(point3):
				continue
			var point := camera.unproject_position(point3)
			if UnitPicker.pick(camera, point) == server_id:
				return {"id": server_id, "point": point}
		push_key(KEY_RIGHT, true)
		await frames(3)
		push_key(KEY_RIGHT, false)
	fail("Unit not visible/unoccluded: %s %s" % [id, unit_name])
	return {}

func find_control(control_name: String) -> Control:
	# Merchant and standalone bag registries share Retail names; input must hit the visible one.
	for control in client.find_children(control_name, "Control", true, false):
		if control.is_visible_in_tree():
			return control as Control
	return null

func label(control_name: String) -> String:
	var control := find_control(control_name)
	return control.text if control is Label and control.is_visible_in_tree() else ""

func wait_for(predicate: Callable, what: String, timeout := 60000) -> bool:
	var deadline := Time.get_ticks_msec() + timeout
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call():
			return true
	fail("Timeout: " + what)
	return false

func frames(count: int) -> void:
	for frame in range(count):
		await process_frame

func click_control(control: Control, button := MOUSE_BUTTON_LEFT) -> void:
	if control == null:
		fail("Missing physical click control")
		return
	await click(control.get_global_rect().get_center(), button)

func click(point: Vector2, button: MouseButton) -> void:
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
		event.button_mask = (1 << (button - 1)) if pressed else 0
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await frames(3)

func key(code: Key) -> void:
	push_key(code, true)
	await process_frame
	push_key(code, false)
	await frames(3)

func push_key(code: Key, pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.pressed = pressed
	root.push_input(event, true)

func capture(stage: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var path := OS.get_environment("ACHIEVEMENTS_PNGS") + "/" + skin.to_lower() + "-" + stage + ".png"
	var result := image.save_png(path)
	if result != OK:
		fail("Screenshot failed: " + str(result))

func fail(message: String) -> void:
	if client != null:
		print("CRITERIA_FAILURE_STATE account=", client.account_state(), " target=", client.target_state(), " merchant=", client.merchant_state())
		root.get_texture().get_image().save_png(evidence + "/failure-%d.png" % Time.get_ticks_msec())
	push_error("CRITERIA_LIVE_FAIL " + message)
	quit(1)
