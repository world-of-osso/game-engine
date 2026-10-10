extends SceneTree

# Real private-server quest28766, no injected UI state or fabricated geometry.
const QUEST := 28766
const TITLE := "Beating Them Back!"
var client: Node
var evidence: String
var failed := false

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	evidence = OS.get_environment("QUEST_POI_EVIDENCE")
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if evidence.is_empty() or server.is_empty() or server.ends_with(":5000"):
		fail("Require owned evidence directory and private GODOT_TEST_SERVER")
		return
	DirAccess.make_dir_recursive_absolute(evidence)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error: String = client.connect_account(server, "fb_questpoi", "fbtest", false)
	if not error.is_empty():
		fail(error)
		return
	if not await wait_until(func(): return client.account_state().get("screen", "") == "CharacterSelect", 90000):
		return
	var enter := client.get_node("CharacterSelectUI").find_child("EnterWorld", true, false) as Control
	if enter == null:
		fail("EnterWorld button absent")
		return
	await click(enter.get_global_rect().get_center())
	if not await wait_until(func():
		var state: Dictionary = client.account_state()
		return state.get("screen", "") == "InWorld" and state.get("local_player_position") != null and not state.get("terrain", {}).get("parsed_tiles", []).is_empty(), 180000):
		return
	print("QUEST_POI IN_WORLD ", client.account_state().selected_character_name)
	if not await wait_until(func(): return client.accept_quest_from("Marshal McBride", QUEST) == "", 30000):
		return
	if not await wait_until(func(): return not tracker_quest().is_empty(), 30000):
		return
	var title := client.get_node("ObjectiveTrackerUI").find_child("QuestBlock28766HeaderText", true, false) as Control
	await click(title.get_global_rect().get_center())
	await frames(15)
	if client.objective_tracker_state().get("super_tracked", -1) != QUEST:
		fail("Tracker click did not super-track the real quest")
		return
	await key(KEY_L)
	await frames(10)
	for forever in [false, true]:
		if not await capture_skin(forever, false):
			return
	var ready := FileAccess.open(evidence.path_join("ready-completion"), FileAccess.WRITE)
	ready.store_string("quest28766 accepted and both skins captured")
	ready.close()
	print("QUEST_POI READY_COMPLETION")
	if not await wait_until(func(): return FileAccess.file_exists(evidence.path_join("complete-now")), 120000):
		return
	await key(KEY_M)
	if not await wait_until(func(): return has_pin("QuestTurnIn"), 30000):
		return
	if not client.minimap_state().get("quest_areas", []).is_empty():
		fail("Completed objective area remains on minimap")
		return
	await key(KEY_M)
	for forever in [false, true]:
		if not await capture_skin(forever, true):
			return
	print("QUEST_POI PASS: real acceptance, selected quest, live completion, both skins")
	client.free()
	quit(0)

func tracker_quest() -> Dictionary:
	for quest in client.objective_tracker_state().get("quests", []):
		if quest.quest_id == QUEST:
			return quest
	return {}

func has_pin(kind: String) -> bool:
	for pin in client.world_map_state().get("pins", []):
		if pin.label == TITLE and pin.type == kind:
			return true
	return false

func capture_skin(forever: bool, complete: bool) -> bool:
	var skin := "forever" if forever else "modern"
	var phase := "complete" if complete else "objective"
	var error: String = client.get_node("MinimapUI").buttonfit_reskin(forever)
	if not error.is_empty():
		fail(error)
		return false
	await frames(40)
	var number := client.get_node("ObjectiveTrackerUI").find_child("QuestBlock28766POIButtonNumber", true, false) as Label
	if not complete and (number == null or number.text != "1"):
		fail("Tracker/map number1 missing")
		return false
	var minimap: Dictionary = client.minimap_state()
	if not complete and (minimap.get("quest_areas", []).size() != 1 or minimap.get("quest_area_pixels", 0) <= 0):
		fail("No real minimap blob: " + str(minimap))
		return false
	var icon := client.get_node("MinimapUI").find_child("MinimapQuestObjective28766", true, false) as Control
	if icon == null or not icon.is_visible_in_tree():
		fail("Rendered minimap objective/turn-in icon missing")
		return false
	await capture(skin + "-" + phase + "-minimap.png")
	await key(KEY_M)
	if not await wait_until(func(): return client.world_map_state().get("open", false), 5000):
		return false
	var kind := "QuestTurnIn" if complete else "QuestObjective"
	if not has_pin(kind):
		fail("Expected world map " + kind + ": " + str(client.world_map_state()))
		return false
	var areas: int = client.world_map_state().get("quest_area_count", -1)
	if (complete and areas != 0) or (not complete and areas != 1):
		fail("World map objective-area count " + str(areas))
		return false
	await frames(10)
	await capture(skin + "-" + phase + "-world-map.png")
	await key(KEY_M)
	client.set_minimap_rotation(true)
	await frames(10)
	await capture(skin + "-" + phase + "-minimap-rotated.png")
	client.set_minimap_rotation(false)
	await frames(10)
	return true

func capture(name: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var error := image.save_png(evidence.path_join(name))
	if error != OK:
		fail("PNG save failed: " + name)
	print("QUEST_POI CAPTURE ", name, " ", image.get_size())

func wait_until(predicate: Callable, milliseconds: int) -> bool:
	var deadline := Time.get_ticks_msec() + milliseconds
	while Time.get_ticks_msec() < deadline and not failed:
		if predicate.call():
			return true
		await frames(5)
	fail("Timed out; account=" + str(client.account_state()))
	return false

func frames(count: int) -> void:
	for _frame in range(count):
		await process_frame

func click(position: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = position
	Input.parse_input_event(motion)
	await process_frame
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = position
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = pressed
		Input.parse_input_event(event)
		await process_frame

func key(code: Key) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = code
		event.physical_keycode = code
		event.pressed = pressed
		Input.parse_input_event(event)
		await process_frame
	await frames(8)

func fail(message: String) -> void:
	failed = true
	push_error("QUEST_POI FAIL: " + message)
	if is_instance_valid(client):
		client.free()
	quit(1)
