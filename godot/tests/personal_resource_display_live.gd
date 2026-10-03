extends SceneTree

## Retail Personal Resource Display against a private server.
## Environment:
##   GODOT_TEST_SERVER          server address (a private test server)
##   PRD_ACCOUNT / PRD_CHARACTER account (password fbtest) and its only character, a rogue
##                              placed offline near a Blackrock Worg
##   PRD_SHOTS                  screenshot directory
## Off by default (`nameplateShowSelf` 0). Options → HUD → Personal Resource Display, Done,
## saves `nameplateShowSelf: true` and shows PersonalResourceDisplayFrame (health, energy and
## its own five combo points) at the Modern preset; the fixture then fights a worg with
## Sinister Strike and captures the display in combat with a lit combo point.

const PASSWORD := "fbtest"
const MOB := "Blackrock Worg"
const MELEE_REACH := 4.5
const SINISTER_STRIKE := 1752
const BAR_KEYS := [KEY_1, KEY_2, KEY_3, KEY_4, KEY_5, KEY_6, KEY_7, KEY_8, KEY_9, KEY_0, KEY_MINUS, KEY_EQUAL]
const PIP := "PersonalResourceDisplayPlayerSecondaryResourcePip%d"

var client: Node
var shots := ""
var local_id := 0

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("PRD_ACCOUNT")
	shots = OS.get_environment("PRD_SHOTS")
	if server == "" or account == "" or shots == "":
		fail("GODOT_TEST_SERVER, PRD_ACCOUNT and PRD_SHOTS are required")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world():
		return
	local_id = client.account_state().local_player_id
	if control("UnitFramesUI", "PersonalResourceDisplayFrame") != null:
		fail("The display shows before the option is on")
		return
	await capture("00-off.png")
	if not await enable_option():
		return
	var shown := func(): return visible("PersonalResourceDisplayFrame") and control("UnitFramesUI", PIP % 4 + "BGShadow") != null
	if not await wait_until(shown, 5000, "the display with five combo points"):
		dump()
		return
	if visible("PersonalResourceDisplayAlternatePowerBar") or control("UnitFramesUI", PIP % 5 + "BGShadow") != null:
		fail("A rogue shows an alternate power bar or a sixth point")
		return
	dump()
	await capture("01-enabled.png")
	if not await fight():
		return
	dump()
	await capture("02-combat.png")
	print("FIXTURE PRD_LIVE_DONE")
	client.free()
	quit(0)

## Options → HUD → Personal Resource Display, Done; the choice persists as `nameplateShowSelf`.
func enable_option() -> bool:
	await press(KEY_ESCAPE)
	if not await wait_until(func(): return control("GameMenuUI", "MenuBtnOptions") != null, 5000, "game menu"):
		return false
	for name in ["MenuBtnOptions", "OptionsTabhud", "ToggleSwitchpersonal_resource_display", "OptionsDoneButton"]:
		var target := control("GameMenuUI", name)
		if target == null or not target.is_visible_in_tree():
			fail("Options control absent: " + name)
			return false
		await click(target)
	if control("GameMenuUI", "MenuBtnResume") != null:
		await click(control("GameMenuUI", "MenuBtnResume"))
	var path := OS.get_environment("XDG_CONFIG_HOME").path_join("world-of-osso/options_settings.ron")
	var file := FileAccess.get_file_as_string(path).replace(" ", "")
	if not file.contains("nameplateShowSelf:true"):
		fail("Options did not save nameplateShowSelf: " + path)
		return false
	print("FIXTURE OPTION nameplateShowSelf:true saved")
	return true

## Tab to a worg in melee reach and Sinister Strike it until in combat with a lit combo point.
func fight() -> bool:
	var target := 0
	for attempt in range(40):
		target = await worg_in_reach()
		if target != 0:
			break
		await wait_frames(120)
	if target == 0:
		fail("No %s within %.1f yd" % [MOB, MELEE_REACH])
		return false
	var strike: int = client.spells_state().bar.find(SINISTER_STRIKE)
	if strike < 0:
		fail("Sinister Strike not on the bar: %s" % [client.spells_state().bar])
		return false
	# Sinister Strike starts the fight and awards a combo point.
	var deadline := Time.get_ticks_msec() + 20000
	while Time.get_ticks_msec() < deadline and not (lit(0) and visible("PlayerCombatIcon")):
		await press(BAR_KEYS[strike])
		await wait_frames(30)
	if not (lit(0) and visible("PlayerCombatIcon")):
		fail("No combat with a lit combo point: combat=%s lit=%s %s" % [visible("PlayerCombatIcon"), lit(0), client.target_state()])
		return false
	# Let the point's activate animation settle.
	await wait_frames(90)
	print("FIXTURE COMBAT target=%s lit=%s" % [client.target_state().target_name, range(5).filter(func(i): return lit(i))])
	return true

func worg_in_reach() -> int:
	for attempt in range(30):
		await press(KEY_TAB)
		await wait_frames(6)
		var state: Dictionary = client.target_state()
		if state.target != null and str(state.target_name).contains(MOB):
			var distance: float = client.unit_transform(local_id).origin.distance_to(client.unit_transform(state.target).origin)
			if distance < MELEE_REACH:
				print("FIXTURE TARGET %s distance=%.2f" % [state.target_name, distance])
				return state.target
	return 0

## `RogueComboPointTemplate` lit art: the red `IconUncharged`.
func lit(index: int) -> bool:
	return visible(PIP % index + "IconUncharged")

func dump() -> void:
	for name in ["PersonalResourceDisplayFrame", "PersonalResourceDisplayHealthBar", "PersonalResourceDisplayHealthBarFill", "PersonalResourceDisplayPowerBar", "PersonalResourceDisplayPowerBarFill", "PersonalResourceDisplayClassFrame", "PlayerCombatIcon"]:
		var node := control("UnitFramesUI", name)
		print("FIXTURE %s visible=%s rect=%s" % [name, node != null and node.is_visible_in_tree(), node.get_global_rect() if node != null else null])

func visible(name: String) -> bool:
	var node := control("UnitFramesUI", name)
	return node != null and node.is_visible_in_tree()

func enter_world() -> bool:
	var deadline := Time.get_ticks_msec() + 60000
	var ui = null
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		ui = client.get_node_or_null("CharacterSelectUI")
		if state.reply_received and state.screen == "CharacterSelect" and state.character_count >= 1 and ui != null:
			break
	if ui == null:
		fail("No character select: " + str(client.account_state()))
		return false
	await click(ui.find_child("CharCard_0", true, false))
	await click(ui.find_child("EnterWorld", true, false))
	deadline = Time.get_ticks_msec() + 300000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "InWorld" and state.local_player_position != null and state.terrain.pending_count == 0 and not state.terrain.parsed_tiles.is_empty():
			print("FIXTURE IN_WORLD at ", state.local_player_position)
			await wait_frames(60)
			return true
	fail("Timed out entering the world: " + str(client.account_state()))
	return false

func control(host: String, name: String) -> Control:
	var ui := client.get_node_or_null(host)
	return ui.find_child(name, true, false) as Control if ui != null else null

func wait_until(predicate: Callable, timeout_ms: int, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call():
			return true
	fail("Timed out waiting for %s: target=%s" % [what, client.target_state()])
	return false

func press(code: Key) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = code
		event.physical_keycode = code
		event.pressed = pressed
		root.push_input(event, true)
		await wait_frames(2)

func move_mouse(point: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	Input.warp_mouse(point)
	await wait_frames(3)

func click(target: Control) -> void:
	var point := target.get_global_rect().get_center()
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

func capture(name: String) -> void:
	await RenderingServer.frame_post_draw
	var path := shots.path_join(name)
	var error := root.get_texture().get_image().save_png(path)
	if error != OK:
		fail("Could not save %s: %s" % [path, error])
	else:
		print("FIXTURE SHOT ", path)

func wait_frames(count: int) -> void:
	for frame in range(count):
		await process_frame

func fail(message: String) -> void:
	push_error(message)
	quit(1)
