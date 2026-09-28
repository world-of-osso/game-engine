extends SceneTree

## Dungeon-entrance difficulty bar in world against the dev server (docs/specs/instances.md,
## Entrance difficulty bar): standing at the Stockade entrance the bar shows "The Stockade"
## and its only difficulty, "(5) Normal  (0/3)". The character is first put on Heroic, which
## the Stockade lacks, so nothing is boxed; clicking Normal sends SetDungeonDifficulty, spins
## until the server's DungeonDifficultySet and then boxes Normal; clicking it again sends
## nothing. Walking up the stairwell to the room floor (36.8 yd, past Plumber's 31) fades the
## bar out; walking back down shows it again. Account fb_stockade / Fbstockade, placed while
## offline with `game-server-admin set-position Fbstockade -8766.11 845.5 88` (map 0).

const ACCOUNT := "fb_stockade"
const PASSWORD := "fbtest"
const CHARACTER := "Fbstockade"
const STOCKADE := 238
const HEROIC := 2
const NORMAL := 1
## Engine (x, z) = WoW (x, -y): the stairwell, its top, and the room floor 36.8 yd away.
const UP_THE_STAIRS := [Vector2(-8772.0, -836.0), Vector2(-8786.0, -836.0), Vector2(-8785.93, -820.67)]
const DOWN_THE_STAIRS := [Vector2(-8786.0, -836.0)]
const ARRIVE := 0.8
const YAW_TOLERANCE := 0.06
const LEG_TIMEOUT_MS := 20000
const SHOTS := "/tmp/claude/entrancebar/"

var client: Node
var player: Node3D
var turn_sign := 0.0

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	DirAccess.make_dir_recursive_absolute(SHOTS)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if server != "127.0.0.1:5000":
		fail("GODOT_TEST_SERVER must explicitly select 127.0.0.1:5000")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, ACCOUNT, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world():
		return
	player = client.get_node("WorldUnits/" + CHARACTER) as Node3D
	if not await wait_for(func(bar): return bar.dungeon_difficulty != -1, 10000, "DungeonDifficultySet at login"):
		return
	if bar_state().dungeon_difficulty != HEROIC:
		client.set_dungeon_difficulty(HEROIC)
		if not await wait_for(func(bar): return bar.dungeon_difficulty == HEROIC, 10000, "Heroic set"):
			return
	if not await wait_for(func(bar): return bar.open and bar.frame_alpha == 1.0, 10000, "bar faded in"):
		return
	var bar := bar_state()
	print("FIXTURE BAR AT ENTRANCE ", bar)
	if bar.title != "The Stockade" or bar.target != STOCKADE or bar.distance >= 31.0:
		fail("Wrong entrance: " + str(bar))
		return
	if bar.choices.size() != 1 or bar.choices[0].id != NORMAL or bar.choices[0].label != "(5) Normal" or bar.choices[0].progress != "(0/3)":
		fail("Stockade choices: " + str(bar.choices))
		return
	if bar.selected != -1 or bar.box_left != null:
		fail("Heroic is not offered, yet something is boxed: " + str(bar))
		return
	if not check_labels({"EntranceDifficultyTitle": "The Stockade", "EntranceDifficultyLabel1": "(5) Normal", "EntranceDifficultyProgress1": "(0/3)"}):
		return
	await move_mouse(Vector2(20, 700))
	await wait_ms(600)
	if not is_equal_approx(bar_state().bar_alpha, 0.5):
		fail("Idle bar alpha %s, expected 0.5" % bar_state().bar_alpha)
		return
	await capture("01-heroic-unselected-idle.png")

	var normal := button(NORMAL)
	await move_mouse(normal.get_global_rect().get_center())
	if not await wait_for(func(bar): return bar.bar_alpha == 1.0, 3000, "bar brightened under the pointer"):
		return
	await capture("02-hover.png")
	await click(normal)
	if not bar_state().spinning:
		fail("No spinner after clicking Normal: " + str(bar_state()))
		return
	await capture("03-spinner.png")
	if not await wait_for(func(bar): return bar.dungeon_difficulty == NORMAL and bar.selected == NORMAL and not bar.spinning, 10000, "Normal selected by the server"):
		return
	# The box rests 6 px left of the button: 20 px bar padding - 6.
	if not await wait_for(func(bar): return bar.box_left != null and is_equal_approx(bar.box_left, 14.0), 3000, "box on Normal"):
		return
	await move_mouse(Vector2(20, 700))
	await wait_ms(600)
	await capture("04-normal-selected.png")
	await click(button(NORMAL))
	if bar_state().spinning:
		fail("Clicking the selected difficulty spun: " + str(bar_state()))
		return

	for target in UP_THE_STAIRS:
		var result: String = await walk_to(target)
		if result != "arrived":
			fail("Walk up to %s: %s" % [target, result])
			return
	if not await wait_for(func(bar): return not bar.open and not bar.has("target"), 5000, "bar hidden on the room floor"):
		return
	print("FIXTURE ROOM FLOOR ", bar_state(), " at ", player.position)
	await capture("05-room-floor-hidden.png")
	for target in DOWN_THE_STAIRS:
		var result: String = await walk_to(target)
		if result != "arrived":
			fail("Walk down to %s: %s" % [target, result])
			return
	if not await wait_for(func(bar): return bar.open and bar.frame_alpha == 1.0 and bar.selected == NORMAL, 5000, "bar back at the stairwell top"):
		return
	print("FIXTURE STAIRWELL TOP ", bar_state(), " at ", player.position)
	await capture("06-back-in-range.png")
	print("FIXTURE ENTRANCE_BAR_DONE")
	client.free()
	quit(0)

func bar_state() -> Dictionary:
	return client.entrance_bar_state()

func bar_ui() -> Node:
	return client.get_node("EntranceBarUI")

func button(id: int) -> Control:
	return bar_ui().find_child("EntranceDifficultyButton%d" % id, true, false) as Control

func check_labels(expected: Dictionary) -> bool:
	for name in expected:
		var label := bar_ui().find_child(name, true, false) as Label
		if label == null or label.text != expected[name]:
			fail("%s shows '%s', expected '%s'" % [name, label.text if label else "<none>", expected[name]])
			return false
	return true

func wait_for(predicate: Callable, timeout_ms: int, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call(bar_state()):
			return true
	fail("Timed out waiting for %s: %s" % [what, bar_state()])
	return false

func enter_world() -> bool:
	var deadline := Time.get_ticks_msec() + 20000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.reply_received and state.screen == "CharacterSelect" and state.character_count >= 1:
			break
	var ui = client.get_node_or_null("CharacterSelectUI")
	if ui == null:
		fail("No character select: " + str(client.account_state()))
		return false
	await click(ui.find_child("CharCard_0", true, false))
	var selected = ui.find_child("CharSelectCharacterName", true, false)
	if selected.text != CHARACTER:
		fail("Card 0 is %s, not %s" % [selected.text, CHARACTER])
		return false
	await click(ui.find_child("EnterWorld", true, false))
	deadline = Time.get_ticks_msec() + 120000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "InWorld" and state.local_player_position != null and state.local_server_position != null and state.terrain.map == "azeroth" and state.terrain.pending_count == 0 and not state.terrain.parsed_tiles.is_empty():
			print("FIXTURE IN_WORLD at ", state.local_player_position)
			await wait_frames(60)
			return true
	fail("Timed out entering the world: " + str(client.account_state()))
	return false

func walk_to(target: Vector2) -> String:
	var deadline := Time.get_ticks_msec() + LEG_TIMEOUT_MS
	var best := INF
	var best_at := Time.get_ticks_msec()
	var running := false
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var here := Vector2(player.position.x, player.position.z)
		var distance := here.distance_to(target)
		if distance < ARRIVE:
			release_all()
			return "arrived"
		if distance < best - 0.05:
			best = distance
			best_at = Time.get_ticks_msec()
		elif Time.get_ticks_msec() - best_at > 4000:
			release_all()
			return "stuck %.2f yd from %s at %s" % [distance, target, here]
		var error := angle_difference(facing(), bearing(here, target))
		if not await steer(error):
			return "turn keys do not change facing"
		var aligned: bool = abs(error) < 0.4
		if aligned != running:
			running = aligned
			push_key(KEY_W, running)
	release_all()
	return "leg timeout"

func bearing(from: Vector2, to: Vector2) -> float:
	var d := to - from
	return atan2(d.x, d.y)

func facing() -> float:
	return player.rotation.y + PI / 2.0

func steer(error: float) -> bool:
	if turn_sign == 0.0:
		var before := facing()
		push_key(KEY_RIGHT, true)
		await wait_frames(10)
		push_key(KEY_RIGHT, false)
		await process_frame
		var turned := angle_difference(before, facing())
		if abs(turned) < 0.01:
			return false
		turn_sign = sign(turned)
		return true
	push_key(KEY_RIGHT, abs(error) > YAW_TOLERANCE and sign(error) == turn_sign)
	push_key(KEY_LEFT, abs(error) > YAW_TOLERANCE and sign(error) == -turn_sign)
	return true

func release_all() -> void:
	for key in [KEY_W, KEY_LEFT, KEY_RIGHT]:
		push_key(key, false)

func push_key(code: Key, pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.pressed = pressed
	root.push_input(event, true)

func move_mouse(point: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	Input.warp_mouse(point)
	await wait_frames(2)

func click(control: Control) -> void:
	var point := control.get_global_rect().get_center()
	await move_mouse(point)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await wait_frames(3)

func capture(file: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var error := image.save_png(SHOTS + file)
	if error != OK:
		fail("Could not save " + file + ": " + str(error))

func wait_frames(count: int) -> void:
	for frame in range(count):
		await process_frame

func wait_ms(duration: int) -> void:
	var deadline := Time.get_ticks_msec() + duration
	while Time.get_ticks_msec() < deadline:
		await process_frame

func fail(message: String) -> void:
	push_error(message)
	quit(1)
