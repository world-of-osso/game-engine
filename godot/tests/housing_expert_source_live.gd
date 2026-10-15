extends "res://tests/housing_basic_bindings_live.gd"

# These are private fixture assignments, never Retail defaults.
const EXPERT_BINDINGS: Array = [
	["HOUSING_TOGGLEBASICDECORMODE", KEY_F1],
	["HOUSING_TOGGLEEXPERTDECORMODE", KEY_F2],
	["HOUSING_TOGGLECLEANUPMODE", KEY_F3],	
	["HOUSING_EXPERTDECORTRANSLATESUBMODE", KEY_F8],
	["HOUSING_EXPERTDECORROTATESUBMODE", KEY_F9],
	["HOUSING_EXPERTDECORSCALESUBMODE", KEY_F10],
	["HOUSING_EXPERTDECORINCREMENT_UP", KEY_F7],
	["HOUSING_EXPERTDECORINCREMENT_SCALEUP", KEY_F5],
	["HOUSING_EXPERTDECORINCREMENT_ROTATERIGHT", KEY_F6],
	["HOUSING_EXPERTDECORROTATION_NEXTAXIS", KEY_F11],
]

var expert_bindings_cleared: bool = false

func run_test() -> void:
	root.size = Vector2i(1280,720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server!="127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house<=0:
		fail("Expert source proof requires private5870 owned-house configuration")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	var before: Dictionary = client.housing_state()
	if before.decor.size()!=1 or int(before.decor[0].decor_id)!=1693 or housing_count(2330,"stored")!=1:
		fail("Expert proof requires the existing paid bookcase and stored Tesla")
		return
	if not await configure_expert_bindings(false): return
	if not await enter_expert(): return
	if not await record_editor("expert-source-unselected"): return
	if not await select_saved_decor(): return
	if not await prove_manipulation_rows(): return
	await press_escape()
	if not await select_saved_decor(): return
	if not await stage_assigned_precision(): return
	if client.housing_state().decor!=before.decor or client.housing_state().stored!=before.stored:
		fail("Expert staging changed authoritative copies/layout")
		return
	if not await record_editor("expert-source-staged"): return
	await press_escape()
	if not await wait_housing(func(): return int(client.housing_state().get("selected",0))==0 and placed_node().position.distance_to(before.decor[0].position)<0.01,"Expert Escape restores saved render"): return
	if not await record_editor("expert-source-cancelled"): return
	if not await select_saved_decor(): return
	if not await stage_assigned_precision(): return
	var expected: Dictionary = client.housing_state().draft.duplicate(true)
	if not await commit_expert_by_world_click(): return
	if not await wait_housing(func(): return not client.housing_state().pending and saved_matches(expected),"sourced world click commits staged transform",120000): return
	if not await record_editor("expert-source-committed"): return
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	if not await wait_housing(func(): return saved_matches(expected),"Expert saved transform survives reauthentication"): return
	if housing_count(2330,"stored")!=1 or stored_bookcases()!=0 or client.housing_state().decor.size()!=1:
		fail("Expert commit/relog lost or duplicated paid copies")
		return
	if not await record_editor("expert-source-relogged"): return
	if not await configure_expert_bindings(true): return
	if not await enter_expert(): return
	if not await select_saved_decor(): return
	await click_control(housing_control("RotateSubmodeButton"),MOUSE_BUTTON_LEFT)
	var unbound: Dictionary = client.housing_state().draft.duplicate(true)
	if not await hold_assigned(KEY_F6): return
	if client.housing_state().draft!=unbound:
		fail("Old unbound Expert key still increments the staged transform")
		return
	await press_escape()
	if not await record_editor("expert-source-unbound"): return
	await press_escape()
	if not await wait_housing(func(): return client.get_node_or_null("HousingDashboardUI")==null,"inactive housing Escape closes editor"): return
	await source_key(KEY_F10)
	if not await wait_housing(func(): return client.get_node_or_null("HudEditUI")!=null,"unbound F10 retains global HUD edit shortcut"): return
	await source_key(KEY_F10)
	if not await wait_housing(func(): return client.get_node_or_null("HudEditUI")==null,"global F10 closes HUD edit normally"): return
	print("PASS: private5870 assigned Retail Expert held-release stage/cancel/click-commit/relog/conservation ",housing_skin)
	client.free()
	quit(0)

func enter_expert() -> bool:
	await click_control(housing_control("HouseEditorExpertDecorModeButton"),MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HouseEditorExpertSubmodeBar")!=null and housing_control("HouseEditorStorageFrame")==null,"Expert source mode"): return false
	return await prove_source_key_labels()

func prove_source_key_labels() -> bool:
	for control in [["HouseEditorBasicDecorModeButton","F1"],["HouseEditorExpertDecorModeButton","F2"],["HouseEditorCleanupModeButton","F3"],["TranslateSubmodeButton","F8"],["RotateSubmodeButton","F9"],["ScaleSubmodeButton","F10"]]:
		var name: String = control[0]+"ControlText"
		var expected: String = "" if expert_bindings_cleared else control[1]
		if not await wait_housing(func(): return housing_text(name)==expected,"assigned/unbound native source mode key label "+name): return false
		if expert_bindings_cleared and housing_control(name)!=null:
			fail("Unbound mode icon retained a fabricated key label: "+name)
			return false
	return await record_editor("expert-mode-key-labels-unbound" if expert_bindings_cleared else "expert-mode-key-labels-assigned")

func stage_assigned_precision() -> bool:
	await source_key(KEY_F9)
	if not await wait_housing(func(): return housing_control("HouseEditorRotateControlRight")!=null,"assigned Rotate submode"): return false
	if not await expert_instruction("RotateRight","Rotate Right","F6"): return false
	for axis in ["pitch","yaw","roll"]:
		var before: float = float(client.housing_state().draft[axis])
		if not await hold_assigned(KEY_F6): return false
		if absf(float(client.housing_state().draft[axis])-before)<0.02:
			fail("Assigned rotate/next-axis did not stage "+axis)
			return false
		await source_key(KEY_F11)
	if not await record_editor("expert-source-rotate"): return false
	await source_key(KEY_F10)
	print("EXPERT_SCALE_DISPATCH state=",client.housing_state()," thumb=",housing_control("HouseEditorScaleThumb")," label=",housing_text("HouseEditorScaleUpInstructionText"))
	if not await record_editor("expert-source-scale-dispatch"): return false
	if not await wait_housing(func(): return housing_control("HouseEditorScaleThumb")!=null,"assigned Scale submode"): return false
	if not await expert_instruction("ScaleUp","Scale Decor Up","F5"): return false
	var scale_before: float = float(client.housing_state().draft.scale)
	if not await hold_assigned(KEY_F5): return false
	if float(client.housing_state().draft.scale)<=scale_before+0.02:
		fail("Distinct assigned scale-up action did not stage scale")
		return false
	if not await record_editor("expert-source-scale"): return false
	await source_key(KEY_F8)
	if not await wait_housing(func(): return housing_control("HouseEditorScaleThumb")==null and housing_control("HouseEditorRotateControlRight")==null,"assigned Translate submode"): return false
	if not await expert_instruction("MoveUp","Move Decor Up","F7"): return false
	var position_before: Vector3 = client.housing_state().draft.position
	if not await hold_assigned(KEY_F7): return false
	if client.housing_state().draft.position.y<=position_before.y+0.02:
		fail("Assigned up action did not stage translation")
		return false
	return await record_editor("expert-source-translate")

func prove_manipulation_rows() -> bool:
	await source_key(KEY_F9)
	if not await wait_housing(func(): return housing_control("HouseEditorRotateControlRight")!=null,"source mouse rotation control"): return false
	var point: Vector2 = housing_control("HouseEditorRotateControlRight").get_global_rect().get_center()
	await tile_mouse_button(point,true)
	if not await wait_housing(func(): return client.housing_state().manipulating and housing_control("HouseEditorRotateRightInstruction")==null and housing_control("HouseEditorRotateSnapInstruction")!=null,"source manipulating rows hide selected-only increments"): return false
	if not await record_editor("expert-source-manipulating"): return false
	await tile_mouse_button(point+Vector2(200,0),false)
	return await wait_housing(func(): return not client.housing_state().manipulating and housing_text("HouseEditorRotateRightInstructionControlText")=="F6","mouse release restores selected instruction set")

func expert_instruction(name: String, label: String, key: String) -> bool:
	if not await wait_housing(func(): return housing_text("HouseEditor"+name+"InstructionText")==label and housing_text("HouseEditor"+name+"InstructionControlText")==key,"native source Expert label and assigned control"): return false
	var scale: float = housing_control("HouseEditorExpertDecorModeButton").get_global_rect().size.x/64.0
	var row: Rect2 = housing_control("HouseEditor"+name+"Instruction").get_global_rect()
	var instructions: Rect2 = housing_control("HouseEditorInstructions").get_global_rect()
	if absf(row.size.y-45.0*scale)>0.1 or absf(instructions.end.x-(root.size.x-30.0*scale))>1.0 or absf(instructions.get_center().y-root.size.y/2.0)>1.0:
		fail("Expert source instruction row height/right-centre anchor drifted")
		return false
	return true

func hold_assigned(code: Key) -> bool:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.pressed = true
	Input.parse_input_event(event)
	await create_timer(0.35).timeout
	event.pressed = false
	Input.parse_input_event(event)
	await frames(4)
	var released: Dictionary = client.housing_state().draft.duplicate(true)
	await create_timer(0.25).timeout
	if client.housing_state().draft!=released:
		fail("Expert increment continued after physical key release")
		return false
	return true

func commit_expert_by_world_click() -> bool:
	var origin: Vector2 = ground_point(8.0)
	print("EXPERT_COMMIT_ORIGINAL point=",origin," picked=",UnitPicker.pick(camera(),origin))
	for offset in [Vector2(-160,80),Vector2(160,80),Vector2(0,120)]:
		var point: Vector2 = origin+offset
		if UnitPicker.pick(camera(),point)!=null: continue
		push_mouse_motion(point)
		await frames(3)
		print("EXPERT_COMMIT_CLEAR_RAY point=",point," picked=",UnitPicker.pick(camera(),point))
		await click(point,MOUSE_BUTTON_LEFT)
		print("EXPERT_COMMIT_DISPATCH selected=",client.housing_state().get("selected",0)," pending=",client.housing_state().pending," draft=",client.housing_state().get("draft",{}))
		return true
	fail("No empty native world ray found for sourced Expert click commit")
	return false

func saved_matches(expected: Dictionary) -> bool:
	var state: Dictionary = client.housing_state()
	if state.decor.size()!=1: return false
	var decor: Dictionary = state.decor[0]
	if decor.position.distance_to(expected.position)>0.01: return false
	for field in ["pitch","yaw","roll","scale"]:
		if absf(float(decor[field])-float(expected[field]))>0.01: return false
	return true

func configure_expert_bindings(clear: bool) -> bool:
	if not OS.get_environment("XDG_CONFIG_HOME").begins_with("/home/osso/.worktrees/"):
		fail("Expert fixtures cannot overwrite a player's native config")
		return false
	if int(client.housing_state().get("selected",0))!=0: await press_escape()
	if not await open_housing_binding_page(): return false
	for binding in EXPERT_BINDINGS:
		var action: String = binding[0]
		var control: Control = await reveal_binding(action)
		if control==null: return false
		if clear:
			await click(control.get_global_rect().get_center(),MOUSE_BUTTON_RIGHT)
			if not await wait_housing(func(): return binding_label(action)=="Not Bound","native Expert right-click unbind"): return false
		else:
			var assigned_key: String = OS.get_keycode_string(binding[1])
			if binding_label(action)==assigned_key:
				# Retry after a failed owned private run: unbind its exact prior fixture key through Options.
				await click(control.get_global_rect().get_center(),MOUSE_BUTTON_RIGHT)
				if not await wait_housing(func(): return binding_label(action)=="Not Bound","unbind prior private fixture assignment"): return false
			elif binding_label(action)!="Not Bound":
				fail("Fixture cannot overwrite an unexpected Expert assignment: "+action)
				return false
			if not await click_exposed_control(menu_control("KeybindingButton"+action)): return false
			await source_key(binding[1])
			if not await wait_housing(func(): return binding_label(action)==OS.get_keycode_string(binding[1]),"native Expert binding assignment"): return false
	if not await record_editor("expert-binding-options-unbound" if clear else "expert-binding-options-assigned"): return false
	expert_bindings_cleared = clear
	return await close_binding_page()

func reveal_binding(action: String) -> Control:
	for _attempt in range(24):
		var control: Control = menu_control("KeybindingButton"+action)
		if control!=null:
			var point: Vector2 = control.get_global_rect().get_center()
			push_mouse_motion(point)
			await frames(3)
			var hovered: Control = root.gui_get_hovered_control()
			if hovered==control or (hovered!=null and control.is_ancestor_of(hovered)): return control
		var viewport: Control = menu_control("OptionsContentScroll")
		if viewport==null:
			fail("Native Options scroll box is unavailable")
			return null
		await click(viewport.get_global_rect().get_center(),MOUSE_BUTTON_WHEEL_DOWN)
	fail("Expert binding remained outside native Options scroll box: "+action)
	return null
