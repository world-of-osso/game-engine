extends "res://tests/toy_effects16_cooking_live.gd"
# Actual owned-item learning/journal use, recipe and source Cozy Fire refresh.
func proof_cases() -> Array:
	return [[70722, 99767, 208935, "Little Wickerman", 10726, 0.25, 524488]]

func prove_toy(sample: Array) -> void:
	if not await wait_until(func(): return bag_count(2672) > 0, "owned recipe reagents", 120000): return
	await learn_toy(int(sample[0]))
	if failed: return
	await click("CollectionsMicroButton")
	await click("CollectionsJournalTab3")
	await click("ToyBoxSearchBox")
	var search := control("ToyBoxSearchBox") as LineEdit
	search.select_all()
	await type_text(str(sample[3]))
	if not await wait_until(func(): return control("ToySpellButton1Name") != null and control("ToySpellButton1Name").text == str(sample[3]), "source linked fire search"): return
	await click("ToySpellButton1")
	await click("CollectionsMicroButton")
	if not await wait_until(func(): return fire(int(sample[2])) != null and has_aura(7353), "native linked fire and Cozy Fire", 45000): return
	var object := fire(int(sample[2]))
	var model := object.get_node("GameObjectModel") as Node3D
	if int(object.get_meta("game_object_display_id")) != int(sample[4]): fail("wrong source display"); return
	if int(model.get_meta("model_file_data_id")) != int(sample[6]): fail("wrong source fire model"); return
	if abs(object.scale.x - float(sample[5])) > 0.00001: fail("wrong source fire scale"); return
	if not await wait_until(func(): return int(object.get_meta("game_object_particle_quads", 0)) > 0, "own authored fire particles"): return
	if not await wait_until(func(): return client.spells_state().casting == 0 and client.spells_state().gcd_ms == 0, "source cast completion"): return
	var meat := bag_count(2672)
	var food := bag_count(2679)
	if client.use_spell(2538) != "": fail("native cooking request unavailable"); return
	if not await wait_until(func(): return bag_count(2672) == meat - 1 and bag_count(2679) == food + 1, "source recipe output"): return
	receipts.append({"phase": "linked-fire-recipe", "item": sample[0], "entry": sample[2], "display": sample[4], "fdid": sample[6], "scale": object.scale.x, "meshes": model.find_children("*", "MeshInstance3D", true, false).size(), "emitters": int(object.get_meta("game_object_particle_emitters", 0)), "quads": int(object.get_meta("game_object_particle_quads", 0)), "auras": client.aura_state(), "meat_before": meat, "meat_after": bag_count(2672), "food_before": food, "food_after": bag_count(2679)})
	await capture_fire(object, "linked-fire-toy%d" % sample[0])
	# Ordinary native cancellation while in range must be refreshed by the trap,
	# not an injected aura or an instant permanent area lease.
	await cancel_aura(7353)
	if failed: return
	if not await wait_until(func(): return not has_aura(7353), "actual native buff cancellation"): return
	var started := Time.get_ticks_msec()
	if not await wait_until(func(): return has_aura(7353), "source trap refresh", 6000): return
	receipts.append({"phase": "cozy-fire-refresh", "elapsed_ms": Time.get_ticks_msec() - started, "auras": client.aura_state()})
	await capture("linked-fire-buff-refreshed")
