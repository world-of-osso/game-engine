extends "res://tests/housing_basic_bindings_live.gd"

func run_test() -> void:
	root.size = Vector2i(1280,720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server!="127.0.0.1:5870" or housing_account!="fb_housing_core_modern2" or shots.is_empty() or editor_house!=1:
		fail("Diagnostic requires exact failed private Modern owner receipt")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	if not await select_saved_decor(): return
	if housing_text("HouseEditorRotateLeftInstructionControlText")!="F5" or housing_text("HouseEditorRotateRightInstructionControlText")!="F6":
		fail("Diagnostic requires the previous actual F5/F6 assignment")
		return
	print("BOUND_DIAGNOSTIC_BEFORE ",client.housing_state())
	if not await record_editor("bound-diagnostic-before"): return
	await source_key(KEY_F5)
	if not await wait_housing(func(): return not client.housing_state().get("pending",true),"diagnostic F5 authoritative ACK",120000): return
	print("BOUND_DIAGNOSTIC_F5 ",client.housing_state())
	if not await record_editor("bound-diagnostic-F5"): return
	await source_key(KEY_F6)
	if not await wait_housing(func(): return not client.housing_state().get("pending",true),"diagnostic F6 authoritative ACK",120000): return
	print("BOUND_DIAGNOSTIC_F6 ",client.housing_state())
	if not await record_editor("bound-diagnostic-F6"): return
	await press_escape()
	if not await clear_housing_bindings(): return
	print("PASS: private5870 bound diagnostic recorded and prior fixture keys unbound")
	client.free()
	quit(0)
