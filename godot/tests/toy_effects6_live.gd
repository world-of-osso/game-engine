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
	if OS.get_environment("TOYFX_ONLY_MORTAR") != "1":
		await prove_attachments()
		if failed: return
	# Learning closes bags with Escape, which also clears unit selection. Learn first.
	await learn_toy(204818)
	if failed or not learned_toy(204818): return
	if not await move_near_authored_hostile(): return
	var hostile := false
	for attempt in range(40):
		await tap(KEY_TAB)
		var selected = client.target_state().target
		if selected != null:
			var rules: Dictionary = client.nameplate_rules(int(selected))
			receipts.append({"phase": "target-selection", "target": client.target_state(), "rules": rules})
			if str(rules.get("reaction", "")) == "Hostile" and bool(rules.get("alive", false)):
				hostile = true
				break
	if not hostile: fail("no living hostile target in Tab cycle"); return
	var target_id := int(client.target_state().target)
	if not await wait_until(func(): return client.target_state().server_target == target_id, "authoritative hostile target"): return
	var target_node: Node3D
	for area in client.find_children("*", "Area3D", true, false):
		if area.has_meta("unit_server_id") and int(area.get_meta("unit_server_id")) == target_id:
			target_node = area.get_parent() as Node3D
			break
	if target_node == null: fail("selected hostile has no authored model"); return
	var direction := actor.global_position - target_node.global_position
	direction.y = 0.0
	if direction.length() == 0.0: fail("zero direction to target"); return
	var destination := target_node.global_position + direction.normalized() * 20.0
	destination.y = actor.position.y
	if not command(OS.get_environment("TOYFX_ADMIN"), ["set-position", character, str(destination.x), str(-destination.z), str(destination.y)]): return
	if not await wait_until(func(): return abs(actor.global_position.distance_to(target_node.global_position) - 20.0) < 1.0, "valid mortar20yd"): return
	receipts.append({"phase": "mortar-range", "distance": actor.global_position.distance_to(target_node.global_position), "target": client.target_state(), "rules": client.nameplate_rules(target_id)})
	await use_toy(204818, "Mallard Mortar", false)
	if failed: return
	for step in range(30):
		await capture_phase("mortar-%02d" % step)
		await create_timer(0.1).timeout
	if not has_go(406870): fail("valid-range mortar did not receive SpellGo"); return
	write_receipts()
	if failed: return
	print("PASS: toy-effects6 actual Toy Box ", "mortar" if OS.get_environment("TOYFX_ONLY_MORTAR") == "1" else "Spitzy/train/nap/mortar", " ", skin)
	client.free()
	quit(0)

func move_near_authored_hostile() -> bool:
	var closest: Node3D
	for node in client.find_children("Blackrock Invader", "Node3D", true, false):
		if node.get_node_or_null("NpcVisualRoot") == null: continue
		if closest == null or actor.position.distance_to(node.position) < actor.position.distance_to(closest.position): closest = node
	if closest == null: fail("no source-authored Blackrock Invader42937 model"); return false
	var camera := root.get_camera_3d()
	if camera == null: fail("no world camera"); return false
	var forward := -camera.global_basis.z
	forward.y = 0.0
	var destination := closest.position - forward.normalized() * 20.0
	destination.y = closest.position.y
	if not command(OS.get_environment("TOYFX_ADMIN"), ["set-position", character, str(destination.x), str(-destination.z), str(destination.y)]): return false
	if not await wait_until(func(): return actor.position.distance_to(destination) < 3.0, "private20yd hostile positioning"): return false
	await create_timer(1.0).timeout
	return true

func prove_attachments() -> void:
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
	for unit in client.find_children("Wind-Up Train Wrecker", "Node3D", true, false):
		for node in unit.find_children("NpcModel", "Node3D", true, false):
			if not node.find_children("*", "MeshInstance3D", true, false).is_empty(): return node
	return null

func capture_phase(label: String) -> void:
	await RenderingServer.frame_post_draw
	var path := OS.get_environment("TOYFX_SHOTS").path_join("%s-%s.png" % [skin, label])
	if root.get_texture().get_image().save_png(path) != OK: fail("capture " + path); return
	receipts.append({"phase": label, "skin": skin, "screenshot": path, "auras": client.aura_state(), "visuals": client.spell_visuals_state(), "target": client.target_state(), "toybox": client.toybox_state(), "actor_position": [actor.position.x, actor.position.y, actor.position.z]})
	write_receipts()
