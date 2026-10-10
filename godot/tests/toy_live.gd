extends "res://tests/toy_objects_live.gd"
# Real Toy Box pointer input; isolated private realm, no injected casts or visuals.
const SAMPLES := [
	[45011, 62736, "Stormwind Banner"],
	[33223, 42766, "Fishing Chair"],
	[40768, 54710, "MOLL-E"],
	[156871, 261981, "Spitzy"],
	[165791, 286277, "Worn Cloak"],
	[221964, 455023, "Filmless Camera"],
	[228413, 462145, "Lampyridae Lure"],
	[263198, 1269949, "Valdekar's Special"],
	[267456, 1280563, "Lil' Scoots' Pillow"],
	[88580, 128328, "Ken-Ken's Mask"],
	[204818, 406870, "Mallard Mortar"],
	[45057, 62949, "Wind-Up Train Wrecker"],
	[116139, 170950, "Haunting Memento"],
	[54452, 75136, "Ethereal Portal"],
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
		var nodes := client.find_children(character, "Node3D", true, false)
		if nodes.is_empty(): return false
		actor = nodes[0]
		return actor.find_children("PlayerModel", "Node3D", true, false).size() > 0, "native player visual", 120000): return
	await create_timer(2.0).timeout
	for sample in SAMPLES:
		var selected := OS.get_environment("TOYFX_ITEM")
		if selected.is_empty() or int(selected) == int(sample[0]):
			await sample_toy(sample)
	print("PASS: completed real Toy Box sampling (individual render verdicts in receipts) ", skin)
	client.free()
	quit(0)

func sample_toy(sample: Array) -> void:
	var item := int(sample[0])
	var spell := int(sample[1])
	await learn_toy(item)
	if not learned_toy(item): return
	await click("CollectionsMicroButton")
	await click("CollectionsJournalTab3")
	if not await wait_until(func(): return control("ToyBoxSearchBox") != null, "Toy Box"): return
	await click("ToyBoxSearchBox")
	var edit := control("ToyBoxSearchBox") as LineEdit
	edit.select_all()
	await type_text(str(sample[2]))
	if not await wait_until(func(): return control("ToySpellButton1Name") != null and control("ToySpellButton1Name").text == str(sample[2]), "search " + str(item)): return
	var button := control("ToySpellButton1")
	var before: Dictionary = client.spell_visuals_state()
	var start_ms := Time.get_ticks_msec()
	var result := {"skin": skin, "item": item, "spell": spell, "disabled": button.get("disabled"), "before": before, "frames": []}
	if item == 88580 and control("PlayerFrame") != null:
		await click("PlayerFrame")
	if item == 204818:
		await tap(KEY_TAB)
	await click("ToySpellButton1")
	await tap(KEY_ESCAPE)
	# Sample short-lived cast art and held attachments separately; preserve actual state.
	var frame_count := 13 if item == 54452 else 7
	for frame in range(frame_count):
		await create_timer(0.4 if frame < 3 else 1.0).timeout
		if frame == 4 and item in [45011, 33223, 40768]:
			var pos := actor.position
			if not command(OS.get_environment("TOYFX_ADMIN"), ["set-position", character, str(pos.x + 3.0), str(-pos.z), str(pos.y)]): return
		await RenderingServer.frame_post_draw
		var image_path := OS.get_environment("TOYFX_SHOTS").path_join("%s-toy%d-%d.png" % [skin, item, frame])
		if root.get_texture().get_image().save_png(image_path) != OK: fail("capture " + image_path); return
		var state: Dictionary = client.spell_visuals_state()
		var objects: Array = []
		for node in client.find_children("*", "Node3D", true, false):
			if node.has_meta("game_object_entry"):
				objects.append({"entry": node.get_meta("game_object_entry"), "display": node.get_meta("game_object_display_id"), "position": str(node.position), "meshes": node.find_children("*", "MeshInstance3D", true, false).size()})
		result.frames.append({"ms": Time.get_ticks_msec() - start_ms, "visuals": state, "auras": client.aura_state(), "position": str(actor.position), "objects": objects, "image": image_path})
	result["after"] = client.toybox_state()
	receipts.append(result)
	write_receipts()
	print("TOYLIVE_SAMPLE ", skin, " item=", item, " spell=", spell, " kits=", client.spell_visuals_state().started)
	# Ordinary player right-click cancellation prevents one toy contaminating the next.
	if has_aura(spell): await cancel_aura(spell)
	await create_timer(1.0).timeout
