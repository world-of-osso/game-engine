extends "res://tests/housing_storage_anchor_live.gd"

func run_test() -> void:
	root.size = Vector2i(1280,720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house<=0:
		fail("Placed list needs private5870/fb_housing_* owned house")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	var before: Dictionary = client.housing_state()
	if before.decor.size()!=1 or int(before.decor[0].decor_id)!=1693 or housing_count(2330,"stored")!=1:
		fail("Placed list requires actual paid bookcase/Tesla fixture")
		return
	var instance: int = int(before.decor[0].instance_id)
	if not await click_exposed_control(housing_control("HouseEditorExpertDecorModeButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorPlacedDecorListButton") != null, "Expert source list toggle"): return
	if housing_text("HouseEditorDecorBudgetText")!="— / —":
		fail("Placement budget was inferred instead of remaining unknown")
		return
	if not await click_exposed_control(housing_control("HouseEditorPlacedDecorListButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorPlacedDecorList") != null and housing_control("HouseEditorPlacedDecorEntry%d"%instance) != null, "actual confirmed placed list entry"): return
	if housing_text("HouseEditorPlacedDecorEntry%dName"%instance)!=BOOKCASE_NAME:
		fail("Placed entry lost authenticated name")
		return
	if not await record_editor("placed-list-open"): return
	if not await drag_placed_list_panel(): return
	var entry_name: String = "HouseEditorPlacedDecorEntry%dName"%instance
	push_mouse_motion(housing_control("HouseEditorPlacedDecorEntry%d"%instance).get_global_rect().get_center())
	if not await wait_housing(func(): return placed_row_color(entry_name).is_equal_approx(Color.WHITE), "source hover white row"): return
	if not await record_editor("placed-list-hover"): return
	if not await click_exposed_control(housing_control("HouseEditorPlacedDecorEntry%d"%instance)): return
	if not await wait_housing(func(): return int(client.housing_state().get("selected",0))==instance and housing_control("HouseEditorPlacedDecorList") != null, "row selects real instance without closing list"): return
	push_mouse_motion(Vector2(500,500))
	if not await wait_housing(func(): return placed_row_color(entry_name).is_equal_approx(Color(1.0,0.82,0.0,1.0)), "source selected gold row"): return
	if not await record_editor("placed-list-selected"): return
	if not await click_exposed_control(housing_control("HouseEditorPlacedDecorEntry%d"%instance)): return
	if not await wait_housing(func(): return int(client.housing_state().get("selected",0))==0, "same row deselects"): return
	var row: Control = housing_control("HouseEditorPlacedDecorEntry%d"%instance)
	await click(row.get_global_rect().get_center(),MOUSE_BUTTON_RIGHT)
	if not await wait_housing(func(): return housing_control("HouseEditorPlacedDecorRemove") != null, "released right button opens Remove Decor menu"): return
	var menu: Rect2 = housing_control("HouseEditorPlacedDecorContextMenu").get_global_rect()
	if menu.position.distance_to(row.get_global_rect().get_center()) > 1.0:
		fail("Context menu lost actual released-right pointer position: " + str(menu))
		return
	if housing_text("HouseEditorPlacedDecorRemoveText")!="Remove Decor":
		fail("Context action is not source Remove Decor")
		return
	if not await record_editor("placed-list-context"): return
	if not await click_exposed_control(housing_control("HouseEditorPlacedDecorRemove")): return
	if not await wait_housing(func(): return client.housing_state().decor.is_empty() and stored_bookcases()==1 and not client.housing_state().get("pending",true) and housing_control("HouseEditorPlacedDecorEntry%d"%instance)==null, "normal authoritative removal updates list and paid storage"): return
	if not await record_editor("placed-list-removed"): return
	if not await click_exposed_control(housing_control("HouseEditorPlacedDecorListCloseButton")): return
	if not await click_exposed_control(housing_control("HouseEditorBasicDecorModeButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStored1693") != null and housing_control("HouseEditorPlacedDecorListButton")==null, "mode exit hides list and shows real stored bookcase"): return
	if not await place_owned_bookcase(): return
	if total_bookcases(client.housing_state())!=1 or housing_count(2330,"stored")!=1:
		fail("List removal/normal re-place lost or duplicated paid decor")
		return
	print("PASS: private5870 source Expert PlacedDecorList ",housing_skin)
	client.free()
	quit(0)

func placed_row_color(name: String) -> Color:
	var control: Control = housing_control(name)
	if control == null: return Color.TRANSPARENT
	var labels: Array = control.find_children("*", "Label", true, false)
	if labels.is_empty(): return Color.TRANSPARENT
	return labels[0].get_theme_color("font_color")

func drag_placed_list_panel() -> bool:
	var before: Rect2 = housing_control("HouseEditorPlacedDecorList").get_global_rect()
	var start: Vector2 = housing_control("HouseEditorPlacedDecorListDragBar").get_global_rect().get_center()
	var delta := Vector2(-45.0,30.0)
	var press := InputEventMouseButton.new()
	press.position = start
	press.global_position = start
	press.button_index = MOUSE_BUTTON_LEFT
	press.pressed = true
	press.button_mask = MOUSE_BUTTON_MASK_LEFT
	Input.parse_input_event(press)
	await frames(3)
	var motion := InputEventMouseMotion.new()
	motion.position = start + delta
	motion.global_position = start + delta
	motion.button_mask = MOUSE_BUTTON_MASK_LEFT
	Input.parse_input_event(motion)
	await frames(10)
	press.position = start + delta
	press.global_position = start + delta
	press.pressed = false
	press.button_mask = 0
	Input.parse_input_event(press)
	if not await wait_housing(func(): return housing_control("HouseEditorPlacedDecorList").get_global_rect().position.distance_to(before.position+delta)<0.2, "native source list header drag"): return false
	return await record_editor("placed-list-dragged")
