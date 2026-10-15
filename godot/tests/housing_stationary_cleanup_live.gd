extends "res://tests/housing_cleanup_live.gd"

class PointerProbe extends Node:
	var motions: int = 0
	var last_point := Vector2(-1.0, -1.0)
	func _input(event: InputEvent) -> void:
		if event is InputEventMouseMotion:
			motions += 1
			last_point = event.position

var coord: String
var probe: PointerProbe
var fixed_motions: int
var stationary_receipts: Array[Dictionary] = []

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	coord = OS.get_environment("HOUSING_HOVER_COORD")
	cleanup_role = "visitor"
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0 or not coord.begins_with("/home/osso/.worktrees/housing-core14-evidence/"):
		fail("Stationary Cleanup proof requires explicit private5870 visitor/owner coordination")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	if not await wait_housing(func(): return FileAccess.file_exists(coord.path_join("ready.json")), "real owner peer ready", 120000): return
	var owner: Dictionary = read_peer_json("ready.json")
	if owner.is_empty(): return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	var before: Dictionary = client.housing_state()
	if before.decor.size() != 1 or int(before.decor[0].decor_id) != 1693 or before.can_edit or int(owner.layout.house_id) != editor_house or int(owner.layout.decor[0].instance_id) != int(before.decor[0].instance_id):
		fail("Owner and readonly native visitor must share the real paid instance")
		return
	if not await wait_housing(func(): return placed_node() != null, "replicated paid actor ready"): return
	if not await enter_cleanup(): return
	var instance: int = int(before.decor[0].instance_id)
	var origin: Vector3 = placed_node().global_transform.origin
	var hit: Dictionary = find_visible_cornerstone_triangle(placed_node(), instance)
	if hit.is_empty():
		fail("Stationary proof needs a genuinely visible paid decor triangle")
		return
	probe = PointerProbe.new()
	root.add_child(probe)
	probe.set_process_input(true)
	push_mouse_motion(hit.point)
	if not await wait_housing(func(): return probe.motions > 0 and root.get_mouse_position().distance_to(hit.point) < 0.01 and highlighted_meshes() > 0 and client.tooltip_state().get("title", "") == "Small Val'sharah Bookcase", "initial actual Cleanup hover"): return
	if probe.last_point.distance_to(hit.point) > 0.01:
		fail("Pointer probe did not observe the actual target input")
		return
	if not await record_stationary("stationary-cleanup-initial", hit.point): return
	fixed_motions = probe.motions
	var source_position: Array = owner.layout.decor[0].transform.position
	# This is an owner's test placement input, not an asserted authored room boundary.
	var moved_position: Array = [float(source_position[0]) + 5.0, float(source_position[1]), float(source_position[2])]
	if not write_peer_signal("move.json", {"position": moved_position}): return
	if not await wait_housing(func(): return FileAccess.file_exists(coord.path_join("moved.json")) and placed_node().global_transform.origin.distance_to(origin) > 0.01 and UnitPicker.pick(camera(), hit.point) != instance, "actual replicated movement leaves the stationary ray", 120000): return
	if not assert_stationary("after authoritative movement"): return
	if not await record_stationary("stationary-cleanup-moved-observed", hit.point): return
	if not await wait_housing(func(): return highlighted_meshes() == 0 and client.tooltip_state().get("title", "") != "Small Val'sharah Bookcase", "stationary Cleanup clears moved target", 15000):
		print_cleanup_hover_probe("stationary-moved-timeout", hit.point)
		return
	if not assert_stationary("after moved hover clears"): return
	if not write_peer_signal("restore.request", {}): return
	if not await wait_housing(func(): return FileAccess.file_exists(coord.path_join("restored.json")) and placed_node().global_transform.origin.distance_to(origin) < 0.001 and UnitPicker.pick(camera(), hit.point) == instance and highlighted_meshes() > 0 and client.tooltip_state().get("title", "") == "Small Val'sharah Bookcase", "stationary Cleanup discovers restored target", 120000):
		print_cleanup_hover_probe("stationary-restored-timeout", hit.point)
		return
	if not assert_stationary("after original layout returns"): return
	if client.housing_state().decor != before.decor or client.housing_state().stored != before.stored:
		fail("Stationary Cleanup proof did not restore exact visitor layout/quantities")
		return
	if not await record_stationary("stationary-cleanup-restored", hit.point): return
	print("PASS: private5870 stationary Cleanup follows authoritative movement ", housing_skin)
	client.free()
	quit(0)

func assert_stationary(phase: String) -> bool:
	if probe.motions != fixed_motions:
		fail("Stationary-pointer precondition changed " + phase + ": " + str(probe.motions) + " vs " + str(fixed_motions))
		return false
	return true

func read_peer_json(name: String) -> Dictionary:
	var file := FileAccess.open(coord.path_join(name), FileAccess.READ)
	if file == null:
		fail("Cannot read owner peer snapshot " + name)
		return {}
	var value: Variant = JSON.parse_string(file.get_as_text())
	file.close()
	if not value is Dictionary:
		fail("Owner peer snapshot is not an object: " + name)
		return {}
	return value

func write_peer_signal(name: String, value: Dictionary) -> bool:
	var path: String = coord.path_join(name)
	var temporary: String = path + ".tmp"
	var file := FileAccess.open(temporary, FileAccess.WRITE)
	if file == null:
		fail("Cannot write owner peer signal " + name)
		return false
	file.store_string(JSON.stringify(value))
	if file.get_error() != OK:
		fail("Owner peer signal write failed " + name)
		return false
	file.close()
	if DirAccess.rename_absolute(temporary, path) != OK:
		fail("Cannot publish owner peer signal " + name)
		return false
	return true

func record_stationary(phase: String, point: Vector2) -> bool:
	stationary_receipts.append({"phase": phase, "skin": housing_skin, "pointer_events": probe.motions, "point": str(point), "viewport_mouse": str(root.get_mouse_position()), "picked": UnitPicker.pick(camera(), point), "actor_origin": str(placed_node().global_transform.origin), "highlighted": highlighted_meshes(), "tooltip": client.tooltip_state(), "housing": client.housing_state()})
	var file := FileAccess.open(shots.path_join("stationary-hover-receipts.json"), FileAccess.WRITE)
	if file == null:
		fail("Cannot write stationary hover receipts")
		return false
	file.store_string(JSON.stringify(stationary_receipts, "\t"))
	if file.get_error() != OK:
		fail("Stationary hover receipt write failed")
		return false
	file.close()
	return await capture_housing(phase)
