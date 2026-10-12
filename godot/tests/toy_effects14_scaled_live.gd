extends "res://tests/toy_effects14_live.gd"
# Actual bag/journal activation of source-scaled stationary scenery on private5911.
func proof_cases() -> Array:
	return [[170476, 304620, 92605, "Underlight Sealamp", 0.35, 156086, false]]

func prove_toy(sample: Array) -> void:
	await learn_toy(int(sample[0]))
	if failed: return
	await click("CollectionsMicroButton")
	await click("CollectionsJournalTab3")
	await click("ToyBoxSearchBox")
	var edit := control("ToyBoxSearchBox") as LineEdit
	edit.select_all()
	await type_text(str(sample[3]))
	if not await wait_until(func(): return control("ToySpellButton1Name") != null and control("ToySpellButton1Name").text == str(sample[3]), "source scaled toy search"): return
	await click("ToySpellButton1")
	await click("CollectionsMicroButton")
	var clicked = null
	if sample[6]:
		clicked = await place_cursor_scenery()
		if failed: return
	receipts.append({"phase": "scaled-activation", "item": sample[0], "clicked": str(clicked), "account": client.account_state(), "toys": client.toybox_state()})
	write_receipts()
	if not await wait_until(func(): return scenery(str(sample[3])) != null, "source scenery native meshes " + str(sample[0]), 120000): return
	var placed := scenery(str(sample[3]))
	if abs(placed.scale.x - float(sample[4])) > 0.00001: fail("authored display scale was dropped"); return
	if clicked != null and placed.global_position.distance_to(clicked) > 0.2: fail("scaled scenery ignored clicked point"); return
	if control("PetFrame") != null: fail("scenery acquired the combat pet frame"); return
	var model := placed.find_children("NpcModel", "Node3D", true, false)[0] as Node3D
	receipts.append({"phase": "scaled-source-scenery", "item": sample[0], "entry": sample[5], "display": sample[2], "source_display_scale": sample[4], "replicated_root_scale": [placed.scale.x, placed.scale.y, placed.scale.z], "native_visual_scale": [model.scale.x, model.scale.y, model.scale.z], "meshes": model.find_children("*", "MeshInstance3D", true, false).size(), "clicked": str(clicked), "placed": str(placed.global_position)})
	await capture("toy%d-display%d-scale%s" % [sample[0], sample[2], str(sample[4]).replace(".", "")])

func scenery(title: String) -> Node3D:
	for unit in client.find_children(title, "Node3D", true, false):
		if not unit.find_children("NpcModel", "Node3D", true, false).is_empty() and not unit.find_children("*", "MeshInstance3D", true, false).is_empty(): return unit
	return null

func place_cursor_scenery():
	for unit in client.find_children(character, "Node3D", true, false):
		if unit.get_node_or_null("PlayerModel") != null:
			actor = unit
			break
	if actor == null: fail("native actor missing"); return null
	if not await wait_until(func(): return client.find_child("GroundSpellReticle", true, false) != null, "scaled ground cursor", 30000): return null
	var camera := root.get_camera_3d()
	if camera == null: fail("world camera missing"); return null
	var forward := -camera.global_basis.z
	forward.y = 0.0
	var point := camera.unproject_position(actor.global_position + forward.normalized() * 3.0)
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await create_timer(0.5).timeout
	var reticle := client.find_child("GroundSpellReticle", true, false) as Node3D
	if reticle == null or not reticle.visible: fail("no source ground under scaled cursor"); return null
	var clicked := reticle.global_position - Vector3.UP * 0.06
	for down in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = down
		root.push_input(event, true)
		await process_frame
	return clicked

func capture(label: String) -> void:
	await super.capture("scaled-" + label)
