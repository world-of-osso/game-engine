extends "res://tests/world_soft_interact_live.gd"

# Live soft interact game object icon against a private server, starting from Retail
# defaults (fresh options): Controls "Enable Interact Key" is turned on in the Options UI,
# turning in place until the named object stands directly in front makes it the soft
# interact target with no icon (Interact Key Icons "NPCs Only (Default)"); choosing
# Accessibility > Interact Key Icons > "Show All" puts its cursor icon (Mail over a
# mailbox, the Interact gears over a chair) on top of its model.
# Environment:
#   GODOT_TEST_SERVER      private server address (never 127.0.0.1:5000)
#   SOFT_INTERACT_ACCOUNT / SOFT_INTERACT_CHARACTER  account (password fbtest) and character
#   SOFT_INTERACT_OBJECT / SOFT_INTERACT_ICON        object name and expected icon (Mailbox / Mail)
#   SOFT_INTERACT_SHOTS    screenshot directory
#   XDG_CONFIG_HOME        fresh, with no options file
# Setup (game-server-admin, character offline): set-position <character> -9455.5 49.0 57.0,
# 3 yd north of the Goldshire Mailbox (world.db gameobject guid 26784).

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("SOFT_INTERACT_ACCOUNT")
	var character := OS.get_environment("SOFT_INTERACT_CHARACTER")
	var object := OS.get_environment("SOFT_INTERACT_OBJECT")
	var icon := OS.get_environment("SOFT_INTERACT_ICON")
	shots = OS.get_environment("SOFT_INTERACT_SHOTS")
	if server.is_empty() or server == "127.0.0.1:5000" or account.is_empty() or character.is_empty() or object.is_empty() or icon.is_empty() or shots.is_empty():
		fail("Needs a private GODOT_TEST_SERVER, SOFT_INTERACT_ACCOUNT/CHARACTER/OBJECT/ICON and SOFT_INTERACT_SHOTS")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Live connection: " + error)
		return
	if not await enter_as(character):
		return
	if not await choose_option("hud", "ToggleSwitchsoft_target_interactRightHit", ""):
		return
	var id := await face_soft_object(object)
	if id < 0:
		return
	var state: Dictionary = client.soft_interact_state()
	if state.has("icon") or state.has("icon_rect"):
		fail("Default Interact Key Icons show an icon over %s: %s" % [object, state])
		return
	print("FIXTURE DEFAULT_ICONS ", object, " ", state)
	await shot("%s-default-no-icon.png" % icon.to_lower())
	if not await choose_option("accessibility", "Choiceinteract_key_icons2Hit", "options-show-all.png"):
		return
	await frames(5)
	state = client.soft_interact_state()
	if state.get("target") != id or state.get("icon") != icon or not state.has("icon_rect"):
		fail("Show All: %s shows no %s icon: %s" % [object, icon, state])
		return
	var top := camera().unproject_position(object_top(id))
	var rect: Rect2 = state.icon_rect
	var bottom := Vector2(rect.get_center().x, rect.end.y)
	if rect.size != Vector2(32, 32) or bottom.distance_to(top) > 1.0:
		fail("%s icon %s is not a 32 px icon standing on the model top %s" % [object, rect, top])
		return
	print("FIXTURE SOFT_TARGET_OBJECT ", object, " ", id, " ", state, " model_top_screen ", top)
	await shot("%s-show-all-icon.png" % icon.to_lower())
	print("FIXTURE SOFT_INTERACT_OBJECT_LIVE_DONE")
	client.free()
	quit(0)

# Open Options on `category`, click the authored control `name`, optionally capture the
# panel, and close it with Done.
func choose_option(category: String, name: String, capture_file: String) -> bool:
	# The first Escape clears a hard target when there is one.
	for attempt in 2:
		if menu_open():
			break
		await tap(KEY_ESCAPE)
		await frames(3)
	for control_name in ["MenuBtnOptions", "OptionsTab" + category, name]:
		if not await click_menu_control(control_name):
			return false
		await frames(3)
	if capture_file != "":
		await shot(capture_file)
	if not await click_menu_control("OptionsDoneButton"):
		return false
	await frames(3)
	if menu_open():
		await tap(KEY_ESCAPE)
		await frames(3)
	if menu_open():
		fail("Game menu still open after Done")
		return false
	return true

func menu_open() -> bool:
	return client.get_node_or_null("GameMenuUI") != null

func click_menu_control(name: String) -> bool:
	var menu := client.get_node_or_null("GameMenuUI")
	var control = menu.find_child(name, true, false) if menu != null else null
	if not control is Control:
		fail("Authored Options control absent: " + name)
		return false
	await click_control(control, MOUSE_BUTTON_LEFT)
	return true

# The nearest shown game object called `name`: its node.
func object_node(name: String) -> Node3D:
	var best: Node3D = null
	var player: Vector3 = client.account_state().local_player_position
	for node in client.find_children("Mailbox_*", "Node3D", true, false):
		if node.get_meta("game_object_name", "") != name:
			continue
		if best == null or node.global_position.distance_to(player) < best.global_position.distance_to(player):
			best = node
	return best

func object_top(id: int) -> Vector3:
	for node in client.find_children("Mailbox_%d" % id, "Node3D", true, false):
		var model := node.get_node("GameObjectModel") as Node3D
		var bounds: AABB = model.get_meta("m2_bounds")
		return model.global_transform * (bounds.get_center() + Vector3(0, bounds.size.y / 2.0, 0))
	return Vector3.ZERO

# Turn right in small steps until the named object is the soft interact target; its id.
func face_soft_object(name: String) -> int:
	var deadline := Time.get_ticks_msec() + NPC_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await frames(2)
		var node := object_node(name)
		if node == null:
			continue
		var id: int = node.get_meta("game_object_server_id")
		if client.soft_interact_state().get("target") == id:
			await frames(5)
			return id
		push_key(KEY_RIGHT, true)
		await frames(2)
		push_key(KEY_RIGHT, false)
	fail("%s never became the soft interact target: %s" % [name, client.soft_interact_state()])
	return -1
