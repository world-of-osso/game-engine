extends SceneTree

## Live proof of the player stand state (`PlayerStandState`) against a private server.
## Run once alone, or twice on one server for the replicated view:
## - STAND_ROLE=sitter (default): enters the world and, with real key events, presses X
##   (SITORSTAND): SitGroundDown 96 then SitGround 97; X again: SitGroundUp 98 then Stand 0;
##   /kneel: KneelStart 114 then KneelLoop 115; walking (W) stands it up out of the kneel.
##   Each phase is captured and, with STAND_SYNC, announced in a file.
## - STAND_ROLE=observer: enters the world, turns to face STAND_OTHER and records its
##   model's animation ID per announced phase. It passes when the other player sat down
##   (96 → 97), stood up (98 → 0) and knelt (115).
## - STAND_FOOD=1 (sitter): instead of the X//kneel phases it announces "need_damage" and
##   waits for its health to drop (the operator damages it), right-clicks the Tough Hunk of
##   Bread in backpack slot 0 (the food sits it down: 96 → 97) and requires its health to
##   rise while seated, then announces "eating" and waits, seated, for a hit (the operator
##   moves it next to a hostile creature) to stand it up: health drops and the pose ends.
## Environment: GODOT_TEST_SERVER (127.0.0.1:<port>, never :5000), STAND_ACCOUNT (password
## fbtest), STAND_CHARACTER (card 0), GODOT_TEST_CAPTURE_DIR; for two clients also
## STAND_SYNC (a directory both share) and, for the observer, STAND_OTHER (the sitter's
## character). Place both characters a few yards apart while they are offline.

const PASSWORD := "fbtest"
const READY_FILE := "observer_ready"
const PHASE_FILE := "sitter_phase"
const YAW_TOLERANCE := 0.12
const STAND := 0
const SIT_DOWN := 96
const SIT := 97
const SIT_UP := 98
const KNEEL_DOWN := 114
const KNEEL := 115

var NAME := OS.get_environment("STAND_CHARACTER")
var OTHER := OS.get_environment("STAND_OTHER")
var SYNC := OS.get_environment("STAND_SYNC")
var shots := OS.get_environment("GODOT_TEST_CAPTURE_DIR")
var client: Node
var turn_sign := 0.0

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if not server.begins_with("127.0.0.1:") or server.ends_with(":5000"):
		fail("GODOT_TEST_SERVER must select a private loopback server, not :5000")
		return
	var role := OS.get_environment("STAND_ROLE")
	if role == "":
		role = "sitter"
	var account := OS.get_environment("STAND_ACCOUNT")
	if account == "" or NAME == "" or shots == "":
		fail("STAND_ACCOUNT, STAND_CHARACTER and GODOT_TEST_CAPTURE_DIR are required")
		return
	if role == "observer" and (OTHER == "" or SYNC == ""):
		fail("The observer needs STAND_OTHER and STAND_SYNC")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("connect: " + error)
		return
	if not await enter_world():
		return
	if role == "sitter" and OS.get_environment("STAND_FOOD") == "1":
		await eat()
	elif role == "sitter":
		await sit()
	elif role == "observer":
		await observe()
	else:
		fail("STAND_ROLE must be sitter or observer")

# --- Sitter ------------------------------------------------------------------------

func sit() -> void:
	if SYNC != "":
		var deadline := Time.get_ticks_msec() + 180000
		while not FileAccess.file_exists(SYNC.path_join(READY_FILE)):
			if Time.get_ticks_msec() > deadline:
				fail("Observer never became ready")
				return
			await process_frame
	var animation := local_animation()
	if not await expect_clips(animation, [STAND], "Stand before X"):
		return
	announce("sit")
	await tap(KEY_X)
	if not await expect_clips(animation, [SIT_DOWN, SIT], "X sits down"):
		return
	await capture("stand-state-sit.png")
	await wait_ms(2000)
	announce("stand")
	await tap(KEY_X)
	if not await expect_clips(animation, [SIT_UP, STAND], "X stands up"):
		return
	await capture("stand-state-stand.png")
	await wait_ms(2000)
	announce("kneel")
	if not await send_line("/kneel"):
		return
	if not await expect_clips(animation, [KNEEL_DOWN, KNEEL], "/kneel"):
		return
	await capture("stand-state-kneel.png")
	await wait_ms(2000)
	announce("walk")
	push_key(KEY_W, true)
	await wait_ms(600)
	push_key(KEY_W, false)
	if animation.current_animation_id() == KNEEL:
		fail("Walking kept KneelLoop")
		return
	if not await expect_clips(animation, [STAND], "Stand after walking out of /kneel"):
		return
	await wait_ms(2000)
	announce("done")
	await wait_ms(2000)
	print("FIXTURE STAND_STATE_DONE")
	quit(0)

# --- Food ----------------------------------------------------------------------------

func eat() -> void:
	var animation := local_animation()
	var full: float = client.account_state().local_player_health
	announce("need_damage")
	if not await wait_until(func(state): return state.local_player_health < full, 120000, "damage"):
		return
	await wait_ms(1500)
	var hurt: float = client.account_state().local_player_health
	await wait_ms(4000)
	var standing_gain: float = client.account_state().local_player_health - hurt
	hurt = client.account_state().local_player_health
	print("FIXTURE HURT %s of %s, regained %s standing in 4 s" % [hurt, full, standing_gain])
	if hurt >= full:
		fail("Healed to full before eating")
		return
	var backpack := client.find_child("MainMenuBarBackpackButton", true, false) as Control
	if backpack == null or not backpack.is_visible_in_tree():
		fail("No visible backpack button")
		return
	await pointer_click(backpack.get_global_rect().get_center(), MOUSE_BUTTON_LEFT)
	var bread: Control = null
	var bag_deadline := Time.get_ticks_msec() + 10000
	while bread == null or not bread.is_visible_in_tree() or not bread.get_global_rect().has_area():
		if Time.get_ticks_msec() > bag_deadline:
			fail("Backpack slot 0 not shown")
			return
		await process_frame
		bread = client.find_child("ContainerFrame0Slot0", true, false) as Control
	await capture("food-bag.png")
	await pointer_click(bread.get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
	if not await expect_clips(animation, [SIT_DOWN, SIT], "Bread sits down"):
		return
	await pointer_click(backpack.get_global_rect().get_center(), MOUSE_BUTTON_LEFT)
	var seated: float = client.account_state().local_player_health
	var samples: Array = []
	for _second in 4:
		await wait_ms(1000)
		samples.append(client.account_state().local_player_health)
	var eating_gain: float = samples[-1] - seated
	print("FIXTURE EATING_HEALTH %s from %s: %s in 4 s seated vs %s standing" % [samples, seated, eating_gain, standing_gain])
	if eating_gain <= 2.0 * standing_gain or animation.current_animation_id() != SIT:
		fail("Eating seated did not out-heal standing regen: %s vs %s, clip %d" % [eating_gain, standing_gain, animation.current_animation_id()])
		return
	await capture("food-eating.png")
	var fed: float = client.account_state().local_player_health
	announce("eating")
	var hit := func(state): return state.local_player_health < fed and animation.current_animation_id() != SIT
	if not await wait_until(hit, 120000, "a hit to stand the eater up"):
		return
	print("FIXTURE HIT health %s clip %d" % [client.account_state().local_player_health, animation.current_animation_id()])
	await wait_ms(300)
	await capture("food-hit.png")
	announce("done")
	print("FIXTURE FOOD_DONE")
	quit(0)

func pointer_click(point: Vector2, button: int) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await process_frame
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = button
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await process_frame

func local_animation() -> WowAnimationPlayer:
	var model := client.find_child("PlayerModel", true, false)
	return model.get_node_or_null("M2Animation") as WowAnimationPlayer if model != null else null

## Wait until `animation` has played `ids` in order (each within 5 s of the previous).
func expect_clips(animation: WowAnimationPlayer, ids: Array, what: String) -> bool:
	var seen: Array = []
	for id in ids:
		var deadline := Time.get_ticks_msec() + 5000
		while animation.current_animation_id() != id:
			if Time.get_ticks_msec() > deadline:
				fail("%s: played %s, not %s" % [what, seen, ids])
				return false
			if seen.is_empty() or seen[-1] != animation.current_animation_id():
				seen.append(animation.current_animation_id())
			await process_frame
		seen.append(id)
	print("FIXTURE CLIPS ", what, ": ", seen)
	return true

func announce(phase: String) -> void:
	if SYNC == "":
		return
	var file := FileAccess.open(SYNC.path_join(PHASE_FILE), FileAccess.WRITE)
	file.store_string(phase)
	file.close()
	print("TRACE phase %s at %d" % [phase, Time.get_ticks_msec()])

func send_line(line: String) -> bool:
	await tap(KEY_ENTER)
	var ui := client.get_node_or_null("ChatFrameUI")
	var edit := ui.find_child("ChatFrame1EditBox", true, false) as LineEdit if ui != null else null
	if edit == null or not edit.has_focus():
		fail("Enter did not focus the chat edit box before '%s'" % line)
		return false
	for character in line:
		var code := KEY_SLASH if character == "/" else OS.find_keycode_from_string(character.to_upper())
		push_key(code, true, character.unicode_at(0))
		await process_frame
		push_key(code, false)
	await process_frame
	await tap(KEY_ENTER)
	return true

# --- Observer ----------------------------------------------------------------------

func observe() -> void:
	var player := client.get_node("WorldUnits/" + NAME) as Node3D
	var other := await find_other()
	if other == null:
		return
	var deadline := Time.get_ticks_msec() + 20000
	while not await face(player, other):
		if Time.get_ticks_msec() > deadline:
			fail("Could not turn to face " + OTHER)
			return
	release_turn()
	for _frame in 30:
		await process_frame
	var animation := other.find_child("PlayerModel", true, false).get_node("M2Animation") as WowAnimationPlayer
	FileAccess.open(SYNC.path_join(READY_FILE), FileAccess.WRITE).close()
	var timeline := {}
	var phase := ""
	var captured := {}
	var done_deadline := Time.get_ticks_msec() + 120000
	while Time.get_ticks_msec() < done_deadline:
		await process_frame
		var announced := read_phase()
		if announced != phase:
			phase = announced
			print("TRACE observer sees phase %s" % phase)
			if phase == "done":
				break
		if phase == "":
			continue
		var id := animation.current_animation_id()
		var ids: Array = timeline.get_or_add(phase, [])
		if ids.is_empty() or ids[-1] != id:
			ids.append(id)
			print("TRACE %s: %s plays %d" % [phase, OTHER, id])
		if id in [SIT, KNEEL] and not captured.has(phase):
			captured[phase] = true
			await capture("stand-state-observer-%s.png" % phase)
	if phase != "done":
		fail("Sitter did not finish; timeline %s" % timeline)
		return
	var failures: Array[String] = []
	var sat: Array = timeline.get("sit", [])
	if not (sat.has(SIT_DOWN) and sat.find(SIT) > sat.find(SIT_DOWN)):
		failures.append("sit: not SitGroundDown 96 -> SitGround 97 (%s)" % [sat])
	var stood: Array = timeline.get("stand", [])
	if not (stood.has(SIT_UP) and stood[-1] == STAND):
		failures.append("stand: not SitGroundUp 98 -> Stand 0 (%s)" % [stood])
	if not timeline.get("kneel", []).has(KNEEL):
		failures.append("kneel: never KneelLoop 115 (%s)" % [timeline.get("kneel", [])])
	if not failures.is_empty():
		fail("\n".join(failures) + "\ntimeline " + str(timeline))
		return
	print("FIXTURE STAND_STATE_OBSERVED %s" % timeline)
	quit(0)

func read_phase() -> String:
	var path := SYNC.path_join(PHASE_FILE)
	if not FileAccess.file_exists(path):
		return ""
	return FileAccess.get_file_as_string(path).strip_edges()

func find_other() -> Node3D:
	var deadline := Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var other := client.get_node_or_null("WorldUnits/" + OTHER) as Node3D
		if other != null and other.find_child("PlayerModel", true, false) != null:
			return other
	fail("%s never appeared with a PlayerModel" % OTHER)
	return null

## Turn the character toward `target` with the arrow keys; whether it already faces it.
func face(player: Node3D, target: Node3D) -> bool:
	await process_frame
	var d := target.global_position - player.global_position
	var error := angle_difference(player.rotation.y + PI / 2.0, atan2(d.x, d.z))
	if turn_sign == 0.0:
		var before := player.rotation.y
		push_key(KEY_RIGHT, true)
		for _frame in 10:
			await process_frame
		push_key(KEY_RIGHT, false)
		await process_frame
		turn_sign = 1.0 if angle_difference(before, player.rotation.y) >= 0.0 else -1.0
		return false
	push_key(KEY_RIGHT, abs(error) > YAW_TOLERANCE and sign(error) == turn_sign)
	push_key(KEY_LEFT, abs(error) > YAW_TOLERANCE and sign(error) == -turn_sign)
	return abs(error) <= YAW_TOLERANCE

func release_turn() -> void:
	push_key(KEY_RIGHT, false)
	push_key(KEY_LEFT, false)

# --- Shared ------------------------------------------------------------------------

func enter_world() -> bool:
	if not await wait_until(func(state): return state.screen == "CharacterSelect" and state.reply_received, 60000, "CharacterSelect"):
		return false
	var ui: Node = null
	var card: Control = null
	var ui_deadline := Time.get_ticks_msec() + 180000
	while card == null:
		if Time.get_ticks_msec() > ui_deadline:
			fail("CharacterSelectUI never showed CharCard_0")
			return false
		await process_frame
		ui = client.get_node_or_null("CharacterSelectUI")
		card = ui.find_child("CharCard_0", true, false) as Control if ui != null else null
	await click_control(card)
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
	if not await wait_until(ready, 300000, "InWorld with terrain"):
		return false
	for _frame in 60:
		await process_frame
	print("FIXTURE IN_WORLD ", NAME)
	return true

func wait_until(predicate: Callable, timeout_ms: int, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call(client.account_state()):
			return true
	fail("Timed out waiting for %s: %s" % [what, client.account_state()])
	return false

func capture(file: String) -> void:
	await RenderingServer.frame_post_draw
	var path := shots.path_join(file)
	var error := root.get_texture().get_image().save_png(path)
	if error != OK:
		fail("Could not save %s: %s" % [path, error])
		return
	print("FIXTURE CAPTURE ", path)

func tap(code: Key) -> void:
	push_key(code, true)
	for _frame in 3:
		await process_frame
	push_key(code, false)
	await process_frame
	await process_frame

func wait_ms(ms: int) -> void:
	var until := Time.get_ticks_msec() + ms
	while Time.get_ticks_msec() < until:
		await process_frame

func push_key(code: Key, pressed: bool, unicode: int = 0) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.unicode = unicode if pressed else 0
	event.pressed = pressed
	root.push_input(event, true)

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
	print("FIXTURE FAIL ", message)
	quit(1)
