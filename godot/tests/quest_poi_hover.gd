extends SceneTree

# Real private-server quest; input events drive hover, never injected UI state.
const QUEST := 28766
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
	var account := OS.get_environment("QUEST_POI_ACCOUNT")
	if evidence.is_empty() or server.is_empty() or server.ends_with(":5000") or not account.begins_with("fb_questpoi"):
		fail("Require private server, owned evidence, disposable account")
		return
	DirAccess.make_dir_recursive_absolute(evidence)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error: String = client.connect_account(server, account, "fbtest", false)
	if not error.is_empty():
		fail(error)
		return
	if not await wait_until(func():
		var ui := client.get_node_or_null("CharacterSelectUI")
		return client.account_state().get("screen", "") == "CharacterSelect" and ui != null and ui.find_child("EnterWorld", true, false) is Control, 90000):
		return
	var enter := client.get_node("CharacterSelectUI").find_child("EnterWorld", true, false) as Control
	await click(enter.get_global_rect().get_center())
	if not await wait_until(func():
		return client.account_state().get("screen", "") == "InWorld" and not client.account_state().get("terrain", {}).get("parsed_tiles", []).is_empty(), 180000):
		return
	if not await wait_until(func(): return client.accept_quest_from("Marshal McBride", QUEST) == "", 30000):
		return
	if not await wait_until(func(): return client.get_node_or_null("ObjectiveTrackerUI") != null and client.get_node("ObjectiveTrackerUI").find_child("QuestBlock28766HeaderText", true, false) is Control, 30000):
		return
	for forever in [false, true]:
		if not await capture_skin(forever):
			return
	print("QUEST_POI_HOVER PASS: log/tracker hover and leave, unchanged super-tracking, watched minimap set, both skins")
	client.free()
	quit(0)

func capture_skin(forever: bool) -> bool:
	var skin := "forever" if forever else "modern"
	var error: String = client.get_node("MinimapUI").buttonfit_reskin(forever)
	if not error.is_empty():
		fail(error)
		return false
	await motion(Vector2(1200, 950))
	await frames(40)
	if client.objective_tracker_state().get("super_tracked", -1) != 0:
		fail("Fixture requires no super-tracked quest")
		return false
	var minimap: Dictionary = client.minimap_state()
	if minimap.get("quest_areas", []).size() != 1 or minimap.get("quest_area_pixels", 0) <= 0:
		fail("Watched, not super-tracked minimap quest set absent: " + str(minimap))
		return false
	print("QUEST_POI_HOVER MINIMAP_SET ", skin, " watched28766=1 supertracked=0")
	await capture("v3-" + skin + "-minimap-watched.png")
	await key(KEY_M)
	if not await wait_until(func(): return client.world_map_state().get("open", false), 5000):
		return false
	await motion(Vector2(1200, 950))
	await frames(15)
	if client.world_map_state().get("quest_area_count", -1) != 0:
		fail("Unselected world-map blob present")
		return false
	await capture("v3-" + skin + "-world-map-unhovered.png")
	var title := client.get_node("ObjectiveTrackerUI").find_child("QuestBlock28766HeaderText", true, false) as Control
	await motion(title.get_global_rect().get_center())
	if not await assert_hover(skin + " tracker"):
		return false
	await capture("v3-" + skin + "-world-map-tracker-hover.png")
	await motion(Vector2(1200, 950))
	await frames(15)
	if client.world_map_state().get("quest_area_count", -1) != 0:
		fail("Tracker leave did not clear blob")
		return false
	var log_title := client.get_node("WorldMapUI").find_child("QuestLogTitle28766Text", true, false) as Control
	if log_title == null:
		fail("Docked quest title absent")
		return false
	await motion(log_title.get_global_rect().get_center())
	if not await assert_hover(skin + " quest log"):
		return false
	await capture("v3-" + skin + "-world-map-log-hover.png")
	await motion(Vector2(1200, 950))
	await frames(15)
	if client.world_map_state().get("quest_area_count", -1) != 0:
		fail("Quest log leave did not clear blob")
		return false
	await key(KEY_M)
	return true

func assert_hover(source: String) -> bool:
	if not await wait_until(func(): return client.world_map_state().get("highlighted_quest", 0) == QUEST, 5000):
		return false
	var map: Dictionary = client.world_map_state()
	if map.get("quest_area_count", -1) != 1 or client.objective_tracker_state().get("super_tracked", -1) != 0:
		fail(source + " changed tracking or omitted area: " + str(map))
		return false
	var pins: Array = map.get("pins", [])
	for index in range(pins.size()):
		if pins[index].get("quest_id", 0) == QUEST:
			var glow := client.get_node("WorldMapUI").find_child("WorldMapPin%dHighlight" % index, true, false) as Control
			if glow == null or not glow.is_visible_in_tree():
				fail(source + " missing rendered button inner glow")
				return false
			print("QUEST_POI_HOVER ACTIVE ", source, " area=1 rendered_glow=1 supertracked=0")
			await frames(10)
			return true
	fail("Quest pin absent")
	return false

func capture(name: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	if image.save_png(evidence.path_join(name)) != OK:
		fail("PNG save failed: " + name)
	print("QUEST_POI_HOVER CAPTURE ", name, " ", image.get_size())

func motion(position: Vector2) -> void:
	var event := InputEventMouseMotion.new()
	event.position = position
	Input.parse_input_event(event)
	await process_frame

func click(position: Vector2) -> void:
	await motion(position)
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

func fail(message: String) -> void:
	failed = true
	push_error("QUEST_POI_HOVER FAIL: " + message)
	if is_instance_valid(client):
		client.free()
	quit(1)
