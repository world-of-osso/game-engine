extends "res://tests/housing_basic_source_live.gd"

const BIND_RIGHT: String = "HOUSING_BASICDECOR_ROTATERIGHT"
const BIND_LEFT: String = "HOUSING_BASICDECOR_ROTATELEFT"

func run_test() -> void:
	root.size = Vector2i(1280,720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server!="127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house<=0:
		fail("Binding proof needs private5870/fb_housing_* owned house")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	var before: Dictionary = client.housing_state()
	if before.decor.size()!=1 or int(before.decor[0].decor_id)!=1693 or housing_count(2330,"stored")!=1:
		fail("Binding proof needs real paid decor fixture")
		return
	if not await assign_housing_bindings(): return
	if not await select_saved_decor(): return
	if not await wait_housing(func(): return housing_text("HouseEditorRotateRightInstructionControlText")=="F6" and housing_text("HouseEditorRotateLeftInstructionControlText")=="F5","assigned source hotkey labels"): return
	if not await record_editor("basic-bindings-assigned"): return
	await source_key(KEY_F6)
	if not await wait_housing(func(): return absf(client.housing_state().decor[0].yaw-before.decor[0].yaw)>0.05 and not client.housing_state().get("pending",true),"assigned F6 dispatches normal RotateDecor"): return
	if not await record_editor("basic-bound-key-rotated"): return
	await source_key(KEY_F5)
	if not await wait_housing(func(): return absf(client.housing_state().decor[0].yaw-before.decor[0].yaw)<0.001 and not client.housing_state().get("pending",true),"assigned F5 restores actual yaw"): return
	await press_escape()
	if not await wait_housing(func(): return int(client.housing_state().get("selected",0))==0,"Escape cancels selected editing"): return
	if not await prove_preview_rotation(before): return
	if not await clear_housing_bindings(): return
	if not await select_saved_decor(): return
	if housing_text("HouseEditorRotateRightInstructionControlText")!="[UNBOUND]" or housing_text("HouseEditorRotateLeftInstructionControlText")!="[UNBOUND]":
		fail("Unbind did not restore source unbound instructions")
		return
	await source_key(KEY_F6)
	await frames(20)
	if client.housing_state().decor!=before.decor or client.housing_state().stored!=before.stored:
		fail("Unbound old key still changed confirmed paid decor")
		return
	await press_escape()
	if not await record_editor("basic-bindings-unbound-restored"): return
	print("PASS: private5870 assigned Retail housing bindings native UI/input ",housing_skin)
	client.free()
	quit(0)

func prove_preview_rotation(before: Dictionary) -> bool:
	if not await click_exposed_control(housing_control("HouseEditorStored2330")): return false
	push_mouse_motion(ground_point(7.0))
	if not await wait_housing(func(): return client.get_node_or_null("HousingDecorPreview")!=null,"actual owned Tesla preview"): return false
	var preview: Node3D = client.get_node("HousingDecorPreview") as Node3D
	var yaw: float = preview.rotation.y
	await source_key(KEY_F6)
	await frames(20)
	preview = client.get_node_or_null("HousingDecorPreview") as Node3D
	if preview==null or absf(preview.rotation.y-yaw)<0.05:
		await press_escape()
		await clear_housing_bindings()
		fail("Assigned source Rotate Right did not rotate owned placement preview")
		return false
	if client.housing_state().stored!=before.stored or client.housing_state().decor!=before.decor:
		fail("Preview rotation mutated confirmed paid copies")
		return false
	if not await record_editor("basic-bound-preview-rotated"): return false
	await source_key(KEY_F5)
	await press_escape()
	return await wait_housing(func(): return int(client.housing_state().get("placing",0))==0 and housing_control("HouseEditorStored2330")!=null,"preview Escape restores exact owned tile")

func source_key(code: Key) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.pressed = true
	Input.parse_input_event(event)
	await frames(3)
	event.pressed = false
	Input.parse_input_event(event)
	await frames(3)

func menu_control(name: String) -> Control:
	return housing_named_control("GameMenuUI",name)

func open_housing_binding_page() -> bool:
	if client.get_node_or_null("HousingDashboardUI")!=null:
		await press_escape()
		if not await wait_housing(func(): return client.get_node_or_null("HousingDashboardUI")==null,"inactive editor Escape closes housing window"): return false
	await press_escape()
	if not await wait_housing(func(): return menu_control("MenuBtnOptions")!=null,"native game menu"): return false
	if not await click_exposed_control(menu_control("MenuBtnOptions")): return false
	if not await wait_housing(func(): return menu_control("OptionsTabkeybindings")!=null,"native Options"): return false
	if not await click_exposed_control(menu_control("OptionsTabkeybindings")): return false
	if not await wait_housing(func(): return menu_control("KeybindingSectionhousingButton")!=null,"registered Housing section"): return false
	if not await click_exposed_control(menu_control("KeybindingSectionhousingButton")): return false
	return await wait_housing(func(): return menu_control("KeybindingButton"+BIND_RIGHT)!=null,"registered Retail action names")

func binding_label(action: String) -> String:
	var host: Node = client.get_node_or_null("GameMenuUI")
	return str(host.frame_text("KeybindingButtonText"+action)) if host!=null else ""

func assign_housing_bindings() -> bool:
	if not await open_housing_binding_page(): return false
	for action: String in [BIND_RIGHT,BIND_LEFT]:
		if binding_label(action)!="Not Bound":
			fail("Fixture must not overwrite a player's assigned housing key: "+action)
			return false
		if not await click_exposed_control(menu_control("KeybindingButton"+action)): return false
		if not await wait_housing(func(): return binding_label(action)=="Press a key…","native key capture"): return false
		await source_key(KEY_F6 if action==BIND_RIGHT else KEY_F5)
		if not await wait_housing(func(): return binding_label(action)==("F6" if action==BIND_RIGHT else "F5"),"native assigned key label"): return false
	if not await record_editor("housing-keybinding-options"): return false
	return await close_binding_page()

func clear_housing_bindings() -> bool:
	if not await open_housing_binding_page(): return false
	for action: String in [BIND_RIGHT,BIND_LEFT]:
		var control: Control = menu_control("KeybindingButton"+action)
		await click(control.get_global_rect().get_center(),MOUSE_BUTTON_RIGHT)
		if not await wait_housing(func(): return binding_label(action)=="Not Bound","native right-click unbind"): return false
	return await close_binding_page()

func close_binding_page() -> bool:
	if not await click_exposed_control(menu_control("OptionsCloseButton")): return false
	if not await wait_housing(func(): return client.get_node_or_null("GameMenuUI")==null,"Options Close closes menu"): return false
	return await open_cleanup_house()
