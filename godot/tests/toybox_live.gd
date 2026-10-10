extends SceneTree
# Private master server only. Native input; observations never mutate gameplay state.
const ITEM := 32782
var client: Node
var shots := OS.get_environment("TOYBOX_SHOTS")
var skin := OS.get_environment("TOYBOX_SKIN")
var relog := OS.get_environment("TOYBOX_RELOG") == "1"

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_fixture")

func run_fixture() -> void:
	root.size = Vector2i(1920, 1080)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_until(func(): return not client.account_state().assets_starting, "asset startup", 180000): return
	var error: String = client.connect_account(OS.get_environment("GODOT_TEST_SERVER"), OS.get_environment("TOYBOX_ACCOUNT"), "fbtest", false)
	if error != "": fail(error); return
	if not await wait_until(func(): return client.get_node_or_null("CharacterSelectUI") != null, "character select", 120000): return
	await click("CharCard_0")
	await click("EnterWorld")
	if not await wait_until(func(): return client.account_state().screen == "InWorld" and int(client.toybox_state().catalog_count) > 0 and control("CollectionsMicroButton") != null, "world and catalog", 180000): return
	await wait_frames(120)
	await click("CollectionsMicroButton")
	if not await wait_until(func(): return control("ToySpellButton18") != null, "eighteen catalog tiles"): return
	for node in client.find_children("*", "Label", true, false):
		if node.is_visible_in_tree() and node.text == OS.get_environment("TOYBOX_CHARACTER"):
			print("CHARACTER_LABEL ", node.get_path(), " ", node.get_global_rect())
	await capture("catalog-relog" if relog else "catalog")
	await click("ToyBoxSearchBox")
	await type_text("Time-Lost Figurine")
	if not await wait_until(func(): return control("ToySpellButton1Name") != null and control("ToySpellButton1Name").text == "Time-Lost Figurine", "search result"): return
	if relog:
		if learned().is_empty() or not learned().favourite or int(client.toybox_state().slots.get(8, 0)) != ITEM:
			fail("favourite/action slot did not survive relog: " + str(client.toybox_state())); return
		var final_relog := OS.get_environment("TOYBOX_FINAL_RELOG") == "1"
		await capture("final-relog-favourite-slot" if final_relog else "relog-favourite-slot")
		if not final_relog:
			await click("ActionButton9")
			if not await wait_until(func(): return float(learned().cooldown) > 0.0 and transformed(), "bar cast and authoritative cooldown"): return
			await capture("bar-used-cooldown")
		print("PASS: Toy Box relog favourite and slot ", client.toybox_state(), " auras=", client.aura_state())
		client.free(); quit(0); return
	await capture("uncollected")
	await click("CollectionsMicroButton")
	await click("MainMenuBarBackpackButton")
	if not await wait_until(func(): return control("ContainerFrame0") != null, "backpack"): return
	var bag_name := ""
	for entry in client.merchant_state().bags:
		if int(entry.item_id) == ITEM: bag_name = "ContainerFrame%dSlot%d" % [entry.bag, entry.slot]
	if bag_name == "": fail("supported physical toy missing from bag"); return
	await click(bag_name, MOUSE_BUTTON_RIGHT)
	if not await wait_until(func(): return not learned().is_empty(), "UseItem learned toy"): return
	for entry in client.merchant_state().bags:
		if int(entry.item_id) == ITEM: fail("learning did not consume toy"); return
	await tap(KEY_ESCAPE)
	await click("CollectionsMicroButton")
	if not await wait_until(func(): return control("ToySpellButton1Name") != null, "newly learned page"): return
	await capture("learned-new")
	await click("ToySpellButton1", MOUSE_BUTTON_RIGHT)
	if not await wait_until(func(): return control("ToyBoxFavouriteToggle") != null, "Retail favourite context menu"): return
	await capture("favourite-menu")
	await click("ToyBoxFavouriteToggle")
	if not await wait_until(func(): return learned().favourite, "account favourite update"): return
	await click("ToyBoxSearchClear")
	if not await wait_until(func(): return control("ToySpellButton18") != null and control("ToySpellButton1Name").text == "Time-Lost Figurine", "favourite sorts ahead of full catalog"): return
	await capture("favourite-sorted")
	await click("ToySpellButton1")
	if not await wait_until(func(): return float(learned().cooldown) > 0.0 and transformed(), "journal cast and authoritative toy cooldown"): return
	await capture("used-cooldown")
	await drag_to("ToySpellButton1", "ActionButton9")
	if not await wait_until(func(): return int(client.toybox_state().slots.get(8, 0)) == ITEM, "toy action assignment"): return
	await capture("dragged-action-slot")
	await click("ActionButton9")
	if not await wait_until(func(): return client.toybox_state().error != "", "same UseToy bar path cooldown refusal"): return
	await capture("bar-cooldown-error")
	print("PASS: Toy Box learned/use/favourite/drag/bar-refusal ", client.toybox_state(), " auras=", client.aura_state())
	client.free()
	quit(0)

func transformed() -> bool:
	for aura in client.aura_state().buffs:
		if int(aura.spell_id) == 41301: return true
	return false

func learned() -> Dictionary:
	for toy in client.toybox_state().learned:
		if int(toy.item_id) == ITEM: return toy
	return {}

func control(name: String) -> Control:
	for node in client.find_children(name, "Control", true, false):
		if node.is_visible_in_tree(): return node
	return null

func wait_until(predicate: Callable, label: String, timeout: int = 30000) -> bool:
	var deadline := Time.get_ticks_msec() + timeout
	while Time.get_ticks_msec() < deadline:
		if predicate.call(): return true
		await process_frame
	fail("timeout: " + label + " account=" + str(client.account_state()) + " toys=" + str(client.toybox_state()))
	return false

func click(name: String, button: int = MOUSE_BUTTON_LEFT) -> void:
	var node := control(name)
	if node == null: fail("missing control " + name); return
	var point := node.get_global_rect().get_center()
	await motion(point, false)
	await button_event(point, button, true)
	await button_event(point, button, false)
	await wait_frames(5)

func motion(point: Vector2, down: bool) -> void:
	var event := InputEventMouseMotion.new()
	event.position = point
	event.global_position = point
	event.button_mask = MOUSE_BUTTON_MASK_LEFT if down else 0
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

func drag_to(source: String, destination: String) -> void:
	var start := control(source).get_global_rect().get_center()
	var end := control(destination).get_global_rect().get_center()
	await motion(start, false)
	await button_event(start, MOUSE_BUTTON_LEFT, true)
	for step in range(1, 9): await motion(start.lerp(end, float(step) / 8.0), true)
	await button_event(end, MOUSE_BUTTON_LEFT, false)
	await wait_frames(10)

func type_text(value: String) -> void:
	for character in value:
		var event := InputEventKey.new()
		event.unicode = character.unicode_at(0)
		event.keycode = OS.find_keycode_from_string(character.to_upper())
		event.physical_keycode = event.keycode
		event.pressed = true
		root.push_input(event, true)
		await process_frame
		event.pressed = false
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
	var path := shots.path_join("v2-" + skin + "-" + label)
	root.get_texture().get_image().save_png(path + ".png")
	var receipt := OS.get_environment("TOYBOX_RECEIPTS").path_join("v2-" + skin + "-" + label + ".json")
	var file := FileAccess.open(receipt, FileAccess.WRITE)
	file.store_string(JSON.stringify({"toys": client.toybox_state(), "account": client.account_state(), "auras": client.aura_state()}, "\t"))
	file.close()
	print("TOYBOX_CAPTURE ", path)

func wait_frames(count: int) -> void:
	for frame in range(count): await process_frame

func fail(message: String) -> void:
	push_error("Toy Box live fixture: " + message)
	quit(1)
