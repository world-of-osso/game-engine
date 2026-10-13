extends "res://tests/toy_effects14_scaled_live.gd"
# Actual Toy Box activation of placed statue scenery on private5911.
# [item, spell, source displays, title, template DisplayScale, entry, native CDI scale by display]
func proof_cases() -> Array:
	return [
		[198646, 384618, [108683], "Ornate Dragon Statue", 0.7, 194793, {108683: 0.3}],
		[204405, 405204, [111821], "Stuffed Bear", 1.0, 203158, {111821: 1.0}],
	]

func prove_toy(sample: Array) -> void:
	await learn_toy(int(sample[0]))
	if failed: return
	await click("CollectionsMicroButton")
	await click("CollectionsJournalTab3")
	await click("ToyBoxSearchBox")
	var edit := control("ToyBoxSearchBox") as LineEdit
	edit.select_all()
	await type_text(str(sample[3]))
	if not await wait_until(func(): return matching_toy_button(str(sample[3])) != "", "statue toy search"): return
	await click(matching_toy_button(str(sample[3])))
	await click("CollectionsMicroButton")
	receipts.append({"phase": "statue-activation", "ticks_msec": Time.get_ticks_msec(), "item": sample[0], "account": client.account_state(), "toys": client.toybox_state()})
	write_receipts()
	if not await wait_until(func(): return scenery(str(sample[3])) != null, "statue native meshes " + str(sample[0]), 120000): return
	var placed := scenery(str(sample[3]))
	if abs(placed.scale.x - float(sample[4])) > 0.00001: fail("authored display scale was dropped"); return
	if control("PetFrame") != null: fail("statue acquired the combat pet frame"); return
	var model := placed.find_children("NpcModel", "Node3D", true, false)[0] as Node3D
	var visual := model.get_parent() as Node3D
	var display := int(visual.get_meta("creature_display_id"))
	if not sample[2].has(display): fail("wrong statue source display %d" % display); return
	if abs(visual.scale.x - float(sample[6][display])) > 0.00001: fail("native CDI scale was dropped"); return
	var before := placed.global_position
	var position: Vector3 = client.account_state().local_player_position
	if not command(OS.get_environment("TOYFX_ADMIN"), ["set-position", character, str(position.x + 3.0), str(-position.z), str(position.y)]): return
	await create_timer(1.5).timeout
	if scenery(str(sample[3])) != placed or placed.global_position.distance_to(before) > 0.05: fail("statue followed or vanished"); return
	receipts.append({"phase": "statue-source-scenery", "item": sample[0], "entry": sample[5], "display": display, "source_display_scale": sample[4], "replicated_root_scale": [placed.scale.x, placed.scale.y, placed.scale.z], "native_visual_scale": [visual.scale.x, visual.scale.y, visual.scale.z], "meshes": model.find_children("*", "MeshInstance3D", true, false).size(), "placed": str(placed.global_position)})
	await capture("statue-toy%d-display%d" % [sample[0], display])
