extends "res://tests/world_merchant_flow.gd"

# Live soft interact against a private server (Retail SoftTargetInteract): turning in place
# until Brother Danil stands directly in front makes him the soft interact target with the
# Buy cursor icon above him; the INTERACTTARGET key (E) then opens his MerchantFrame.
# Environment:
#   GODOT_TEST_SERVER      private server address (never 127.0.0.1:5000)
#   SOFT_INTERACT_ACCOUNT / SOFT_INTERACT_CHARACTER  account (password fbtest) and character
#   SOFT_INTERACT_SHOTS    screenshot directory
#   XDG_CONFIG_HOME        isolated options: hud softTargetInteract true, InteractTarget key:KeyE
# Setup (game-server-admin, character offline): set-position <character> -8904.6 -112.7 82.1,
# 3 yd west of Brother Danil (world.db creature guid 79950).

const DANIL := "Brother Danil"
const LIVE_WORLD_MS := 420000

var shots := ""

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("SOFT_INTERACT_ACCOUNT")
	var character := OS.get_environment("SOFT_INTERACT_CHARACTER")
	shots = OS.get_environment("SOFT_INTERACT_SHOTS")
	if server.is_empty() or server == "127.0.0.1:5000" or account.is_empty() or character.is_empty() or shots.is_empty():
		fail("Needs a private GODOT_TEST_SERVER, SOFT_INTERACT_ACCOUNT/CHARACTER and SOFT_INTERACT_SHOTS")
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
	var danil := await face_soft_target(DANIL)
	if danil < 0:
		return
	var state: Dictionary = client.soft_interact_state()
	if state.get("icon") != "Buy" or not state.has("icon_rect"):
		fail("Soft target %s shows no Buy icon: %s" % [DANIL, state])
		return
	print("FIXTURE SOFT_TARGET ", DANIL, " ", state)
	await shot("01-soft-target-icon.png")
	await tap(KEY_E)
	if not await wait_for(func(s): return s.open and s.vendor_name == DANIL, DANIL + " frame"):
		return
	await frames(3)
	print("FIXTURE INTERACT_KEY_OPENED ", DANIL, " target ", client.target_state().get("target"))
	await shot("02-interact-key-merchant.png")
	print("FIXTURE SOFT_INTERACT_LIVE_DONE")
	client.free()
	quit(0)

func enter_as(character: String) -> bool:
	var ui: Node = null
	var deadline := Time.get_ticks_msec() + WORLD_WAIT_MS
	while ui == null and Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.reply_received and state.screen == "CharacterSelect" and state.character_count >= 1:
			ui = client.get_node_or_null("CharacterSelectUI")
	await frames(10)
	var card: Control = null
	for candidate in ui.find_children("CharCard_*", "", true, false) if ui else []:
		for label in candidate.find_children("*", "Label", true, false):
			if label.text == character:
				card = candidate
	if card == null:
		fail("Roster has no %s: %s" % [character, client.account_state()])
		return false
	await click_control(card, MOUSE_BUTTON_LEFT)
	await click_control(ui.find_child("EnterWorld", true, false), MOUSE_BUTTON_LEFT)
	deadline = Time.get_ticks_msec() + LIVE_WORLD_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "InWorld" and state.local_player_position != null:
			print("FIXTURE IN_WORLD ", state.selected_character_name, " at ", state.local_player_position)
			return true
	fail("Timed out entering the world: " + str(client.account_state()))
	return false

# Turn right in small steps until the named unit is the soft interact target; its id.
func face_soft_target(name: String) -> int:
	var deadline := Time.get_ticks_msec() + NPC_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await frames(2)
		var id := unit_id(name)
		if id < 0:
			continue
		var state: Dictionary = client.soft_interact_state()
		if state.get("target") == id and state.has("icon_rect"):
			await frames(5)
			return id
		push_key(KEY_RIGHT, true)
		await frames(2)
		push_key(KEY_RIGHT, false)
	fail("%s never became the soft interact target: %s" % [name, client.soft_interact_state()])
	return -1

func unit_id(name: String) -> int:
	var units := client.get_node_or_null("WorldUnits")
	if units == null:
		return -1
	for unit in units.get_children():
		if str(unit.name) == name:
			var area := unit.find_child("UnitPick", true, false) as Area3D
			if area != null:
				return area.get_meta("unit_server_id")
	return -1

func shot(file: String) -> void:
	await RenderingServer.frame_post_draw
	var error := root.get_texture().get_image().save_png(shots.path_join(file))
	if error != OK:
		fail("Could not save " + file + ": " + str(error))
