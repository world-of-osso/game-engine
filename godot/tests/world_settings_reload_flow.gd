extends "res://tests/world_loot_options_flow.gd"

# Two distinct native child processes; the peer retains the owned canonical file.
# The parent deliberately cleans up each child after its verified marker. Neither
# branch exercises or claims normal shutdown, which remains explicitly deferred.
const SAVED_FOV := 105.0
const FOV_EPSILON := 0.001
const SAVED_INVERTED_DELTA := 0.04
const PITCH_EPSILON := 0.00001
const LOOTED_COUNT := 5
const LOOTED_MONEY := 11752

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	var config := OS.get_environment("XDG_CONFIG_HOME")
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	if not config.contains("/data/diagnostics/native-input-") or not endpoint.begins_with("127.0.0.1:") or endpoint.ends_with(":0"):
		fail("Settings reload requires fixture-owned config and loopback peer")
		return
	var phase := FileAccess.get_file_as_string(config.path_join("world-of-osso/settings-reload-phase")).strip_edges()
	if phase not in ["save", "load"]:
		fail("Settings reload phase missing or invalid: " + phase)
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_screen(client, "Loading", 180000):
		return
	print("FIXTURE SETTINGS_LOADING")
	if not await wait_world(client):
		return
	print("FIXTURE SETTINGS_READY")
	if not await wait_inventory(client, INITIAL_COUNT, INITIAL_MONEY):
		return
	if phase == "save":
		if not await save_options(client, config):
			return
		print("FIXTURE SETTINGS_SAVED")
	else:
		if not await prove_loaded_consumers(client):
			return
		print("FIXTURE SETTINGS_LOADED")
	# Keep the successfully checked client alive for explicit owned parent cleanup.
	while true:
		await process_frame

func save_options(client: Node, config: String) -> bool:
	if not await set_auto_loot(client, config, true):
		return false
	if not await save_camera(client):
		return false
	var binding_probe = load("res://tests/target_binding_options_probe.gd").new()
	binding_probe.fixture = self
	if not await binding_probe.capture_binding(client, KEY_T, "F1", "T", "key:KeyT"):
		return false
	var camera := root.get_camera_3d()
	if camera == null or absf(camera.fov - SAVED_FOV) > FOV_EPSILON:
		fail("Authored saved FOV did not reach the first physical camera")
		return false
	print("SETTINGS SAVE consumers FOV=%s TargetSelf=T AutoLoot=true InvertY=true" % camera.fov)
	return true

func save_camera(client: Node) -> bool:
	await tap(KEY_ESCAPE)
	if not await wait_menu(client):
		return false
	await click_menu_action(client, "MenuBtnOptions")
	var menu := client.get_node_or_null("GameMenuUI")
	var tab := menu.find_child("OptionsTabcamera", true, false) as Control
	if tab == null:
		fail("Authored Camera Options tab missing")
		return false
	await click(tab)
	var invert := menu.find_child("ToggleSwitchinvert_yRightHit", true, false) as Control
	var fov := menu.find_child("Sliderfov_degrees", true, false) as Control
	if invert == null or fov == null or not invert.is_visible_in_tree() or not fov.is_visible_in_tree():
		fail("Owned default Camera controls missing or unexpectedly inverted")
		return false
	await click(invert)
	# Original FOV bounds 90..120: midpoint click requests 105, independently.
	await click(fov)
	await click_menu_action(client, "OptionsDoneButton")
	if client.get_node_or_null("GameMenuUI") != null:
		await click_menu_action(client, "MenuBtnResume")
	return await wait_menu_closed(client, null)

func prove_loaded_consumers(client: Node) -> bool:
	var camera := root.get_camera_3d()
	if camera == null or absf(camera.fov - SAVED_FOV) > FOV_EPSILON:
		fail("Fresh process did not load physical camera FOV 105")
		return false
	if not await prove_loaded_binding(client):
		return false
	if not await prove_inverted_look(client):
		return false
	var corpse: Dictionary = await find_corpse(client)
	if corpse.is_empty() or not await wait_corpse_feedback(client, corpse, true):
		return false
	var items_before := chat_count(client, ITEM_LINE)
	var money_before := chat_count(client, MONEY_LINE)
	await corpse_click(corpse.point, false)
	print("FIXTURE SETTINGS_LOOT_CLICKED")
	if not await wait_chat(client, ITEM_LINE, items_before + 1) or not await wait_chat(client, MONEY_LINE, money_before + 1):
		return false
	if not await wait_hidden(client) or not await wait_inventory(client, LOOTED_COUNT, LOOTED_MONEY):
		return false
	print("SETTINGS LOAD consumers FOV=%s TargetSelf=T InvertY=true AutoLoot=true count=%s money=%s" % [camera.fov, LOOTED_COUNT, LOOTED_MONEY])
	return true

func prove_loaded_binding(client: Node) -> bool:
	if client.target_state().target != null:
		fail("Fresh settings consumer started with an unexpected target")
		return false
	await tap(KEY_F1)
	if client.target_state().target != null:
		fail("Fresh process still acts on old TargetSelf F1")
		return false
	await tap(KEY_T)
	var state: Dictionary = client.target_state()
	var player_id = client.account_state().local_player_id
	if state.target != player_id or state.target_name != NAME or state.circle_on != player_id:
		fail("Fresh process did not apply saved TargetSelf T: " + str(state))
		return false
	var ui := client.get_node_or_null("UnitFramesUI")
	var frame := ui.find_child("TargetFrame", true, false) as Control if ui != null else null
	if frame == null or not frame.is_visible_in_tree():
		fail("Fresh TargetSelf T did not render the authored TargetFrame")
		return false
	await tap(KEY_ESCAPE)
	if client.target_state().target != null or client.get_node_or_null("GameMenuUI") != null:
		fail("Fresh TargetSelf cleanup did not clear selection before menu")
		return false
	return true

func prove_inverted_look(client: Node) -> bool:
	var before: float = client.account_state().camera_pitch
	var point := Vector2(640, 450)
	var down := InputEventMouseButton.new()
	down.position = point
	down.global_position = point
	down.button_index = MOUSE_BUTTON_RIGHT
	down.pressed = true
	root.push_input(down, true)
	await process_frame
	var motion := InputEventMouseMotion.new()
	motion.position = point + Vector2(0, 4)
	motion.relative = Vector2(0, 4)
	motion.button_mask = MOUSE_BUTTON_MASK_RIGHT
	root.push_input(motion, true)
	await process_frame
	down.position = motion.position
	down.pressed = false
	root.push_input(down, true)
	await process_frame
	var after: float = client.account_state().camera_pitch
	var delta := after - before
	if absf(delta - SAVED_INVERTED_DELTA) > PITCH_EPSILON:
		fail("Fresh process did not load inverted look: delta=%s expected +0.04" % delta)
		return false
	print("SETTINGS LOAD inverted pitch delta=%s" % delta)
	return true

func tap(key: Key) -> void:
	push_key(key, true)
	await process_frame
	push_key(key, false)
	for frame in range(4):
		await process_frame
