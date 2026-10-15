extends "res://tests/housing_expert_source_live.gd"

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0:
		fail("Submode entry proof requires explicit private5870 paid owner configuration")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	var before: Dictionary = client.housing_state()
	if not before.can_edit or before.decor.size() != 1 or int(before.decor[0].decor_id) != 1693:
		fail("Submode entry proof requires the genuine editable paid bookcase")
		return
	if not await configure_expert_bindings(false): return
	for origin in [["Basic", "HouseEditorBasicDecorModeButton"], ["Cleanup", "HouseEditorCleanupModeButton"]]:
		for submode in [["Translate", KEY_F8], ["Rotate", KEY_F9], ["Scale", KEY_F10]]:
			if not await prove_submode_entry(origin, submode): return
			if client.housing_state().decor != before.decor or client.housing_state().stored != before.stored:
				fail("Assigned submode entry changed authoritative paid decor or storage")
				return
	if not await configure_expert_bindings(true): return
	if not await click_exposed_control(housing_control("HouseEditorBasicDecorModeButton")): return
	if not await wait_housing(func(): return editor_mode_is("Basic"), "Basic after clearing bindings"): return
	for key in [KEY_F8, KEY_F9]:
		await source_key(key)
		await frames(3)
		if not editor_mode_is("Basic"):
			fail("Unbound Expert submode key still enters Advanced Mode")
			return
	print("PASS: private5870 assigned Expert submode enters from Basic and Cleanup ", housing_skin)
	client.free()
	quit(0)

func editor_mode_is(mode: String) -> bool:
	if housing_control("HouseEditorFrame") == null: return false
	var basic := housing_control("HouseEditorBasicSubmodeBar") != null
	var expert := housing_control("HouseEditorExpertSubmodeBar") != null
	match mode:
		"Basic": return basic and not expert
		"Expert": return expert and not basic
		"Cleanup": return not basic and not expert
	return false

func prove_submode_entry(origin: Array, submode: Array) -> bool:
	if not await click_exposed_control(housing_control(origin[1])): return false
	if not await wait_housing(func(): return editor_mode_is(origin[0]), "source starting mode " + origin[0]): return false
	if not await record_editor("submode-entry-" + origin[0].to_lower() + "-" + submode[0].to_lower() + "-before"): return false
	await source_key(submode[1])
	if not await record_editor("submode-entry-" + origin[0].to_lower() + "-" + submode[0].to_lower() + "-after"): return false
	if not await wait_housing(func(): return editor_mode_is("Expert") and client.housing_state().precision_submode == submode[0], "assigned " + submode[0] + " enters Expert from " + origin[0], 10000):
		print("SUBMODE_ENTRY_FAILURE origin=", origin, " submode=", submode, " state=", client.housing_state())
		return false
	if housing_control("HouseEditorExpertSubmodeBar") == null or housing_control("HouseEditorStorageFrame") != null:
		fail("Expert shortcut did not reveal the existing Expert bar and hide storage")
		return false
	return true
