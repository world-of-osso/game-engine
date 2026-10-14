extends "res://tests/housing_expert_source_live.gd"

func run_test() -> void:
	root.size = Vector2i(1280,720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server!="127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house<=0:
		fail("Tesla Expert proof requires owned private paid-copy configuration")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	var before: Dictionary = client.housing_state()
	if before.decor.size()!=1 or int(before.decor[0].decor_id)!=1693 or housing_count(2330,"stored")!=1:
		fail("Tesla proof requires the existing paid stored Tesla and placed bookcase")
		return
	var bookcase: Dictionary = before.decor[0].duplicate(true)
	if not await normal_click_placement(before): return
	var initial: Dictionary = tesla_row().duplicate(true)
	if not await configure_expert_bindings(false): return
	if not await enter_expert(): return
	if not await select_tesla(): return
	if not await stage_assigned_precision(): return
	if not saved_matches(initial) or current_bookcase()!=bookcase:
		fail("Tesla staging mutated confirmed decor")
		return
	await press_escape()
	if not await wait_housing(func(): return tesla_render_matches(initial),"Tesla cancel restores actual saved model"): return
	if not await record_editor("expert-tesla-cancelled"): return
	if not await select_tesla(): return
	if not await stage_assigned_precision(): return
	var expected: Dictionary = client.housing_state().draft.duplicate(true)
	if not await commit_expert_by_world_click(): return
	if not await wait_housing(func(): return not client.housing_state().pending and saved_matches(expected),"paid Tesla sourced click commit",120000): return
	if not await record_editor("expert-tesla-committed"): return
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	if not await wait_housing(func(): return saved_matches(expected) and tesla_render_matches(expected),"Tesla Expert transform survives genuine reauthentication"): return
	if current_bookcase()!=bookcase or housing_count(2330,"stored")!=0 or housing_count(2330,"placed")!=1:
		fail("Tesla Expert changed the other paid instance or total copies")
		return
	if not await record_editor("expert-tesla-relogged"): return
	if not await configure_expert_bindings(true): return
	if not await remove_actual_decor(2330): return
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	if current_bookcase()!=bookcase or client.housing_state().decor.size()!=1 or housing_count(2330,"stored")!=1 or housing_count(2330,"placed")!=0:
		fail("Normal Tesla removal/relog did not conserve the paid copy")
		return
	if not await record_editor("expert-tesla-restored-storage"): return
	print("PASS: private5870 paid Tesla2330 assigned Expert stage/cancel/click-commit/relog/remove conservation ",housing_skin)
	client.free()
	quit(0)

func select_tesla() -> bool:
	var node: Node3D = decor_node(2330)
	if node==null:
		fail("No actual placed Tesla model")
		return false
	var hit: Dictionary = find_visible_cornerstone_triangle(node,int(node.get_meta("housing_instance_id")))
	if hit.is_empty():
		fail("No exposed Tesla triangle for native selection")
		return false
	await click(hit.point,MOUSE_BUTTON_LEFT)
	return await wait_housing(func(): return int(client.housing_state().selected)==int(node.get_meta("housing_instance_id")) and client.housing_state().scale_allowed,"actual M2 Tesla selection permits scaling")

func tesla_row() -> Dictionary:
	for row in client.housing_state().decor:
		if int(row.decor_id)==2330: return row
	return {}

func current_bookcase() -> Dictionary:
	for row in client.housing_state().decor:
		if int(row.decor_id)==1693: return row
	return {}

func saved_matches(expected: Dictionary) -> bool:
	var actual: Dictionary = tesla_row()
	if actual.is_empty() or actual.position.distance_to(expected.position)>0.01: return false
	for field in ["pitch","yaw","roll","scale"]:
		if absf(float(actual[field])-float(expected[field]))>0.01: return false
	return true

func tesla_render_matches(expected: Dictionary) -> bool:
	var node: Node3D = decor_node(2330)
	if node==null or node.position.distance_to(expected.position)>0.01: return false
	return absf(node.rotation.x-float(expected.pitch))<0.01 and absf(node.rotation.y-float(expected.yaw))<0.01 and absf(node.rotation.z-float(expected.roll))<0.01 and absf(node.scale.x-float(expected.scale))<0.01
