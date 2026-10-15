extends "res://tests/housing_editor_live.gd"

var cleanup_role: String

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = OS.get_environment("HOUSING_CLEANUP_ROLE")
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0 or not cleanup_role in ["owner", "visitor"]:
		fail("Cleanup proof requires explicit private5870/fb_housing_* house/role")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	if OS.get_environment("HOUSING_CLEANUP_RESTORE_ONLY") == "1":
		if not await place_owned_bookcase(): return
		print("PASS: private5870 Cleanup paid storage restore ", housing_skin)
		client.free()
		quit(0)
		return
	var before: Dictionary = client.housing_state()
	if before.decor.size() != 1 or int(before.decor[0].decor_id) != 1693:
		fail("Cleanup needs the exact paid bookcase retained by previous live receipts")
		return
	if not await wait_housing(func(): return placed_node() != null, "confirmed Cleanup visual"): return
	if not await enter_cleanup(): return
	var hit: Dictionary = find_visible_cornerstone_triangle(placed_node(), int(before.decor[0].instance_id))
	if hit.is_empty():
		fail("Cleanup target has no real visible pick triangle")
		return
	push_mouse_motion(hit.point)
	if not await wait_housing(func(): return housing_text("HouseEditorCleanupHoverName") == "Small Val'sharah Bookcase" and highlighted_meshes() > 0, "cleanup native hover/highlight"): return
	if client.housing_state().decor != before.decor or client.housing_state().stored != before.stored:
		fail("Cleanup hover changed confirmed decor/storage")
		return
	if not await record_editor("cleanup-hover"): return
	push_mouse_motion(housing_control("HouseEditorBasicDecorModeButton").get_global_rect().get_center())
	if not await wait_housing(func(): return housing_control("HouseEditorCleanupTooltip") == null and highlighted_meshes() == 0, "hover leaves target and restores original material"): return
	print_cleanup_hover_probe("before-return", hit.point)
	push_mouse_motion(hit.point)
	print_cleanup_hover_probe("after-return", hit.point)
	if not await wait_housing(func(): return highlighted_meshes() > 0, "hover returns"):
		print_cleanup_hover_probe("return-timeout", hit.point)
		return
	await click(hit.point, MOUSE_BUTTON_LEFT)
	if cleanup_role == "owner":
		if not await prove_cleanup_owner(before): return
	else:
		if not await prove_cleanup_visitor(before): return
	print("PASS: private5870 Cleanup ", cleanup_role, " ", housing_skin)
	client.free()
	quit(0)

func print_cleanup_hover_probe(phase: String, point: Vector2) -> void:
	print("CLEANUP_HOVER_PROBE phase=", phase, " point=", point, " actual_mouse=", root.get_mouse_position(), " picked=", UnitPicker.pick(camera(), point), " camera=", camera().global_transform, " highlighted=", highlighted_meshes(), " tooltip=", housing_control("HouseEditorCleanupTooltip") != null, " hovered_control=", root.gui_get_hovered_control(), " pending=", client.housing_state().get("pending", true), " house=", client.housing_state().get("house_id", 0))

func push_mouse_motion(point: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	Input.warp_mouse(point)

func ready_dashboard() -> bool:
	print("CLEANUP_DASHBOARD_BEFORE map=", client.account_state().world_map_id, " house=", client.housing_state().get("house_id", 0), " confirmed=", client.housing_state().get("confirmed", false))
	if not await click_exposed_control(housing_named_control("MicroMenuUI", "HousingMicroButton")): return false
	print("CLEANUP_DASHBOARD_CLICK confirmed=", client.housing_state().get("confirmed", false), " dashboard=", client.get_node_or_null("HousingDashboardUI") != null)
	return await wait_housing(func(): return housing_control("HousingDashboardHouseFinderButton") != null and client.housing_state().get("confirmed", false) and not client.housing_state().get("pending", true), "ready Cleanup dashboard")

func open_cleanup_house() -> bool:
	if not await ready_dashboard(): return false
	var is_owner: bool = cleanup_role == "owner"
	if client.housing_state().get("house_id", 0) != editor_house:
		var old_hud: int = housing_named_control("MicroMenuUI", "HousingMicroButton").get_instance_id()
		if is_owner:
			if not await wait_housing(func():
				var visit: BaseButton = housing_control("HousingHomeVisit%d" % editor_house) as BaseButton
				return visit != null and not visit.disabled, "confirmed owned home"): return false
			await click_control(housing_control("HousingHomeVisit%d" % editor_house), MOUSE_BUTTON_LEFT)
		else:
			await click_control(housing_control("HousingDashboardHouseFinderButton"), MOUSE_BUTTON_LEFT)
			if not await open_friend_plot_info(editor_house): return false
			await click_control(housing_control("HouseFinderPlotVisitButton"), MOUSE_BUTTON_LEFT)
		if not await wait_housing(func():
			var hud: Control = housing_named_control("MicroMenuUI", "HousingMicroButton")
			return client.account_state().screen == "InWorld" and int(client.account_state().world_map_id) == 2783 and hud != null and hud.get_instance_id() != old_hud, "new Cleanup house-copy HUD", 420000): return false
		await frames(20)
		if not await ready_dashboard(): return false
	if not await wait_housing(func(): return client.housing_state().get("house_id", 0) == editor_house and client.housing_state().get("can_edit", not is_owner) == is_owner and not client.housing_state().get("pending", true), "confirmed Cleanup layout"): return false
	await click_control(housing_control("HousingDecorateButton"), MOUSE_BUTTON_LEFT)
	return await wait_housing(func(): return housing_control("HouseEditorStorageFrame") != null and housing_control("HouseEditorCleanupModeButton") != null, "Cleanup mode entry")

func enter_cleanup() -> bool:
	await frames(5)
	if not await click_exposed_control(housing_control("HouseEditorCleanupModeButton")): return false
	if not await wait_housing(func(): return housing_control("HouseEditorStorageFrame") == null and housing_text("HouseEditorSelectInstructionText") == "Remove Decor", "Cleanup source instructions hide storage"): return false
	var button: Control = housing_control("HouseEditorCleanupModeButton")
	var scale: float = button.get_global_rect().size.x / 64.0
	var bar: Rect2 = housing_control("HouseEditorModeBar").get_global_rect()
	var row: Rect2 = housing_control("HouseEditorSelectInstruction").get_global_rect()
	var instructions: Rect2 = housing_control("HouseEditorInstructions").get_global_rect()
	var customize: BaseButton = housing_control("HouseEditorCustomizeModeButton") as BaseButton
	if absf(bar.size.x-266.0*scale)>1.0 or absf(bar.get_center().x-root.size.x/2.0)>1.0 or absf(bar.end.y-root.size.y)>1.0:
		fail("Cleanup source mode bar bounds/anchor drifted")
		return false
	if absf(row.size.y-45.0*scale)>0.1 or absf(instructions.end.x-(root.size.x-30.0*scale))>1.0 or housing_control("HouseEditorSelectInstructionControlIcon")==null:
		fail("Cleanup source instruction row/glyph drifted")
		return false
	if customize==null or not customize.disabled or housing_control("HouseEditorBasicSubmodeBar")!=null or housing_control("HouseEditorExpertSubmodeBar")!=null:
		fail("Cleanup invented an available unsupported mode/submode")
		return false
	return true

func highlighted_meshes() -> int:
	var node: Node3D = placed_node()
	if node == null: return 0
	var count: int = 0
	for mesh in node.find_children("*", "MeshInstance3D", true, false):
		if mesh.material_overlay != null: count += 1
	return count

func total_bookcases(state: Dictionary) -> int:
	var total: int = 0
	for field in ["stored", "placed"]:
		for entry in state.get(field, []):
			if int(entry.decor_id) == 1693: total += int(entry.quantity)
	return total

func prove_cleanup_owner(before: Dictionary) -> bool:
	if not await wait_housing(func(): return client.housing_state().decor.is_empty() and stored_bookcases() == 1 and not client.housing_state().get("pending", true) and placed_node() == null, "single Cleanup click returns paid copy to storage"): return false
	var removed: Dictionary = client.housing_state()
	if total_bookcases(removed) != total_bookcases(before) or total_bookcases(removed) != 1 or housing_control("HouseEditorStorageFrame") != null or housing_control("HouseEditorCleanupTooltip") != null:
		fail("Cleanup broke conservation/storage-hidden/hover teardown")
		return false
	if not await record_editor("cleanup-stored"): return false
	if not await connect_housing_account(): return false
	if not await ready_dashboard(): return false
	if not await wait_housing(func(): return client.housing_state().get("house_id", 0) == editor_house and client.housing_state().get("decor", []).is_empty() and stored_bookcases() == 1, "Cleanup storage persisted after real relog"): return false
	if total_bookcases(client.housing_state()) != 1:
		fail("Cleanup relog changed paid quantity")
		return false
	if not await record_editor("cleanup-relogged"): return false
	if not await click_exposed_control(housing_control("HousingDecorateButton")): return false
	if not await wait_housing(func(): return housing_control("HouseEditorStored1693") != null, "Basic storage after relog"): return false
	if not await enter_cleanup(): return false
	if not await click_exposed_control(housing_control("HouseEditorBasicDecorModeButton")): return false
	if not await wait_housing(func(): return housing_control("HouseEditorStored1693") != null, "mode exit restores storage"): return false
	if not await place_owned_bookcase(): return false
	if total_bookcases(client.housing_state()) != 1:
		fail("Cleanup storage re-place duplicated paid quantity")
		return false
	return await record_editor("cleanup-restored")

func prove_cleanup_visitor(before: Dictionary) -> bool:
	if not await wait_housing(func(): return client.housing_state().get("error", "").contains("Visitors") or housing_control("HouseEditorCleanupHoverError") != null, "visitor removal refusal"): return false
	await frames(15)
	var after: Dictionary = client.housing_state()
	if after.can_edit or after.decor != before.decor or after.stored != before.stored or after.get("pending", true):
		fail("Cleanup visitor changed owner layout/storage")
		return false
	if not await record_editor("cleanup-refused"): return false
	if not await connect_housing_account(): return false
	if not await ready_dashboard(): return false
	if not await wait_housing(func(): return client.housing_state().get("house_id", 0) == editor_house and not client.housing_state().get("can_edit", true) and client.housing_state().get("decor", []) == before.decor, "visitor refused layout persisted on relog"): return false
	return await record_editor("cleanup-visitor-relogged")
