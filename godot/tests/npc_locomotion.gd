extends SceneTree

## Live dev-server proof that a wandering creature plays its replicated locomotion: while it
## moves its authored animation is Walk 4 (or Run 5) and its bones change; once it stops it
## returns to Stand 0. The character turns to face the creature so its model is on screen and
## sampled (off-screen NPC animation LOD writes no bones). Account and character (card 0) come from
## NPC_WALK_ACCOUNT, NPC_WALK_PASSWORD, NPC_WALK_CHARACTER; place the character near wandering
## creatures with `game-server-admin set-position` while it is offline.

const Probe := preload("res://tests/player_locomotion_probe.gd")
const SHOT_DIR := "/tmp/claude"
const SEARCH_MS := 60000
const WATCH_MS := 45000
## A creature whose interpolated feet move this far within `MOVE_WINDOW_MS` is moving.
const MOVE_YARDS := 0.25
const MOVE_WINDOW_MS := 500
const MAX_DISTANCE := 45.0
## Per 250 ms sample: a walk covers about 0.6 yd; a stopped creature's feet do not move.
const MOVING_YARDS := 0.3
const STILL_YARDS := 0.02
const YAW_TOLERANCE := 0.12

var NAME := OS.get_environment("NPC_WALK_CHARACTER")
var turn_sign := 0.0
var shot := 0

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if server != "127.0.0.1:5000":
		fail("GODOT_TEST_SERVER must explicitly select 127.0.0.1:5000")
		return
	var account := OS.get_environment("NPC_WALK_ACCOUNT")
	var password := OS.get_environment("NPC_WALK_PASSWORD")
	if account == "" or password == "" or NAME == "":
		fail("NPC_WALK_ACCOUNT, NPC_WALK_PASSWORD and NPC_WALK_CHARACTER are required")
		return
	var client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, password, false)
	if error != "":
		fail("connect: " + error)
		return
	if not await enter_world(client):
		return
	var player := client.get_node("WorldUnits/" + NAME) as Node3D
	var units := client.get_node("WorldUnits") as Node3D
	var creature := await find_moving_creature(units, player)
	if creature == null:
		return
	await watch(creature, player)

func enter_world(client: Node) -> bool:
	if not await wait_until(client, func(state): return state.screen == "CharacterSelect" and state.reply_received, 20000, "CharacterSelect"):
		return false
	var ui = client.get_node("CharacterSelectUI")
	await click_control(ui.find_child("CharCard_0", true, false))
	await process_frame
	var selected = ui.find_child("CharSelectCharacterName", true, false)
	if selected.text != NAME:
		fail("Card 0 is %s, not %s" % [selected.text, NAME])
		return false
	await click_control(ui.find_child("EnterWorld", true, false))
	var ready := func(state):
		return state.screen == "InWorld" and state.selected_character_name == NAME \
			and state.local_player_position != null and state.terrain.pending_count == 0 \
			and not state.terrain.parsed_tiles.is_empty()
	if not await wait_until(client, ready, 120000, "InWorld with terrain"):
		return false
	for _frame in 60:
		await process_frame
	print("TRACE entered: ", client.account_state())
	return true

## The nearest creature with an animated model whose feet move, within `MAX_DISTANCE`.
func find_moving_creature(units: Node3D, player: Node3D) -> Node3D:
	var history := {}
	var deadline := Time.get_ticks_msec() + SEARCH_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var now := Time.get_ticks_msec()
		var best: Node3D = null
		var best_distance := MAX_DISTANCE
		for child in units.get_children():
			var unit := child as Node3D
			if unit == player or animation_of(unit) == null:
				continue
			var samples: Array = history.get_or_add(unit.get_instance_id(), [])
			samples.append([now, unit.global_position])
			while samples.size() > 1 and now - samples[0][0] > MOVE_WINDOW_MS:
				samples.pop_front()
			if now - samples[0][0] < MOVE_WINDOW_MS * 0.8:
				continue
			var distance := unit.global_position.distance_to(player.global_position)
			var start: Vector3 = samples[0][1]
			if start.distance_to(unit.global_position) > MOVE_YARDS and distance < best_distance:
				best = unit
				best_distance = distance
		if best != null:
			print("TRACE moving creature %s at %.1f yd, animation %d" % [best.name, best_distance, animation_of(best).current_animation_id()])
			return best
	fail("No creature within %.0f yd moved in %d ms" % [MAX_DISTANCE, SEARCH_MS])
	return null

func watch(creature: Node3D, player: Node3D) -> void:
	var model := creature.find_child("NpcModel", true, false) as Node3D
	var animation := animation_of(creature)
	var camera := client_camera(creature)
	var probe := Probe.new()
	probe.skeleton = model.get_node("Skeleton3D") as Skeleton3D
	probe.animation = animation
	var last := creature.global_position
	var last_at := Time.get_ticks_msec()
	var still_since := -1
	var walked := false
	var walk_id := -1
	var standing_moves := 0
	var deadline := Time.get_ticks_msec() + WATCH_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if not is_instance_valid(creature):
			fail("Creature despawned while watched")
			return
		var aligned: bool = await face(player, creature)
		var now := Time.get_ticks_msec()
		if now - last_at < 250:
			continue
		var moved := last.distance_to(creature.global_position)
		last = creature.global_position
		last_at = now
		var id: int = animation.current_animation_id()
		print("TRACE %s moved %.3f yd, animation %d, in view %s" % [creature.name, moved, id, camera.is_position_in_frustum(model.global_position)])
		# Between the two thresholds the client is still interpolating toward the last position.
		if moved > MOVING_YARDS:
			still_since = -1
			# Interpolated feet trail the server: one sample may still close the gap after a stop.
			standing_moves = standing_moves + 1 if id != 4 and id != 5 else 0
			if standing_moves >= 2:
				fail("%s moved over 500 ms but plays %d, not Walk 4 / Run 5" % [creature.name, id])
				return
			if standing_moves > 0:
				continue
			if not walked and aligned and camera.is_position_in_frustum(model.global_position):
				release_turn()
				walk_id = id
				walked = await prove_bones_move(creature, probe, id)
				if not walked:
					return
		elif moved < STILL_YARDS:
			standing_moves = 0
		if walked and moved < STILL_YARDS:
			if still_since < 0:
				still_since = now
			# Stopping crossfades back to Stand 0 (at least 150 ms) before holding it.
			if id == 0 and now - still_since >= 150:
				print("PASS: %s moved playing %d with changing bones, then stood with Stand 0" % [creature.name, walk_id])
				quit(0)
				return
			if now - still_since > 2000:
				fail("%s stopped for %d ms but plays %d, not Stand 0" % [creature.name, now - still_since, id])
				return
	fail("Did not observe %s walk on screen and stop within %d ms (walked=%s)" % [creature.name, WATCH_MS, walked])

func prove_bones_move(creature: Node3D, probe: Probe, id: int) -> bool:
	var before := probe.capture_pose()
	await snapshot("walk-a", creature)
	for _frame in 12:
		await process_frame
	await snapshot("walk-b", creature)
	var now_id: int = probe.animation.current_animation_id()
	if now_id != id:
		print("TRACE animation changed %d -> %d while sampling bones" % [id, now_id])
	if not probe.changed_from(before):
		fail("%s plays %d but its bones did not change over 12 frames" % [creature.name, id])
		return false
	print("TRACE %s bones changed while playing %d" % [creature.name, id])
	return true

func animation_of(unit: Node) -> WowAnimationPlayer:
	var model := unit.find_child("NpcModel", true, false)
	if model == null:
		return null
	return model.get_node_or_null("M2Animation") as WowAnimationPlayer

## Turn the character toward `target` with the arrow keys; the follow camera turns with it.
## Whether it already faces the target.
func face(player: Node3D, target: Node3D) -> bool:
	var d := target.global_position - player.global_position
	var error := angle_difference(player.rotation.y + PI / 2.0, atan2(d.x, d.z))
	if turn_sign == 0.0:
		var before := player.rotation.y
		push_key(KEY_RIGHT, true)
		for _frame in 10:
			await process_frame
		push_key(KEY_RIGHT, false)
		await process_frame
		var turned := angle_difference(before, player.rotation.y)
		turn_sign = 1.0 if turned >= 0.0 else -1.0
		return false
	push_key(KEY_RIGHT, abs(error) > YAW_TOLERANCE and sign(error) == turn_sign)
	push_key(KEY_LEFT, abs(error) > YAW_TOLERANCE and sign(error) == -turn_sign)
	return abs(error) <= YAW_TOLERANCE

func release_turn() -> void:
	push_key(KEY_RIGHT, false)
	push_key(KEY_LEFT, false)

func snapshot(label: String, creature: Node3D) -> void:
	await RenderingServer.frame_post_draw
	var path := "%s/npc-locomotion-%02d-%s.png" % [SHOT_DIR, shot, label]
	shot += 1
	root.get_texture().get_image().save_png(path)
	var camera := root.get_camera_3d()
	print("TRACE screenshot %s, creature at pixel %s" % [path, camera.unproject_position(creature.global_position)])

func wait_until(client: Node, predicate: Callable, timeout_ms: int, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call(client.account_state()):
			return true
	fail("Timed out waiting for %s: %s" % [what, client.account_state()])
	return false

func push_key(keycode: Key, pressed: bool) -> void:
	var key := InputEventKey.new()
	key.physical_keycode = keycode
	key.keycode = keycode
	key.pressed = pressed
	root.push_input(key, true)

func click_control(control: Control) -> void:
	var point := control.get_global_rect().get_center()
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await process_frame

func fail(message: String) -> void:
	push_error(message)
	quit(1)

## The client's world camera: the NPC animation LOD samples a model only while its
## bounds are in this camera's view frustum.
func client_camera(creature: Node3D) -> Camera3D:
	return creature.get_parent().get_parent().get_node("WorldCamera") as Camera3D
