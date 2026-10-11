extends "res://tests/toy_objects_live.gd"
# Real bag learning and Toy Box input on an isolated realm. No injected visuals.
var failed := false
const SAMPLES := [
	[156871, 261981, "Spitzy"],
	[45057, 62949, "Wind-Up Train Wrecker"],
	[194056, 383065, "Duck-Stuffed Duck Lovie"],
	[147838, 239560, "Akazamzarak's Spare Hat"],
	[153183, 254513, "Barrier Generator"],
	[204675, 405634, "A Drake's Big Basket of Eggs"],
	[88381, 127771, "Silversage Incense"],
	[88584, 128180, "Totem of Harmony"],
	[165791, 286277, "Worn Cloak"],
	[221964, 455023, "Filmless Camera"],
	[228413, 462145, "Lampyridae Lure"],
	[263198, 1269949, "Valdekar's Special"],
	[267456, 1280563, "Lil' Scoots' Pillow"],
	[88580, 128328, "Ken-Ken's Mask"],
	[204818, 406870, "Mallard Mortar"],
]

func run_fixture() -> void:
	root.size = Vector2i(1920, 1080)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_until(func(): return not client.account_state().assets_starting, "assets", 180000): return
	var error: String = client.connect_account(OS.get_environment("GODOT_TEST_SERVER"), OS.get_environment("TOYFX_ACCOUNT"), "fbtest", false)
	if error != "": fail(error); return
	if not await wait_until(func(): return control("CharCard_0") != null, "roster", 120000): return
	await click("CharCard_0")
	await click("EnterWorld")
	if not await wait_until(func(): return client.account_state().screen == "InWorld" and control("CollectionsMicroButton") != null, "world", 180000): return
	if not await wait_until(func():
		for node in client.find_children(character, "Node3D", true, false):
			if node.get_node_or_null("PlayerModel") != null:
				actor = node
				return true
		return false, "player model", 120000): return
	await create_timer(2.0).timeout
	await click("CollectionsMicroButton")
	await click("CollectionsJournalTab3")
	if not await wait_until(func(): return client.toybox_state().catalog_count > 0, "initial journal query"): return
	await click("CollectionsMicroButton")
	for zoom in range(5):
		var event := InputEventMouseButton.new()
		event.position = Vector2(960, 540)
		event.button_index = MOUSE_BUTTON_WHEEL_UP
		event.pressed = true
		root.push_input(event, true)
		await process_frame
	for sample in SAMPLES:
		var requested := OS.get_environment("TOYFX_ITEM")
		if (requested == "mortar" and int(sample[0]) != 204818) or (requested == "all" and int(sample[0]) == 204818) or (requested == "remaining" and int(sample[0]) not in [165791, 221964, 228413, 263198, 267456, 88580]): continue
		await sample_toy(sample)
		if failed: return
	write_receipts()
	print("PASS: completed real Toy Box sampling; per-toy render verdicts require receipts/PNG inspection ", skin)
	client.free()
	quit(0)

func sample_toy(sample: Array) -> void:
	var item := int(sample[0])
	var spell := int(sample[1])
	await learn_sample(item)
	if failed or not learned_toy(item): return
	# Learning uses Escape; select the mortar target only AFTER learning.
	if item == 204818:
		if not await select_mortar_target(): return
	if item == 88580:
		await click("PlayerFrame")
	await click("CollectionsMicroButton")
	await click("CollectionsJournalTab3")
	if not await wait_until(func(): return control("ToyBoxSearchBox") != null, "Toy Box"): return
	await click("ToyBoxSearchBox")
	var edit := control("ToyBoxSearchBox") as LineEdit
	edit.select_all()
	await type_text(str(sample[2]))
	if not await wait_until(func(): return control("ToySpellButton1Name") != null and control("ToySpellButton1Name").text == str(sample[2]), "search " + str(item)): return
	var result := {"skin": skin, "item": item, "spell": spell, "account": client.account_state(), "before": client.spell_visuals_state(), "target_before": client.target_state(), "disabled": control("ToySpellButton1").get("disabled"), "frames": []}
	var start_ms := Time.get_ticks_msec()
	await click("ToySpellButton1")
	# Closing Collections via its toggle preserves unit selection during casts.
	await click("CollectionsMicroButton")
	for frame in range(7):
		await create_timer(0.1 if frame < 3 else 1.0).timeout
		await RenderingServer.frame_post_draw
		var path := OS.get_environment("TOYFX_SHOTS").path_join("%s-toy%d-%d.png" % [skin, item, frame])
		if root.get_texture().get_image().save_png(path) != OK: fail("capture " + path); return
		var summons: Array = []
		for node in client.find_children("*", "Node3D", true, false):
			if node.get_node_or_null("NpcVisualRoot") != null:
				summons.append({"name": str(node.name), "position": str(node.position), "meshes": node.find_children("*", "MeshInstance3D", true, false).size()})
		result.frames.append({"ms": Time.get_ticks_msec() - start_ms, "visuals": client.spell_visuals_state(), "auras": client.aura_state(), "position": str(actor.position), "target": client.target_state(), "summons": summons, "image": path})
	result["after"] = client.toybox_state()
	receipts.append(result)
	write_receipts()
	print("TOYLIVE2_SAMPLE ", skin, " item=", item, " spell=", spell)
	if has_aura(spell): await cancel_aura(spell)
	if item == 153183: await tap(KEY_W)
	await create_timer(1.0).timeout

func learn_sample(item: int) -> void:
	if learned_toy(item): return
	await click("MainMenuBarBackpackButton")
	if not await wait_until(func(): return control("ContainerFrame0") != null, "backpack"): return
	var slot_name := ""
	for bag_item in client.merchant_state().bags:
		if int(bag_item.item_id) == item: slot_name = "ContainerFrame%dSlot%d" % [bag_item.bag, bag_item.slot]
	if slot_name != "": await click(slot_name, MOUSE_BUTTON_RIGHT)
	await tap(KEY_ESCAPE)
	# Read the authoritative journal through its normal opening/query action.
	await click("CollectionsMicroButton")
	await click("CollectionsJournalTab3")
	var deadline := Time.get_ticks_msec() + 10000
	while not learned_toy(item) and Time.get_ticks_msec() < deadline: await process_frame
	if not learned_toy(item):
		await RenderingServer.frame_post_draw
		var path := OS.get_environment("TOYFX_SHOTS").path_join("%s-toy%d-learning.png" % [skin, item])
		if root.get_texture().get_image().save_png(path) != OK: fail("capture " + path); return
		receipts.append({"item": item, "skin": skin, "phase": "learning-refused", "slot": slot_name, "inventory": client.merchant_state(), "toybox": client.toybox_state(), "image": path})
		write_receipts()
	await click("CollectionsMicroButton")

func select_mortar_target() -> bool:
	# TDB Northshire spawn:42937 (-9029.03,-274.965,74.4195).
	if not command(OS.get_environment("TOYFX_ADMIN"), ["set-position", character, "-9029.03", "-254.965", "74.4195"]): return false
	var destination := Vector3(-9029.03, 74.4195, 254.965)
	if not await wait_until(func(): return actor.position.distance_to(destination) < 4.0, "Northshire20yd positioning"): return false
	await create_timer(3.0).timeout
	for attempt in range(40):
		await tap(KEY_TAB)
		var selected = client.target_state().target
		if selected == null: continue
		var rules: Dictionary = client.nameplate_rules(int(selected))
		var distance := float(rules.get("distance", 0.0))
		if str(rules.get("reaction", "")) != "Hostile" or not bool(rules.get("alive", false)) or distance < 8.0 or distance > 30.0: continue
		var target_id := int(selected)
		if not await wait_until(func(): return client.target_state().server_target == target_id, "authoritative target"): return false
		receipts.append({"phase": "mortar-target", "rules": rules, "target": client.target_state(), "position": str(actor.position)})
		return true
	fail("no valid living hostile in Northshire Tab cycle")
	return false

func fail(message: String) -> void:
	failed = true
	push_error("toy-live2: " + message)
	write_receipts()
	quit(1)
