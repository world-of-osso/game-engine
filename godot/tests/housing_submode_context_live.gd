extends "res://tests/housing_expert_source_live.gd"

# Bindings_Standard.xml gives these actions the Expert-only binding context.
# The wrapper's defensive activation does not make them global shortcuts.
const SUBMODE_KEYS: Array = [["Translate", KEY_F8], ["Rotate", KEY_F9], ["Scale", KEY_F10]]

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0:
		fail("Submode context proof requires explicit private5870 paid owner configuration")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	var before: Dictionary = client.housing_state()
	if not before.can_edit or before.decor.size() != 1 or int(before.decor[0].decor_id) != 1693:
		fail("Submode context proof requires the genuine editable paid bookcase")
		return
	if not await configure_expert_bindings(false): return
	for origin in [["Basic", "HouseEditorBasicDecorModeButton"], ["Cleanup", "HouseEditorCleanupModeButton"]]:
		if not await prove_inactive_context(origin): return
	if not await enter_expert(): return
	# Start in Scale so the Translate key must make a real submode change.
	if not await click_exposed_control(housing_control("ScaleSubmodeButton")): return
	if not await wait_housing(func(): return client.housing_state().precision_submode == "Scale", "initial visible Scale submode"): return
	for submode in SUBMODE_KEYS:
		await source_key(submode[1])
		if not await wait_housing(func(): return editor_mode_is("Expert") and client.housing_state().precision_submode == submode[0], "assigned Expert-context " + submode[0]): return
		if not await record_editor("submode-context-expert-" + submode[0].to_lower()): return
	if not await configure_expert_bindings(true): return
	if not await click_exposed_control(housing_control("HouseEditorBasicDecorModeButton")): return
	if not await wait_housing(func(): return editor_mode_is("Basic"), "Basic after clearing bindings"): return
	for key in [KEY_F8, KEY_F9]:
		await source_key(key)
		await frames(3)
		if not editor_mode_is("Basic"):
			fail("Unbound Expert submode key still changes the editor mode")
			return
	if client.housing_state().decor != before.decor or client.housing_state().stored != before.stored:
		fail("Submode context browsing changed authoritative paid decor or storage")
		return
	print("PASS: private5870 Expert submode binding contexts ", housing_skin)
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

func prove_inactive_context(origin: Array) -> bool:
	if not await click_exposed_control(housing_control(origin[1])): return false
	if not await wait_housing(func(): return editor_mode_is(origin[0]), "source starting mode " + origin[0]): return false
	var before: Dictionary = client.housing_state()
	for submode in SUBMODE_KEYS:
		await source_key(submode[1])
		await frames(3)
		if not editor_mode_is(origin[0]) or client.housing_state().precision_submode != before.precision_submode:
			fail("Inactive Expert binding changed " + origin[0] + " mode or precision submode")
			return false
		# Outside the Expert context the existing global F10 may own its key.
		if client.get_node_or_null("HudEditUI") != null:
			await source_key(KEY_F10)
			if not await wait_housing(func(): return client.get_node_or_null("HudEditUI") == null, "global HUD key closes its own overlay"): return false
	return await record_editor("submode-context-inactive-" + origin[0].to_lower())
