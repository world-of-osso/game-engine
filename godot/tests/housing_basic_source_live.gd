extends "res://tests/housing_storage_tiles_live.gd"

func run_test() -> void:
	root.size = Vector2i(1280,720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house<=0:
		fail("Basic source proof needs private5870/fb_housing_* owned house")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	var before: Dictionary = client.housing_state()
	if before.decor.size()!=1 or int(before.decor[0].decor_id)!=1693 or housing_count(2330,"stored")!=1:
		fail("Basic proof needs actual paid bookcase/Tesla fixture")
		return
	if not await source_mode_states(): return
	var tile: Control = housing_control("HouseEditorStored2330")
	var start: Vector2 = tile.get_global_rect().get_center()
	push_mouse_motion(start)
	await frames(5)
	print("BASIC_DRAG_BEFORE rect=",tile.get_global_rect()," source=",tile.get_instance_id()," hover=",root.gui_get_hovered_control()," mouse=",root.get_mouse_position())
	if not await record_editor("basic-drag-before"): return
	await tile_mouse_button(start,true)
	print("BASIC_DRAG_PRESSED source=",tile.get_instance_id()," current=",housing_control("HouseEditorStored2330").get_instance_id()," mask=",Input.get_mouse_button_mask()," draw=",tile.get_draw_mode())
	await held_motion(start+Vector2(35,0),Vector2(35,0))
	print("BASIC_DRAG_MOVED dragging=",root.gui_is_dragging()," hover=",root.gui_get_hovered_control()," mouse=",root.get_mouse_position()," source=",housing_control("HouseEditorStored2330")," state=",client.housing_state())
	if not await wait_housing(func(): return int(client.housing_state().get("placing",0))==2330,"actual owned storage OnDragStart",10000): return
	if client.housing_state().decor!=before.decor or client.housing_state().stored!=before.stored:
		fail("Held source drag changed confirmed paid copies")
		return
	if not await source_instructions(): return
	if not await record_editor("basic-owned-drag-held"): return
	await press_escape()
	await tile_mouse_button(start+Vector2(35,0),false)
	if not await wait_housing(func(): return housing_control("HouseEditorStored2330")!=null and int(client.housing_state().get("placing",0))==0,"Escape clears owned drag capture"): return
	if not await normal_click_placement(before): return
	if not await remove_actual_decor(2330): return
	if not await remove_actual_decor(1693): return
	if stored_bookcases()!=1 or not client.housing_state().decor.is_empty():
		fail("Normal source removal did not return actual owned bookcase")
		return
	start = housing_control("HouseEditorStored1693").get_global_rect().get_center()
	await tile_mouse_button(start,true)
	await held_motion(start+Vector2(35,0),Vector2(35,0))
	if not await wait_housing(func(): return int(client.housing_state().get("placing",0))==1693,"paid bookcase native drag"): return
	var ground: Vector2 = ground_point(4.0)
	await held_motion(ground,ground-start)
	await frames(8)
	if not client.housing_state().decor.is_empty() or stored_bookcases()!=1:
		fail("Held bookcase drag persisted before release")
		return
	if not await record_editor("basic-bookcase-drag-preview"): return
	await tile_mouse_button(ground,false)
	if not await wait_housing(func(): return stored_bookcases()==0 and client.housing_state().decor.size()==1 and int(client.housing_state().decor[0].decor_id)==1693,"first drag release commits actual bookcase"): return
	if not await record_editor("basic-bookcase-drag-placed"): return
	var saved: Dictionary = client.housing_state()
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	var restored: Dictionary = client.housing_state()
	if restored.decor.size()!=1 or int(restored.decor[0].instance_id)!=int(saved.decor[0].instance_id) or restored.decor[0].position.distance_to(saved.decor[0].position)>0.01 or total_bookcases(restored)!=1 or housing_count(2330,"stored")!=1:
		fail("Basic source drag/click/relog lost or duplicated paid copies")
		return
	if not await record_editor("basic-source-relogged"): return
	print("PASS: private5870 source Basic controls click-versus-owned-drag release ",housing_skin)
	client.free()
	quit(0)

func source_mode_states() -> bool:
	var button: BaseButton = housing_control("HouseEditorExpertDecorModeButton") as BaseButton
	var icon_name: String = "HouseEditorExpertDecorModeButtonIcon"
	var hover_name: String = "HouseEditorExpertDecorModeButtonHover"
	var normal: Rect2 = tile_atlas(icon_name).region
	var scale: float = button.get_global_rect().size.x/64.0
	if absf(button.get_global_rect().size.y-64.0*scale)>0.1:
		fail("Source mode button did not retain64px square")
		return false
	push_mouse_motion(button.get_global_rect().get_center())
	if not await wait_housing(func(): return absf(housing_control(hover_name).modulate.a-0.5)<0.001,"source additive mode hover"): return false
	if not await record_editor("basic-mode-hover"): return false
	await tile_mouse_button(button.get_global_rect().get_center(),true)
	if not await wait_housing(func(): return tile_atlas(icon_name).region!=normal,"source mode held-pressed atlas"): return false
	if not await record_editor("basic-mode-pressed"): return false
	await held_motion(Vector2(700,400),Vector2(700,400)-button.get_global_rect().get_center())
	await tile_mouse_button(Vector2(700,400),false)
	print("MODE_RELEASE_BOUNDARY draw=",button.get_draw_mode()," hover=",root.gui_get_hovered_control()," storage=",housing_control("HouseEditorStored2330")," expert=",housing_control("HouseEditorExpertSubmodeBar")," region=",tile_atlas(icon_name).region," expected=",normal)
	if not await wait_housing(func(): return housing_control("HouseEditorStored2330")!=null and tile_atlas(icon_name).region==normal,"release outside restores source mode without switching"): return false
	var snap: BaseButton = housing_control("HouseEditorSnapButton") as BaseButton
	push_mouse_motion(snap.get_global_rect().get_center())
	await frames(5)
	if not snap.disabled or housing_control("HouseEditorSnapButtonHover").modulate.a!=0.0 or absf(housing_control("HouseEditorSnapButtonIcon").modulate.a-0.5)>0.001:
		fail("Unavailable snap control invented hover/active behavior")
		return false
	return true

func held_motion(point: Vector2,relative: Vector2) -> void:
	var event := InputEventMouseMotion.new()
	event.position = point
	event.global_position = point
	event.relative = relative
	event.button_mask = MOUSE_BUTTON_MASK_LEFT
	Input.parse_input_event(event)
	await frames(8)

func press_escape() -> void:
	var event := InputEventKey.new()
	event.keycode = KEY_ESCAPE
	event.physical_keycode = KEY_ESCAPE
	event.pressed = true
	Input.parse_input_event(event)
	await frames(3)
	event.pressed = false
	Input.parse_input_event(event)
	await frames(3)

func source_instructions() -> bool:
	# Hotkey visual acceptance is blocked on installed70340 atlas-member re-export (agent_5615).
	# Functional labels/overlay checks do not claim member coordinates or pixels.
	var overlay: Control = housing_control("HouseEditorDecorMoveOverlay")
	if overlay==null or overlay.get_global_rect().position.distance_to(Vector2.ZERO)>0.2 or overlay.get_global_rect().size.distance_to(Vector2(root.size))>0.2:
		fail("Source DecorMoveOverlay did not cover the actual viewport")
		return false
	for name: String in ["RotateLeft","RotateRight","Remove","ToggleGrid"]:
		if housing_text("HouseEditor"+name+"InstructionControlText")!="[UNBOUND]":
			fail("Basic instruction invented default key: "+name)
			return false
	if housing_text("HouseEditorCancelInstructionControlText")!="Escape" or housing_control("HouseEditorRotateLeft")!=null or housing_control("HouseEditorDone")!=null:
		fail("Basic did not replace invented text controls with source instructions")
		return false
	for name: String in ["Snap","FreePlace","CustomizeMode"]:
		var button: BaseButton = housing_control("HouseEditor"+name+"Button") as BaseButton
		if button==null or not button.disabled:
			fail("Unavailable source mode is not disabled: "+name)
			return false
	return true

func normal_click_placement(before: Dictionary) -> bool:
	var tile: Control = housing_control("HouseEditorStored2330")
	await tile_mouse_button(tile.get_global_rect().get_center(),true)
	if int(client.housing_state().get("placing",0))!=0:
		fail("Catalog press started click placement before source release")
		return false
	await tile_mouse_button(tile.get_global_rect().get_center(),false)
	if not await wait_housing(func(): return int(client.housing_state().get("placing",0))==2330,"normal released click starts placement"): return false
	if client.housing_state().decor!=before.decor or client.housing_state().stored!=before.stored:
		fail("Original catalog release placed a paid copy")
		return false
	if not await record_editor("basic-click-preview"): return false
	var ground: Vector2 = ground_point(7.0)
	await tile_mouse_button(ground,true)
	if client.housing_state().decor!=before.decor or housing_count(2330,"stored")!=1:
		await tile_mouse_button(ground,false)
		await remove_actual_decor(2330)
		fail("Move overlay persisted placement on press instead of release")
		return false
	await tile_mouse_button(ground,false)
	if not await wait_housing(func(): return client.housing_state().decor.size()==2 and housing_count(2330,"stored")==0,"next overlay release places actual owned Tesla"): return false
	return await record_editor("basic-click-placed")

func decor_node(decor: int) -> Node3D:
	for node in client.get_children():
		if node is Node3D and node.has_meta("housing_instance_id") and int(node.get_meta("housing_decor_id"))==decor:
			return node as Node3D
	return null

func remove_actual_decor(decor: int) -> bool:
	if not await click_exposed_control(housing_control("HouseEditorCleanupModeButton")): return false
	if not await wait_housing(func(): return client.housing_state().get("editor_mode","")=="Cleanup" or housing_control("HouseEditorStorageFrame")==null,"source Cleanup mode"): return false
	var node: Node3D = decor_node(decor)
	if node==null:
		fail("Removal needs actual rendered owned decor"+str(decor))
		return false
	var hit: Dictionary = find_visible_cornerstone_triangle(node,int(node.get_meta("housing_instance_id")))
	if hit.is_empty():
		fail("Removal needs real placed pick triangle"+str(decor))
		return false
	await click(hit.point,MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_count(decor,"stored")==1 and decor_node(decor)==null,"normal removal returns owned copy"+str(decor)): return false
	if not await click_exposed_control(housing_control("HouseEditorBasicDecorModeButton")): return false
	return await wait_housing(func(): return housing_control("HouseEditorStored%d"%decor)!=null,"Basic restores returned owned tile")
