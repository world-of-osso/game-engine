extends "res://tests/toy_effects19_gladiator_live.gd"
# Real learning/journal activation, native back attachment, then ordinary right-click
# AttackSwing against an observed source Blackrock Worg. No aura/combat injection or cooldown reset.
func run_fixture() -> void:
	root.size = Vector2i(1920, 1080)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_until(func(): return not client.account_state().assets_starting, "assets", 180000): return
	if OS.get_environment("GODOT_TEST_SERVER") != "127.0.0.1:5911": fail("private5911 required"); return
	var account := OS.get_environment("TOYFX_ACCOUNT")
	if not account.begins_with("fb_"): fail("private fb_* account required"); return
	var error: String = client.connect_account("127.0.0.1:5911", account, "fbtest", false)
	if error != "": fail(error); return
	if not await wait_until(func(): return control("CharCard_0") != null, "roster", 120000): return
	await click("CharCard_0")
	await click("EnterWorld")
	if not await wait_until(func(): return client.account_state().screen == "InWorld" and control("MinimapLauncherButton") != null, "world", 180000): return
	# Select through the actual Options UI; offline preview defaults omit live HUD rows.
	await tap(KEY_ESCAPE)
	await click("MenuBtnOptions")
	await click("OptionsTabhud")
	var wanted_skin := "Modern" if skin == "modern" else "Forever"
	if client.account_state().ui_skin != wanted_skin:
		await click("Choiceui_layout%sHit" % ("1" if skin == "forever" else "0"))
	if control("CollectionsMicroButton") == null:
		await click("ToggleSwitchlayout_show_micro_menuRightHit")
	await click("OptionsDoneButton")
	if control("MenuBtnResume") != null: await click("MenuBtnResume")
	if client.account_state().ui_skin != wanted_skin: fail("wrong native art skin"); return
	if not await wait_until(func(): return control("CollectionsMicroButton") != null, "enabled native Micro Menu"): return
	if not await wait_until(func(): return native_display(-1), "native player model", 120000): return
	if not await wait_until(func(): return client.merchant_state().item_catalog_loaded, "source item catalog", 180000): return
	if not await wait_until(func(): return client.toybox_state().catalog_count == 1166, "full authenticated toy catalog", 120000): return
	await capture("native")
	for sample in proof_cases():
		await prove_toy(sample)
		if failed: return
	write_receipts()
	print("PASS toy-effects14 actual native cases ", JSON.stringify(proof_cases()), " ", skin)
	client.free()
	quit(0)

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
		receipts.append({"phase": "source-target-candidate", "target": state})
		if state.target == null or not str(state.target_name).contains("Blackrock Worg"): continue
		var candidate := int(state.target)
		if client.unit_transform(candidate).origin.distance_to(client.unit_transform(local_id).origin) < 30.0:
			target = candidate
			break
	if target == 0:
		write_receipts()
		fail("no observed source Blackrock Worg within30yd")
		return
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
