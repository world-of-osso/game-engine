extends "res://tests/toy_effects14_signs_framed_live.gd"
# Bounded witness: real journal use, newly received Snake models and source expiry.
# Retail's follow promise remains unresolved; this is NOT complete-Flute proof.
var prior_snakes: Array[int] = []

func proof_cases() -> Array:
	return [[98552, 142917, 0, "Xan'tish's Flute", 1.0, 71500, false, 60000]]

func prove_toy(_sample: Array) -> void:
	await learn_toy(98552)
	if failed: return
	for unit in snake_units(): prior_snakes.append(unit.get_instance_id())
	await click("CollectionsMicroButton")
	await click("CollectionsJournalTab3")
	await click("ToyBoxSearchBox")
	var edit := control("ToyBoxSearchBox") as LineEdit
	edit.select_all()
	await type_text("Xan'tish's Flute")
	if not await wait_until(func(): return matching_toy_button("Xan'tish's Flute") != "", "source Flute search"): return
	var activated_at := Time.get_ticks_msec()
	await click(matching_toy_button("Xan'tish's Flute"))
	await click("CollectionsMicroButton")
	if not await wait_until(func(): return new_snakes().size() >= 2, "source variable Snake count", 30000): return
	await create_timer(1.0).timeout
	var snakes := new_snakes()
	if snakes.size() < 2 or snakes.size() > 6: fail("native source count outside variance/truncation interval"); return
	for snake in snakes:
		if not await wait_until(func(): return not snake.find_children("*", "MeshInstance3D", true, false).is_empty(), "native source Snake meshes", 30000): return
	if control("PetFrame") != null: fail("wild snakes acquired combat pet frame"); return
	var first_position: Vector3 = snakes[0].global_position
	for snake in snakes:
		if snake.global_position.distance_to(first_position) > 0.01: fail("zero primary radius scattered Snakes"); return
		var model := snake.find_children("NpcModel", "Node3D", true, false)[0] as Node3D
		var visual := model.get_parent() as Node3D
		var display := int(visual.get_meta("creature_display_id"))
		if display not in [3126, 4440, 35803, 36580]: fail("substituted Snake display"); return
		receipts.append({"phase": "flute-source-model", "display": display, "position": str(snake.global_position), "meshes": model.find_children("*", "MeshInstance3D", true, false).size()})
	receipts.append({"phase": "flute-native-count", "count": snakes.size(), "activated_at": activated_at})
	write_receipts()
	await capture("toy98552-source-snakes")
	if failed: return
	if not await wait_until(func(): return new_snakes().is_empty(), "source60000ms natural Snake removal", 80000): return
	var elapsed := Time.get_ticks_msec() - activated_at
	if elapsed < 59000 or elapsed > 80000: fail("source Snake lifetime outside unchanged wall window"); return
	receipts.append({"phase": "flute-natural-expiry", "elapsed_ms": elapsed})
	write_receipts()

func scenery(_title: String) -> Node3D:
	var snakes := new_snakes()
	return null if snakes.is_empty() else snakes[0]

func snake_units() -> Array[Node3D]:
	var result: Array[Node3D] = []
	var units := client.find_child("WorldUnits", true, false)
	if units == null: return result
	for unit in units.get_children():
		if unit is Node3D and unit.has_meta("unit_name") and str(unit.get_meta("unit_name")) == "Snake": result.append(unit)
	return result

func new_snakes() -> Array[Node3D]:
	var result: Array[Node3D] = []
	for unit in snake_units():
		if unit.get_instance_id() not in prior_snakes: result.append(unit)
	return result
