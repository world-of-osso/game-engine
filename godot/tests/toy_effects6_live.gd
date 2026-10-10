extends "res://tests/toy_objects_live.gd"
# Actual Toy Box input, owned private UDP55396, source-bound observations.
var failed := false

func fail(message: String) -> void:
	failed = true
	push_error("toy-effects6: " + message)
	write_receipts()
	quit(1)

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
	await capture_phase("baseline")
	await use_toy(156871, "Spitzy")
	if failed: return
	if not await wait_until(func(): return has_aura(261981) and kit_active(261981, 94072), "Spitzy source kit94072", 45000): return
	await capture_phase("spitzy")
	await cancel_aura(261981)
	if not await wait_until(func(): return not has_aura(261981), "Spitzy cancellation"): return
	await use_toy(45057, "Wind-Up Train Wrecker")
	if failed: return
	if not await wait_until(func(): return train_model() != null, "train wrecker source model123251", 45000): return
	await capture_phase("train-wrecker")
	await use_toy(194056, "Duck-Stuffed Duck Lovie")
	if failed: return
	if not await wait_until(func(): return has_aura(383065) and kit_active(383065, 162763), "Duck Lovie authored nap", 45000): return
	await capture_phase("duck-lovie")
	await cancel_aura(383065)
	if not await wait_until(func(): return not has_aura(383065), "Duck Lovie cancellation"): return
	await tap(KEY_TAB)
	if not await wait_until(func(): return client.target_state().target != null, "hostile Tab target"): return
	var target_name := str(client.target_state().target_name)
	var target_node: Node3D
	for node in client.find_children(target_name, "Node3D", true, false):
		if node.get_node_or_null("NpcModel") != null or node.get_node_or_null("NpcVisualRoot") != null:
			target_node = node
			break
	if target_node == null:
		for node in client.find_children("*", "Node3D", true, false):
			if str(node.name) == target_name: target_node = node; break
	if target_node == null: fail("target node missing: " + target_name); return
	var direction := actor.position - target_node.position
	direction.y = 0.0
	if direction.length() == 0.0: fail("zero direction to target"); return
	var destination := target_node.position + direction.normalized() * 20.0
	destination.y = actor.position.y
	if not command(OS.get_environment("TOYFX_ADMIN"), ["set-position", character, str(destination.x), str(-destination.z), str(destination.y)]): return
	if not await wait_until(func(): return abs(actor.position.distance_to(target_node.position) - 20.0) < 1.0, "valid mortar20yd"): return
	receipts.append({"phase": "mortar-range", "distance": actor.position.distance_to(target_node.position), "target": client.target_state()})
	await use_toy(204818, "Mallard Mortar", false)
	if failed: return
	for step in range(30):
		await capture_phase("mortar-%02d" % step)
		await create_timer(0.1).timeout
	if not has_go(406870): fail("valid-range mortar did not receive SpellGo"); return
	write_receipts()
	if failed: return
	print("PASS: toy-effects6 actual Toy Box Spitzy/train/nap/mortar ", skin)
	client.free()
	quit(0)

func use_toy(item: int, title: String, settle := true) -> void:
	await learn_toy(item)
	if failed or not learned_toy(item): return
	await click("CollectionsMicroButton")
	if not await wait_until(func(): return control("CollectionsJournalTab3") != null, "collection tabs"): return
	await click("CollectionsJournalTab3")
	await click("ToyBoxSearchBox")
	var edit := control("ToyBoxSearchBox") as LineEdit
	if edit == null: fail("missing toy search"); return
	edit.select_all()
	await type_text(title)
	if not await wait_until(func(): return control("ToySpellButton1Name") != null and control("ToySpellButton1Name").text == title, "search " + title): return
	await click("ToySpellButton1")
	await tap(KEY_ESCAPE)
	if settle: await create_timer(0.7).timeout

func kit_active(spell: int, kit: int) -> bool:
	for model in client.spell_visuals_state().active:
		if int(model.spell) == spell and int(model.kit) == kit: return true
	return false

func has_go(spell: int) -> bool:
	for cast in client.spell_visuals_state().casts:
		if int(cast.spell) == spell and bool(cast.go): return true
	return false

func train_model() -> Node3D:
	for node in client.find_children("NpcModel", "Node3D", true, false):
		if node.has_meta("model_file_data_id") and int(node.get_meta("model_file_data_id")) == 123251:
			if not node.find_children("*", "MeshInstance3D", true, false).is_empty(): return node
	return null

func capture_phase(label: String) -> void:
	await RenderingServer.frame_post_draw
	var path := OS.get_environment("TOYFX_SHOTS").path_join("%s-%s.png" % [skin, label])
	if root.get_texture().get_image().save_png(path) != OK: fail("capture " + path); return
	receipts.append({"phase": label, "skin": skin, "screenshot": path, "auras": client.aura_state(), "visuals": client.spell_visuals_state(), "target": client.target_state(), "actor_position": [actor.position.x, actor.position.y, actor.position.z]})
	write_receipts()
