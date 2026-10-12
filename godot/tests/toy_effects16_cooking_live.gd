extends "res://tests/toy_effects14_live.gd"
# Real journal use and ordinary CastSpellByID, private5911/fb_* only.
func proof_cases() -> Array:
	return [
		[67097, 93636, 207461, "Grim Campfire", 396, 1.0, 198193],
		[153039, 254240, 277297, "Crystalline Campfire", 45521, 0.75, 1720843],
		[198402, 383081, 379147, "Maruuk Cooking Pot", 75339, 0.75, 4182540],
	]

func prove_toy(sample: Array) -> void:
	if int(sample[0]) == 67097:
		var errors := client.spells_state().errors.size()
		var meat := bag_count(2672)
		var food := bag_count(2679)
		if client.use_spell(2538) != "": fail("native cooking request unavailable"); return
		if not await wait_until(func(): return client.spells_state().errors.size() > errors, "no-fire cooking refusal"): return
		if not str(client.spells_state().errors[-1]).contains("Requires Cooking Fire"): fail("wrong source focus refusal"); return
		if bag_count(2672) != meat or bag_count(2679) != food: fail("no-focus cast changed real bags"); return
		receipts.append({"phase": "no-fire-cooking", "spells": client.spells_state(), "meat": meat, "food": food})
	await learn_toy(int(sample[0]))
	if failed: return
	await click("CollectionsMicroButton")
	await click("CollectionsJournalTab3")
	await click("ToyBoxSearchBox")
	var search := control("ToyBoxSearchBox") as LineEdit
	search.select_all()
	await type_text(str(sample[3]))
	if not await wait_until(func(): return control("ToySpellButton1Name") != null and control("ToySpellButton1Name").text == str(sample[3]), "source fire search"): return
	await click("ToySpellButton1")
	await click("CollectionsMicroButton")
	if not await wait_until(func(): return fire(int(sample[2])) != null, "native cooking-fire model " + str(sample[0]), 45000): return
	var object := fire(int(sample[2]))
	var model := object.get_node("GameObjectModel") as Node3D
	if int(object.get_meta("game_object_display_id")) != int(sample[4]): fail("wrong source GO display"); return
	if int(model.get_meta("model_file_data_id")) != int(sample[6]): fail("wrong source fire model"); return
	if abs(object.scale.x - float(sample[5])) > 0.00001: fail("source fire scale changed"); return
	if not model.find_children("*", "Area3D", true, false).is_empty(): fail("cooking-only focus gained interaction"); return
	if not await wait_until(func(): return client.account_state().get("game_object_particles", {}).get("drawn", 0) > 0, "authored game-object fire particles"): return
	var meat := bag_count(2672)
	var food := bag_count(2679)
	if client.use_spell(2538) != "": fail("native cooking request unavailable"); return
	if not await wait_until(func(): return bag_count(2672) == meat - 1 and bag_count(2679) == food + 1, "real recipe reagents/output"): return
	receipts.append({"phase": "source-fire-cooking", "item": sample[0], "entry": sample[2], "display": sample[4], "fdid": sample[6], "scale": object.scale.x, "meshes": model.find_children("*", "MeshInstance3D", true, false).size(), "particles": client.account_state().get("game_object_particles"), "meat_before": meat, "meat_after": bag_count(2672), "food_before": food, "food_after": bag_count(2679), "spells": client.spells_state()})
	await capture_fire(object, "cooking-toy%d" % sample[0])

func bag_count(item: int) -> int:
	var count := 0
	for bag in client.merchant_state().bags:
		if int(bag.item_id) == item: count += int(bag.count)
	return count

func fire(entry: int) -> Node3D:
	for object in client.find_children("*", "Node3D", true, false):
		if object.has_meta("game_object_entry") and int(object.get_meta("game_object_entry")) == entry:
			var model := object.get_node_or_null("GameObjectModel")
			if model != null and not model.find_children("*", "MeshInstance3D", true, false).is_empty(): return object
	return null

func capture_fire(object: Node3D, label: String) -> void:
	var meshes := object.find_children("*", "MeshInstance3D", true, false)
	var first := meshes[0] as MeshInstance3D
	var bounds := first.global_transform * first.get_aabb()
	for index in range(1, meshes.size()):
		var mesh := meshes[index] as MeshInstance3D
		bounds = bounds.merge(mesh.global_transform * mesh.get_aabb())
	var camera := root.get_camera_3d()
	var transform := camera.global_transform
	var distance := bounds.size.length() / 2.0 / sin(deg_to_rad(camera.fov / 2.0)) * 1.5
	client.set_process(false)
	camera.global_position = bounds.get_center() + Vector3(0.0, 0.5, 1.0).normalized() * distance
	camera.look_at(bounds.get_center(), Vector3.UP)
	await super.capture(label)
	camera.global_transform = transform
	client.set_process(true)
