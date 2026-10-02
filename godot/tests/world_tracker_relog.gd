extends SceneTree

## Objective tracker after a relog (docs/specs/quest-ui.md): a quest accepted in one
## session is still watched, and shown, when the character enters the world again.
## Environment:
##   GODOT_TEST_SERVER   a private test server (never the shared :5000)
##   TR_ACCOUNT          account (password fbtest) of a level-1 Human who has not taken
##                       "Beating Them Back!" (28766), within 10 yd of Marshal McBride
##                       (game-server-admin set-position <name> -8910.5 -137.5 81.2)
##   TR_SHOTS            screenshot directory

const PASSWORD := "fbtest"
const QUEST_GIVER := "Marshal McBride"
const QUEST := 28766
const QUEST_TITLE := "Beating Them Back!"
const WORLD_WAIT_MS := 420000
const WAIT_MS := 30000

var client: Node
var server := ""
var shots := "/tmp/claude/tracker-relog/"

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	server = OS.get_environment("GODOT_TEST_SERVER")
	if server == "" or server.ends_with(":5000"):
		fail("GODOT_TEST_SERVER must name a private server, not :5000")
		return
	if OS.get_environment("TR_SHOTS") != "":
		shots = OS.get_environment("TR_SHOTS").trim_suffix("/") + "/"
	DirAccess.make_dir_recursive_absolute(shots)
	if not await log_in():
		return
	if not await accept_quest():
		return
	var accepted := await wait_tracked()
	if accepted.is_empty():
		fail("Tracker does not show %s after accepting it: %s" % [QUEST_TITLE, client.objective_tracker_state()])
		return
	await capture("01-tracker-after-accept.png")
	client.free()
	await frames(30)
	if not await log_in():
		return
	var quest := await wait_tracked()
	print("FIXTURE RELOG TRACKER ", client.objective_tracker_state(), " quests ", client.quest_state() if client.has_method("quest_state") else "")
	await capture("02-tracker-after-relog.png")
	if quest.is_empty() or quest.title != QUEST_TITLE:
		fail("Tracker does not show %s after relog: %s" % [QUEST_TITLE, client.objective_tracker_state()])
		return
	var state: Dictionary = client.objective_tracker_state()
	if not state.get("visible", false):
		fail("Tracker hidden after relog: " + str(state))
		return
	print("FIXTURE TRACKER_RELOG_DONE")
	client.free()
	quit(0)

func log_in() -> bool:
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, OS.get_environment("TR_ACCOUNT"), PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return false
	var deadline := Time.get_ticks_msec() + 60000
	var replied := false
	while Time.get_ticks_msec() < deadline and not replied:
		await process_frame
		var state: Dictionary = client.account_state()
		replied = state.reply_received
		if replied and (state.screen != "CharacterSelect" or state.character_count < 1):
			fail("Fixture needs an authenticated character: " + str(state))
			return false
	if not replied:
		fail("Timed out waiting for the login reply: " + str(client.account_state()))
		return false
	var enter = null
	deadline = Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline and not enter is Button:
		await process_frame
		var ui = client.get_node_or_null("CharacterSelectUI")
		enter = ui.find_child("EnterWorld", true, false) if ui != null else null
	if not enter is Button:
		fail("Enter World button missing")
		return false
	await click_point(enter.get_global_rect().get_center())
	deadline = Time.get_ticks_msec() + WORLD_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "InWorld" and state.local_player_position != null and not state.terrain.parsed_tiles.is_empty():
			print("FIXTURE IN_WORLD ", state.selected_character_name, " at ", state.local_player_position)
			return true
	fail("Timed out entering the world: " + str(client.account_state()))
	return false

func accept_quest() -> bool:
	var deadline := Time.get_ticks_msec() + WAIT_MS
	var error := "no attempt"
	while Time.get_ticks_msec() < deadline:
		error = client.accept_quest_from(QUEST_GIVER, QUEST)
		if error == "":
			return true
		await frames(10)
	fail("Accepting %d: %s" % [QUEST, error])
	return false

func wait_tracked() -> Dictionary:
	var deadline := Time.get_ticks_msec() + WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		for quest in client.objective_tracker_state().get("quests", []):
			if quest.quest_id == QUEST:
				return quest
	return {}

func frames(count: int) -> void:
	for frame in range(count):
		await process_frame

func capture(file: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var error := image.save_png(shots + file)
	if error != OK:
		fail("Could not save " + file + ": " + str(error))

func click_point(point: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await process_frame
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await process_frame

func fail(message: String) -> void:
	push_error(message)
	quit(1)
