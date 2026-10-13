extends "res://tests/housing_storage_drawer_live.gd"

var transition_receipts: Array[Dictionary] = []

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0:
		fail("Slide proof requires explicit private5870/fb_housing_* owned house")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await ready_dashboard(): return
	if client.housing_state().get("house_id", 0) != editor_house or not client.housing_state().get("can_edit", false):
		fail("Slide proof requires the saved owned house")
		return
	if not await click_exposed_control(housing_control("HousingDecorateButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageFrame") != null or housing_control("HouseEditorStorageExpandButton") != null, "cold saved drawer"): return
	var before: Dictionary = client.housing_state()
	var cold: Control = housing_control("HouseEditorStorageFrame")
	if cold == null: cold = housing_control("HouseEditorStorageExpandButton")
	if absf(cold.global_position.x) > 0.1:
		fail("Cold load unexpectedly animated saved preferences")
		return
	if housing_control("HouseEditorStorageExpandButton") != null:
		if not await click_exposed_control(housing_control("HouseEditorStorageExpandButton")): return
		if not await wait_housing(func(): return housing_control("HouseEditorStorageFrame") != null and absf(housing_control("HouseEditorStorageFrame").global_position.x) < 0.1, "initial drawer expansion finished"): return
	if not await type_storage_search("Tesla"): return
	if not await prove_paid_thumbnail_visible(): return
	if not await sample_transition("collapse", "HouseEditorStorageCollapseButton", "HouseEditorStorageExpandButton"): return
	if not await record_drawer("storage-slide-collapsed"): return
	if not await sample_transition("expand", "HouseEditorStorageExpandButton", "HouseEditorStorageFrame"): return
	if storage_search_text() != "Tesla" or not drawer_conserved(before): return
	if not await record_drawer("storage-slide-expanded"): return
	print("PASS: private5870 owned-storage transitions ", housing_skin)
	client.free()
	quit(0)

func sample_transition(phase: String, button: String, final_control: String) -> bool:
	var samples: Array[Dictionary] = []
	var started: int = Time.get_ticks_usec()
	if not await click_exposed_control(housing_control(button)): return false
	var deadline: int = Time.get_ticks_msec() + 2000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var frame: Control = housing_control("HouseEditorStorageFrame")
		var handle: Control = housing_control("HouseEditorStorageExpandButton")
		if frame != null and handle != null:
			fail("Slide phases exposed both source frames simultaneously")
			return false
		var active: Control = frame if frame != null else handle
		if active != null:
			var part: String = "panel" if frame != null else "handle"
			samples.append({"us":Time.get_ticks_usec()-started,"part":part,"x":active.global_position.x})
		var final: Control = housing_control(final_control)
		if final != null and absf(final.global_position.x) < 0.01:
			break
	if not prove_transition_samples(phase, samples): return false
	transition_receipts.append({"phase":phase,"samples":samples,"housing":client.housing_state()})
	var file: FileAccess = FileAccess.open(shots.path_join("transition-receipts.json"),FileAccess.WRITE)
	if file == null:
		fail("Cannot record slide sample receipts")
		return false
	file.store_string(JSON.stringify(transition_receipts,"\t"))
	file.close()
	return true

func prove_transition_samples(phase: String, samples: Array[Dictionary]) -> bool:
	var first: String = "panel" if phase == "collapse" else "handle"
	var second: String = "handle" if phase == "collapse" else "panel"
	var saw_first_motion: bool = false
	var saw_second_motion: bool = false
	var reached_second: bool = false
	var previous_first: float = 0.0
	var previous_second: float = -10000.0
	for sample in samples:
		var x: float = float(sample.x)
		if sample.part == first:
			if reached_second or x > previous_first + 0.1:
				fail("Outgoing source slide was not ordered/monotonic")
				return false
			previous_first = x
			if x < -0.1: saw_first_motion = true
		elif sample.part == second:
			reached_second = true
			if x < previous_second - 0.1:
				fail("Incoming source slide moved away from its final position")
				return false
			previous_second = x
			if x < -0.1: saw_second_motion = true
	if samples.is_empty() or not saw_first_motion or not saw_second_motion or samples[-1].part != second or absf(float(samples[-1].x)) > 0.01:
		fail("Native source slide motion/endpoints were not witnessed: " + phase)
		return false
	var elapsed_ms: float = float(samples[-1].us) / 1000.0
	if elapsed_ms < 300.0 or elapsed_ms > 1000.0:
		fail("Slide wall-clock interval outside source0.35s plus dispatch/frame tolerance: " + str(elapsed_ms))
		return false
	print("STORAGE_SLIDE phase=",phase," samples=",samples.size()," elapsed_ms=",elapsed_ms)
	return true
