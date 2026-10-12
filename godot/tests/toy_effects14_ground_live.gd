extends "res://tests/toy_effects14_live.gd"
# Native UI cursor cancellation and real world click; no injected cast or actor.
func proof_cases() -> Array:
	return [[89869, 131171, 45436, "Pandaren Scarecrow"]]

func prove_toy(sample: Array) -> void:
	await learn_toy(int(sample[0]))
	if failed: return
	for unit in client.find_children(character, "Node3D", true, false):
		if unit.get_node_or_null("PlayerModel") != null:
			actor = unit
			break
	if actor == null: fail("native actor missing"); return
	await start_cursor(str(sample[3]))
	if failed: return
	await tap(KEY_ESCAPE)
	if client.find_child("GroundSpellReticle", true, false) != null: fail("Escape retained toy reticle"); return
	if scarecrow() != null: fail("cursor cancellation cast the toy"); return
	await start_cursor(str(sample[3]))
	if failed: return
	var camera := root.get_camera_3d()
	if camera == null: fail("world camera missing"); return
	var forward := -camera.global_basis.z
	forward.y = 0.0
	var desired := actor.global_position + forward.normalized() * 3.0
	var point := camera.unproject_position(desired)
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await create_timer(0.5).timeout
	var reticle := client.find_child("GroundSpellReticle", true, false) as Node3D
	if reticle == null or not reticle.visible: fail("no source ground under pointer"); return
	var clicked := reticle.global_position - Vector3.UP * 0.06
	await capture("ground-click-reticle")
	for down in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = down
		root.push_input(event, true)
		await process_frame
	if not await wait_until(func(): return scarecrow() != null, "source scarecrow at clicked ground", 120000): return
	var placed := scarecrow()
	if placed.global_position.distance_to(clicked) > 0.2: fail("toy ignored clicked ground"); return
	if placed.global_position.distance_to(actor.global_position) < 1.0: fail("toy cast at player's feet"); return
	if client.find_child("GroundSpellReticle", true, false) != null: fail("reticle survived completed click"); return
	receipts.append({"phase": "ground-placement", "clicked": [clicked.x, clicked.y, clicked.z], "placed": [placed.global_position.x, placed.global_position.y, placed.global_position.z], "meshes": placed.find_children("*", "MeshInstance3D", true, false).size()})
	await capture("toy89869-source45436-clicked")

func start_cursor(title: String) -> void:
	await click("CollectionsMicroButton")
	await click("CollectionsJournalTab3")
	await click("ToyBoxSearchBox")
	var edit := control("ToyBoxSearchBox") as LineEdit
	edit.select_all()
	await type_text(title)
	if not await wait_until(func(): return control("ToySpellButton1Name") != null and control("ToySpellButton1Name").text == title, "source toy search"): return
	await click("ToySpellButton1")
	await click("CollectionsMicroButton")
	if not await wait_until(func(): return client.find_child("GroundSpellReticle", true, false) != null, "owned toy ground cursor", 30000): return

func scarecrow() -> Node3D:
	for unit in client.find_children("Pandaren Scarecrow", "Node3D", true, false):
		if not unit.find_children("NpcModel", "Node3D", true, false).is_empty() and not unit.find_children("*", "MeshInstance3D", true, false).is_empty(): return unit
	return null

func capture(label: String) -> void:
	await super.capture("cursor-" + label)
