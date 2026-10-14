extends "res://tests/housing_editor_live.gd"

# Two clients, two accounts, one house copy. The owner edits; the other-account
# observer sends nothing after admission and must render each saved step from the
# server's unsolicited layout publication. Steps hand off through HOUSING_SYNC files.
const STEPS: Array = ["moved", "rotated", "removed", "replaced"]
const STEP_TIMEOUT_MS: int = 900000

var role: String
var sync_dir: String

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	sync_dir = OS.get_environment("HOUSING_SYNC")
	role = OS.get_environment("HOUSING_ROLE")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or sync_dir.is_empty() or editor_house <= 0 or not role in ["owner", "observer"]:
		fail("Observer proof requires private5870/fb_housing_* account, house, role and sync directory")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	DirAccess.make_dir_recursive_absolute(sync_dir)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	var passed: bool
	if role == "owner":
		passed = await run_owner()
	else:
		passed = await run_observer()
	if not passed: return
	print("PASS: private5870 live ", role, " of house ", editor_house, " ", housing_skin)
	client.free()
	quit(0)

func run_owner() -> bool:
	if not await open_editor_home(): return false
	if not await editor_configure_bindings(false): return false
	if client.housing_state().decor.size() != 1 or int(client.housing_state().decor[0].decor_id) != 1693 or stored_bookcases() != 0:
		fail("Owner needs exactly its previously placed paid bookcase: " + str(client.housing_state()))
		return false
	write_sync("owner-ready", layout_record())
	if not await wait_sync("observer-ready"): return false
	if not await select_saved_decor(): return false
	var before: Dictionary = client.housing_state()
	await click(ground_point(7.0), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return client.housing_state().decor.size() == 1 and client.housing_state().decor[0].position.distance_to(before.decor[0].position) > 0.5, "owner move"): return false
	if not await owner_step("moved"): return false
	await editor_key(KEY_F6)
	if not await wait_housing(func(): return absf(client.housing_state().decor[0].yaw - before.decor[0].yaw) > 0.05, "owner rotation"): return false
	if not await owner_step("rotated"): return false
	await editor_key(KEY_F7)
	if not await wait_housing(func(): return client.housing_state().decor.is_empty() and stored_bookcases() == 1, "owner removal"): return false
	if not await owner_step("removed"): return false
	# Re-place the same owned copy so the house keeps its retained bookcase.
	if not await place_owned_bookcase(): return false
	if not await owner_step("replaced"): return false
	return await editor_configure_bindings(true)

func owner_step(phase: String) -> bool:
	if not await record_editor("owner-" + phase): return false
	write_sync("step-" + phase, layout_record())
	return await wait_sync("ack-" + phase)

func run_observer() -> bool:
	if not await wait_sync("owner-ready"): return false
	await click_control(housing_named_control("MicroMenuUI", "HousingMicroButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HousingDashboardHouseFinderButton") != null and client.housing_state().get("confirmed",false), "authoritative dashboard"): return false
	await click_control(housing_control("HousingDashboardHouseFinderButton"), MOUSE_BUTTON_LEFT)
	if not await open_friend_plot_info(editor_house): return false
	await click_control(housing_control("HouseFinderPlotVisitButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return client.account_state().screen == "InWorld" and int(client.account_state().world_map_id) == 2783 and housing_named_control("MicroMenuUI", "HousingMicroButton") != null, "visitor admission", 420000): return false
	await frames(30)
	await click_control(housing_named_control("MicroMenuUI", "HousingMicroButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return client.get_node_or_null("HousingDashboardUI") != null and client.housing_state().get("house_id",0) == editor_house and client.housing_state().get("can_edit",true) == false and not client.housing_state().get("pending",true), "read-only visited layout"): return false
	await click_control(housing_named_control("MicroMenuUI", "HousingMicroButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return client.get_node_or_null("HousingDashboardUI") == null, "dashboard closed"): return false
	var ready: Dictionary = read_sync("owner-ready")
	if not await wait_rendered(ready.decor, "owner's saved layout"): return false
	if not await frame_owner_and_decor(as_vector(ready.player)): return false
	if not await record_editor("observer-ready"): return false
	write_sync("observer-ready", layout_record())
	# No input from here on: every change must arrive unrequested.
	for phase in STEPS:
		if not await wait_sync("step-" + phase): return false
		var expected: Array = read_sync("step-" + phase).decor
		if not await wait_rendered(expected, "live " + phase): return false
		if client.housing_state().get("can_edit",true) or client.housing_state().get("pending",false):
			fail("Observer became editable or sent a request: " + str(client.housing_state()))
			return false
		if not await record_editor("observer-" + phase): return false
		write_sync("ack-" + phase, layout_record())
	return true

# Both characters arrive on the authored entry point beside the decor. Strafe Right
# leaves the decor, then Turn Right until owner, decor and observer sit apart on screen.
func frame_owner_and_decor(owner: Vector3) -> bool:
	var start: Vector3 = client.account_state().local_player_position
	push_key(KEY_D, true)
	var moved: bool = await wait_housing(func(): return client.account_state().local_player_position.distance_to(start) >= 5.0, "observer strafe", 5000)
	push_key(KEY_D, false)
	if not moved: return false
	await frames(30)
	for turn in range(160):
		if framed(owner):
			await frames(30)
			if framed(owner): return true
		push_key(KEY_RIGHT, true)
		await frames(2)
		push_key(KEY_RIGHT, false)
		await frames(2)
	fail("Observer view never separated owner and decor: " + str(client.account_state()))
	return false

func framed(owner: Vector3) -> bool:
	var decor: Node3D = placed_node()
	if decor == null: return false
	var view: Camera3D = camera()
	var xs: Array = []
	for point in [owner, decor.global_position, client.account_state().local_player_position]:
		if view.is_position_behind(point): return false
		var screen: Vector2 = view.unproject_position(point)
		if screen.x < 120.0 or screen.x > root.size.x - 120.0: return false
		xs.append(screen.x)
	# Owner and decor stand 4 m apart: ~125 px at most from the 15 m camera.
	return absf(xs[0] - xs[1]) > 90.0 and absf(xs[2] - xs[1]) > 150.0

# Authoritative layout and rendered decor nodes both equal the owner's saved step.
func wait_rendered(expected: Array, label: String) -> bool:
	return await wait_housing(func(): return same_layout(client.housing_state().get("decor",[]), expected) and same_nodes(expected), label, STEP_TIMEOUT_MS)

func same_layout(actual: Array, expected: Array) -> bool:
	if actual.size() != expected.size(): return false
	for decor in expected:
		var found: bool = false
		for row in actual:
			if int(row.instance_id) == int(decor.instance_id) and int(row.decor_id) == int(decor.decor_id) and row.position.distance_to(as_vector(decor.position)) < 0.01 and absf(float(row.yaw) - float(decor.yaw)) < 0.001 and absf(float(row.pitch) - float(decor.pitch)) < 0.001 and absf(float(row.roll) - float(decor.roll)) < 0.001 and absf(float(row.scale) - float(decor.scale)) < 0.001:
				found = true
		if not found: return false
	return true

func same_nodes(expected: Array) -> bool:
	var nodes: Dictionary = {}
	for node in client.get_children():
		if node is Node3D and node.has_meta("housing_instance_id"):
			nodes[int(node.get_meta("housing_instance_id"))] = node
	if nodes.size() != expected.size(): return false
	for decor in expected:
		var node: Node3D = nodes.get(int(decor.instance_id))
		if node == null or node.position.distance_to(as_vector(decor.position)) > 0.01 or absf(node.rotation.y - float(decor.yaw)) > 0.001 or absf(node.rotation.x - float(decor.pitch)) > 0.001 or absf(node.rotation.z - float(decor.roll)) > 0.001 or absf(node.scale.x - float(decor.scale)) > 0.001: return false
		if node.find_children("*", "MeshInstance3D", true, false).is_empty(): return false
	return true

func layout_record() -> Dictionary:
	var decor: Array = []
	for row in client.housing_state().get("decor",[]):
		decor.append({"instance_id": int(row.instance_id), "decor_id": int(row.decor_id), "position": [row.position.x, row.position.y, row.position.z], "yaw": float(row.yaw), "pitch": float(row.pitch), "roll": float(row.roll), "scale": float(row.scale)})
	var player: Vector3 = client.account_state().local_player_position
	return {"role": role, "account": housing_account, "skin": housing_skin, "house_id": editor_house, "decor": decor, "player": [player.x, player.y, player.z], "msec": Time.get_ticks_msec(), "unix": Time.get_unix_time_from_system()}

func as_vector(value: Array) -> Vector3:
	return Vector3(float(value[0]), float(value[1]), float(value[2]))

func write_sync(name: String, record: Dictionary) -> void:
	var partial: String = sync_dir.path_join(name + ".partial")
	var file: FileAccess = FileAccess.open(partial, FileAccess.WRITE)
	file.store_string(JSON.stringify(record, "\t"))
	file.close()
	DirAccess.rename_absolute(partial, sync_dir.path_join(name + ".json"))

func read_sync(name: String) -> Dictionary:
	return JSON.parse_string(FileAccess.get_file_as_string(sync_dir.path_join(name + ".json")))

func wait_sync(name: String) -> bool:
	return await wait_housing(func(): return FileAccess.file_exists(sync_dir.path_join(name + ".json")), "peer " + name, STEP_TIMEOUT_MS)
