extends "res://tests/toy_effects19_gladiator_live.gd"
# Real learning/journal activation, native back attachment, then ordinary right-click
# AttackSwing against an observed source Kobold. No aura/combat injection or cooldown reset.
func proof_cases() -> Array:
	return [[169275, 300104, [2925498], "Alliance War Banner"]]

func prove_toy(sample: Array) -> void:
	var item := int(sample[0])
	var spell := int(sample[1])
	await learn_toy(item)
	if failed or not learned_toy(item): return
	await click("CollectionsMicroButton")
	await click("CollectionsJournalTab3")
	if not await wait_until(func(): return control("ToyBoxSearchBox") != null, "Toy Box"): return
	await click("ToyBoxSearchBox")
	var edit := control("ToyBoxSearchBox") as LineEdit
	edit.select_all()
	await type_text(str(sample[3]))
	if not await wait_until(func(): return control("ToySpellButton1Name") != null and control("ToySpellButton1Name").text == str(sample[3]), "War Banner search"): return
	await click("ToySpellButton1")
	await click("CollectionsMicroButton")
	if not await wait_until(func(): return has_aura(spell) and kit_model(spell, sample[2]) != 0, "authored faction back banner", 120000): return
	if not native_display(-1): fail("War Banner replaced native model"); return
	receipts.append({"phase": "war-banner", "item": item, "spell": spell, "model": kit_model(spell, sample[2])})
	client.set_camera_orbit(PI / 2.0, -0.3, 8.0)
	await capture("war-toy%d-back-banner" % item)
	await enter_source_combat()
	if failed: return
	if not await wait_until(func(): return not has_aura(spell) and kit_model(spell, sample[2]) == 0 and native_display(-1), "combat removed banner", 30000): return
	receipts.append({"phase": "war-combat-removal", "item": item, "spell": spell, "target": client.target_state(), "combat": client.assisted_combat_state()})
	await capture("war-toy%d-combat-restored" % item)
	await tap(KEY_ESCAPE)

func enter_source_combat() -> void:
	var target := 0
	var local_id := int(client.account_state().local_player_id)
	for attempt in range(60):
		await tap(KEY_TAB)
		await create_timer(0.1).timeout
		var state: Dictionary = client.target_state()
		if state.target == null or not str(state.target_name).contains("Kobold"): continue
		var candidate := int(state.target)
		if client.unit_transform(candidate).origin.distance_to(client.unit_transform(local_id).origin) < 30.0:
			target = candidate
			break
	if target == 0: fail("no observed source Kobold within30yd"); return
	var position: Vector3 = client.unit_transform(target).origin
	if not command(OS.get_environment("TOYFX_ADMIN"), ["set-position", character, str(position.x + 2.5), str(-position.z), str(position.y)]): return
	await create_timer(1.0).timeout
	var camera := root.get_camera_3d()
	for angle in range(8):
		client.set_camera_orbit(float(angle) * PI / 4.0, -0.3, 8.0)
		await create_timer(0.3).timeout
		position = client.unit_transform(target).origin + Vector3.UP * 0.8
		var point := camera.unproject_position(position)
		if camera.is_position_behind(position) or not Rect2(Vector2.ZERO, Vector2(root.size)).has_point(point): continue
		var motion := InputEventMouseMotion.new()
		motion.position = point
		motion.global_position = point
		root.push_input(motion, true)
		for pressed in [true, false]:
			var event := InputEventMouseButton.new()
			event.position = point
			event.global_position = point
			event.button_index = MOUSE_BUTTON_RIGHT
			event.pressed = pressed
			root.push_input(event, true)
			await create_timer(0.1).timeout
		if client.target_state().auto_attack == target:
			receipts.append({"phase": "source-right-click-attack", "target": client.target_state(), "source_position": position})
			return
	fail("physical right-click did not start source NPC combat")
