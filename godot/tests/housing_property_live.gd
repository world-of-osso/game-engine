extends "res://tests/housing_collection_live.gd"

# Native UI / world pointer proof on private5870. No item/decor/house grants.
# The external runner moves the actor to the DB2 plot teleport only after entry proof.
var property_entry: int
var property_plot: int
var property_receipts: Array = []

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	property_entry = int(OS.get_environment("HOUSING_CORNERSTONE_ENTRY"))
	property_plot = int(OS.get_environment("HOUSING_PLOT_ID"))
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or property_entry <= 0 or property_plot <= 0:
		fail("Property proof needs explicit private5870/fb_housing_* plot configuration")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if OS.get_environment("HOUSING_RELOG_ONLY") != "1":
		if not await enter_neighborhood(): return
		if not await purchase_property(): return
	if not await inspect_owned_property("owned"): return
	if not await connect_housing_account(): return
	if int(client.account_state().world_map_id) != 2735:
		fail("Relog lost persisted neighborhood identity: " + str(client.account_state()))
		return
	if not await inspect_owned_property("relogged"): return
	print("PASS: private5870 authored neighborhood, physical cornerstone, paid ownership and relog ", housing_skin)
	client.free()
	quit(0)

func enter_neighborhood() -> bool:
	await click_control(housing_named_control("MicroMenuUI", "HousingMicroButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HousingDashboardHouseFinderButton") != null and housing_text("HouseInfoContentUnavailable").contains("account-owned plot(s)"), "confirmed house finder entry"): return false
	await click_control(housing_control("HousingDashboardHouseFinderButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func():
		var visit: BaseButton = housing_control("HouseFinderVisitButton") as BaseButton
		return visit != null and not visit.disabled, "confirmed native finder"): return false
	await frames(20)
	if not await capture_housing("finder"): return false
	await click_control(housing_control("HouseFinderVisitButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return client.account_state().screen == "InWorld" and int(client.account_state().world_map_id) == 2735, "actual neighborhood admission", 420000): return false
	await frames(90)
	property_receipts.append({"phase":"entered", "skin":housing_skin, "state":client.account_state()})
	if not await capture_housing("neighborhood-entry"): return false
	var marker: FileAccess = FileAccess.open(shots.path_join("entered.marker"),FileAccess.WRITE)
	if marker == null:
		fail("Cannot publish private entry barrier")
		return false
	marker.store_string("actual2735 admission captured; external DB2 teleport priming may proceed")
	marker.close()
	return true

func purchase_property() -> bool:
	var stone: Dictionary = await find_cornerstone()
	if stone.is_empty(): return false
	await click(stone.point,MOUSE_BUTTON_RIGHT)
	if not await wait_housing(func(): return housing_control("HousingCornerstoneBuyButton") != null, "authenticated cornerstone reply",180000): return false
	if not await capture_housing("cornerstone"): return false
	var before_money: int = int(client.merchant_state().money)
	await click_control(housing_control("HousingCornerstoneBuyButton"),MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_named_control("StaticPopupUI","StaticPopup1Button1") != null, "plot purchase confirmation"): return false
	if not await capture_housing("purchase-confirm"): return false
	await click_control(housing_named_control("StaticPopupUI","StaticPopup1Button1"),MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_text("HousingCornerstoneStatus").begins_with("Plot purchased.") and int(client.merchant_state().money) == before_money - 10000000, "authoritative paid plot ownership"): return false
	property_receipts.append({"phase":"purchased","skin":housing_skin,"plot_id":property_plot,"entry":property_entry,"money_before":before_money,"money_after":client.merchant_state().money,"status":housing_text("HousingCornerstoneStatus"),"state":client.account_state()})
	if not await capture_housing("purchased"): return false
	await click_control(housing_control("HousingCornerstonePurchaseFrameCloseButton"),MOUSE_BUTTON_LEFT)
	return true

func find_cornerstone() -> Dictionary:
	var deadline: int = Time.get_ticks_msec() + 180000
	var turned: int = 0
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if camera() == null: continue
		for node in client.get_children():
			if not node is Node3D or not node.has_meta("game_object_entry") or int(node.get_meta("game_object_entry")) != property_entry: continue
			var area: Area3D = node.find_child("UnitPick",true,false) as Area3D
			if area == null: continue
			var hit: Dictionary = find_visible_cornerstone_triangle(node, area.get_meta("unit_server_id"))
			if not hit.is_empty(): return hit
		if turned < 60:
			push_key(KEY_RIGHT,true)
			await frames(3)
			push_key(KEY_RIGHT,false)
			turned += 1
	fail("Authentic cornerstone is not visible and unoccluded: " + str(client.account_state()))
	return {}

# The authenticated model's header includes the tall plot beam. Its box centre
# is empty space, not a visible triangle; use real mesh triangles and the native ray.
func find_visible_cornerstone_triangle(node: Node3D, id: int) -> Dictionary:
	for mesh_node in node.find_children("*", "MeshInstance3D", true, false):
		var mesh: MeshInstance3D = mesh_node as MeshInstance3D
		if not mesh.is_visible_in_tree() or mesh.mesh == null: continue
		for surface in range(mesh.mesh.get_surface_count()):
			var arrays: Array = mesh.mesh.surface_get_arrays(surface)
			var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
			var indices: PackedInt32Array = arrays[Mesh.ARRAY_INDEX]
			for triangle in range(0, indices.size(), 3):
				var centre: Vector3 = (vertices[indices[triangle]] + vertices[indices[triangle + 1]] + vertices[indices[triangle + 2]]) / 3.0
				var world_point: Vector3 = mesh.global_transform * centre
				if not camera().is_position_in_frustum(world_point): continue
				var point: Vector2 = camera().unproject_position(world_point)
				if UnitPicker.pick(camera(), point) == id:
					return {"id": id, "entry": property_entry, "point": point}
	return {}

func inspect_owned_property(phase: String) -> bool:
	await click_control(housing_named_control("MicroMenuUI","HousingMicroButton"),MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_text("HouseInfoContentUnavailable").contains("1 account-owned plot(s)"), "durable owned plot"): return false
	property_receipts.append({"phase":phase,"skin":housing_skin,"status":housing_text("HouseInfoContentUnavailable"),"state":client.account_state()})
	var file: FileAccess = FileAccess.open(shots.path_join("property-receipts.json"),FileAccess.WRITE)
	if file == null:
		fail("Cannot write property receipts")
		return false
	file.store_string(JSON.stringify(property_receipts,"\t"))
	if not await capture_housing(phase): return false
	await click_control(housing_control("HousingDashboardFrameCloseButton"),MOUSE_BUTTON_LEFT)
	return true
