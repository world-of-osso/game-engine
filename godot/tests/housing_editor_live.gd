extends "res://tests/housing_home_live.gd"

var editor_house: int
var editor_receipts: Array = []

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0:
		fail("Editor proof requires explicit private5870/fb_housing_* house identity")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_editor_home(): return
	if not await editor_configure_bindings(false): return
	if OS.get_environment("HOUSING_EDITOR_RESUME_PLACED") == "1":
		if not await wait_housing(func(): return placed_node() != null, "previous authenticated placement node"): return
		if stored_bookcases() != 0 or client.housing_state().decor.size() != 1 or int(client.housing_state().decor[0].decor_id) != 1693:
			fail("Resume requires the exact paid bookcase already placed by the prior receipt")
			return
		if not await record_editor("resumed"): return
	else:
		if not await place_owned_bookcase(): return
	if not await select_saved_decor(): return
	var before: Dictionary = client.housing_state()
	var moved_point: Vector2 = ground_point(7.0)
	print("HOUSING_EDITOR_MOVE point=", moved_point, " picked=", UnitPicker.pick(camera(), moved_point), " state=", client.housing_state())
	await click(moved_point, MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return client.housing_state().decor.size() == 1 and client.housing_state().decor[0].position.distance_to(before.decor[0].position) > 0.5, "authoritative decor movement"): return
	if not await record_editor("moved"): return
	await editor_key(KEY_F6)
	if not await wait_housing(func(): return absf(client.housing_state().decor[0].yaw - before.decor[0].yaw) > 0.05, "authoritative decor rotation"): return
	if not await record_editor("rotated"): return
	var saved: Dictionary = client.housing_state()
	if not await connect_housing_account(): return
	if int(client.account_state().world_map_id) != 2783:
		fail("Editor relog lost visited house")
		return
	await click_control(housing_named_control("MicroMenuUI", "HousingMicroButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return client.housing_state().get("decor",[]).size() == 1, "persisted saved layout"): return
	var restored: Dictionary = client.housing_state()
	if stored_bookcases() != 0 or restored.decor[0].position.distance_to(saved.decor[0].position) > 0.01 or absf(restored.decor[0].yaw - saved.decor[0].yaw) > 0.001:
		fail("Editor relog changed quantity/position/rotation: " + str(restored))
		return
	if not await record_editor("relogged"): return
	await click_control(housing_control("HousingDecorateButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HouseEditorStorageFrame") != null, "editor after relog"): return
	if not await select_saved_decor(): return
	await editor_key(KEY_F7)
	if not await wait_housing(func(): return client.housing_state().decor.is_empty() and stored_bookcases() == 1, "remove returns the exact owned copy"): return
	if not await record_editor("removed"): return
	if OS.get_environment("HOUSING_EDITOR_KEEP_PLACED") == "1":
		if not await place_owned_bookcase(): return
	if not await editor_configure_bindings(true): return
	print("PASS: private5870 real owned bookcase place/move/rotate/remove and persisted layout ", housing_skin)
	client.free()
	quit(0)

func open_editor_home() -> bool:
	var micro: Control = housing_named_control("MicroMenuUI", "HousingMicroButton")
	print("HOUSING_HOME_OPEN micro=", micro.get_global_rect(), " account_map=", client.account_state().world_map_id, " old_dashboard=", client.get_node_or_null("HousingDashboardUI") != null)
	await click_control(micro, MOUSE_BUTTON_LEFT)
	print("HOUSING_HOME_CLICK dashboard=", client.get_node_or_null("HousingDashboardUI") != null, " confirmed=", client.housing_state().get("confirmed",false), " houses=", client.housing_state().get("houses",[]))
	if not await wait_housing(func(): return housing_control("HousingHomeVisit%d" % editor_house) != null, "owned home"): return false
	if not home_allows_visitors():
		await click_control(housing_control("HousingSettings%d" % editor_house), MOUSE_BUTTON_LEFT)
		if not await wait_housing(func(): return housing_control("HousingSettingsHouseMode") != null, "House Settings"): return false
		await click_control(housing_control("HousingSettingsHouseMode"), MOUSE_BUTTON_LEFT)
		if not await wait_housing(func(): return housing_control("HousingSettingsModeanyone") != null, "access dropdown"): return false
		await click_control(housing_control("HousingSettingsModeanyone"), MOUSE_BUTTON_LEFT)
		await click_control(housing_control("HousingSettingsSave"), MOUSE_BUTTON_LEFT)
		await click_control(housing_named_control("MicroMenuUI", "HousingMicroButton"), MOUSE_BUTTON_LEFT)
		if not await wait_housing(func(): return home_allows_visitors(), "explicit owner visitor permission"): return false
	if int(client.account_state().world_map_id) != 2783 or client.housing_state().get("house_id",0) != editor_house:
		var previous_hud: int = housing_named_control("MicroMenuUI", "HousingMicroButton").get_instance_id()
		await click_control(housing_control("HousingHomeVisit%d" % editor_house), MOUSE_BUTTON_LEFT)
		# Two house copies share map2783. The old map/HUD cannot prove the new
		# world-port finished; wait for its actual HUD generation before querying.
		if not await wait_housing(func():
			var hud: Control = housing_named_control("MicroMenuUI", "HousingMicroButton")
			return client.account_state().screen == "InWorld" and int(client.account_state().world_map_id) == 2783 and hud != null and hud.get_instance_id() != previous_hud, "new house-copy HUD",420000): return false
		# Registry projection needs its initial layout frames before physical input.
		await frames(5)
		await click_control(housing_named_control("MicroMenuUI", "HousingMicroButton"), MOUSE_BUTTON_LEFT)
		await frames(5)
	if not await wait_housing(func(): return client.housing_state().get("house_id",0) == editor_house and client.housing_state().get("can_edit",false), "confirmed owner layout"): return false
	await click_control(housing_control("HousingDecorateButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HouseEditorStorageFrame") != null, "real editor storage"): return false
	return await record_editor("storage")

func home_allows_visitors() -> bool:
	for house in client.housing_state().get("houses", []):
		if int(house.house_id) == editor_house: return house.get("access", {}) is Dictionary and house.access.get("anyone", false) == true
	return false

func stored_bookcases() -> int:
	for decor in client.housing_state().get("stored",[]):
		if int(decor.decor_id) == 1693: return int(decor.quantity)
	return 0

func ground_point(distance: float) -> Vector2:
	var forward: Vector3 = -camera().global_transform.basis.z
	forward.y = 0.0
	var origin: Vector3 = client.account_state().local_player_position
	return camera().unproject_position(origin + forward.normalized() * distance)

func place_owned_bookcase() -> bool:
	if stored_bookcases() != 1 or not client.housing_state().decor.is_empty():
		fail("Needs exactly one genuinely acquired stored bookcase and an empty layout")
		return false
	await click_control(housing_control("HouseEditorStored1693"), MOUSE_BUTTON_LEFT)
	await click(ground_point(4.0), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return stored_bookcases() == 0 and client.housing_state().decor.size() == 1, "paid acquired decor placed from storage"): return false
	if not await wait_housing(func(): return placed_node() != null, "actual decor M2 node"): return false
	var node: Node3D = placed_node()
	if int(node.get_meta("model_file_data_id")) != 1096761 or node.find_children("*", "MeshInstance3D", true, false).is_empty():
		fail("Placed bookcase does not render its authenticated model")
		return false
	return await record_editor("placed")

func placed_node() -> Node3D:
	for node in client.get_children():
		if node is Node3D and node.has_meta("housing_instance_id"): return node as Node3D
	return null

func select_saved_decor() -> bool:
	await frames(15)
	var node: Node3D = placed_node()
	if node == null:
		fail("No rendered saved decor")
		return false
	var hit: Dictionary = find_visible_cornerstone_triangle(node, int(node.get_meta("housing_instance_id")))
	if hit.is_empty():
		fail("Saved decor has no visible native-pick triangle")
		return false
	await click(hit.point,MOUSE_BUTTON_LEFT)
	await frames(5)
	return true

func record_editor(phase: String) -> bool:
	editor_receipts.append({"phase":phase, "skin":housing_skin, "state":client.housing_state(), "account":client.account_state()})
	var file: FileAccess = FileAccess.open(shots.path_join("editor-receipts.json"), FileAccess.WRITE)
	if file == null:
		fail("Cannot record native editor evidence")
		return false
	file.store_string(JSON.stringify(editor_receipts,"\t"))
	file.close()
	return await capture_housing(phase)

func editor_key(code: Key) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.pressed = true
	Input.parse_input_event(event)
	await frames(3)
	event.pressed = false
	Input.parse_input_event(event)
	await frames(3)

func press_editor_escape() -> void:
	await editor_key(KEY_ESCAPE)

# Explicit fixture assignments, not Retail default keys. Restore unbound on exit.
func editor_configure_bindings(clear: bool) -> bool:
	if not OS.get_environment("XDG_CONFIG_HOME").begins_with("/home/osso/.worktrees/"):
		fail("Housing binding fixtures require an owned private config directory")
		return false
	if int(client.housing_state().get("selected",0))!=0 or int(client.housing_state().get("placing",0))!=0:
		await press_editor_escape()
	if client.get_node_or_null("HousingDashboardUI")!=null:
		await press_editor_escape()
		if not await wait_housing(func(): return client.get_node_or_null("HousingDashboardUI")==null,"editor fixture closes housing before Options"): return false
	await press_editor_escape()
	if not await wait_housing(func(): return housing_named_control("GameMenuUI","MenuBtnOptions")!=null,"editor fixture game menu"): return false
	await click_control(housing_named_control("GameMenuUI","MenuBtnOptions"),MOUSE_BUTTON_LEFT)
	await click_control(housing_named_control("GameMenuUI","OptionsTabkeybindings"),MOUSE_BUTTON_LEFT)
	await click_control(housing_named_control("GameMenuUI","KeybindingSectionhousingButton"),MOUSE_BUTTON_LEFT)
	for binding in [["HOUSING_BASICDECOR_ROTATERIGHT",KEY_F6],["HOUSING_REMOVEDECOR",KEY_F7]]:
		var action: String = binding[0]
		var control: Control = housing_named_control("GameMenuUI","KeybindingButton"+action)
		if control==null:
			fail("Missing source housing binding action: "+action)
			return false
		if clear:
			await click_control(control,MOUSE_BUTTON_RIGHT)
		else:
			if str(client.get_node("GameMenuUI").frame_text("KeybindingButtonText"+action))!="Not Bound":
				fail("Fixture cannot overwrite an existing player housing assignment")
				return false
			await click_control(control,MOUSE_BUTTON_LEFT)
			await editor_key(binding[1])
	await click_control(housing_named_control("GameMenuUI","OptionsDoneButton"),MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return client.get_node_or_null("GameMenuUI")==null,"editor fixture Options Done"): return false
	await click_control(housing_named_control("MicroMenuUI","HousingMicroButton"),MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HousingDecorateButton")!=null and not client.housing_state().get("pending",true),"editor fixture confirmed visited layout"): return false
	await click_control(housing_control("HousingDecorateButton"),MOUSE_BUTTON_LEFT)
	return await wait_housing(func(): return housing_control("HouseEditorBasicDecorModeButton")!=null,"editor fixture Basic mode restored")

# Retail discovery comes from the server's actual persisted friend relationship.
func friend_plot_control(house: int) -> Control:
	for plot in client.housing_state().get("plots", []):
		if int(plot.house_id) == house and int(plot.owner_type) == 2:
			return housing_control("HouseFinderFriendsPlotPin%d" % int(plot.plot_id))
	return null

func open_friend_plot_info(house: int) -> bool:
	if not await wait_housing(func():
		var pin: BaseButton = friend_plot_control(house) as BaseButton
		return pin != null and not pin.disabled, "authenticated Retail friend plot pin"): return false
	if not await click_exposed_control(friend_plot_control(house)): return false
	return await wait_housing(func():
		var visit: BaseButton = housing_control("HouseFinderPlotVisitButton") as BaseButton
		return visit != null and not visit.disabled, "source plot info visit control")
