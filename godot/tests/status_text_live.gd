extends SceneTree

## Retail Status Text on the unit frames against a private server.
## Environment:
##   GODOT_TEST_SERVER                server address (a private test server)
##   STATUS_ACCOUNT / STATUS_CHARACTER account (password fbtest) and a mana-class character
##   STATUS_SHOTS                     screenshot directory
## Default None hides the bar text until the bar is hovered (numeric then); Interface →
## Status Text Percentage and Both, chosen through Options, redraw the PlayerFrame and the
## self-targeted TargetFrame bars.

const PASSWORD := "fbtest"
const PERCENT := "^\\d+%$"
const VALUE := "^[\\d,]+( [KM])?$"
const NUMERIC := "^[\\d,]+( [KM])? / [\\d,]+( [KM])?$"

var client: Node
var shots := "/tmp/claude/status-text-live/"
var character := ""

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("STATUS_ACCOUNT")
	character = OS.get_environment("STATUS_CHARACTER")
	if server == "" or account == "" or character == "":
		fail("GODOT_TEST_SERVER, STATUS_ACCOUNT and STATUS_CHARACTER are required")
		return
	if OS.get_environment("STATUS_SHOTS") != "":
		shots = OS.get_environment("STATUS_SHOTS")
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world():
		return
	if not await check_none_hover():
		return
	await capture("00-none.png")
	if not await choose(2, "PERCENT"):
		return
	if not await expect_texts("Percentage", {
		"PlayerHealthBarText": PERCENT, "PlayerManaBarText": PERCENT,
		"TargetHealthBarText": PERCENT, "TargetManaBarText": PERCENT,
	}):
		return
	await capture("01-percentage.png")
	if not await choose(3, "BOTH"):
		return
	if not await expect_texts("Both", {
		"PlayerHealthBarTextLeft": PERCENT, "PlayerHealthBarTextRight": VALUE,
		"PlayerManaBarTextLeft": PERCENT, "PlayerManaBarTextRight": VALUE,
		"TargetHealthBarTextLeft": PERCENT, "TargetHealthBarTextRight": VALUE,
	}):
		return
	for centre in ["PlayerHealthBarText", "PlayerManaBarText", "TargetHealthBarText"]:
		if control("UnitFramesUI", centre).is_visible_in_tree():
			fail("Both still shows the centred %s" % centre)
			return
	await capture("02-both.png")
	print("FIXTURE STATUS_TEXT_LIVE_DONE")
	client.free()
	quit(0)

## Retail default None: no PlayerFrame bar text until the pointer enters the bar, which
## then shows "value / max" (TextStatusBar.lua:115-123,170-175,217-220).
func check_none_hover() -> bool:
	var text := control("UnitFramesUI", "PlayerHealthBarText") as Label
	if text == null or text.is_visible_in_tree():
		fail("None shows PlayerHealthBarText: %s" % [text.text if text != null else "missing"])
		return false
	await move_mouse(control("UnitFramesUI", "PlayerHealthBar").get_global_rect().get_center())
	var shown := func(): return text.is_visible_in_tree() and RegEx.create_from_string(NUMERIC).search(text.text) != null
	if not await wait_until(shown, 3000, "hover text on PlayerHealthBar"):
		return false
	print("FIXTURE NONE_HOVER PlayerHealthBarText=%s" % text.text)
	await capture("00-none-hover.png")
	await move_mouse(Vector2(640, 360))
	if not await wait_until(func(): return not text.is_visible_in_tree(), 3000, "hover text hiding"):
		return false
	return true

## Options → Interface → Status Text choice `value`, Done; then F1 targets the player.
func choose(value: int, saved: String) -> bool:
	# Escape clears a target before it opens the game menu.
	if client.target_state().target != null:
		await press(KEY_ESCAPE)
	await press(KEY_ESCAPE)
	if not await wait_until(func(): return control("GameMenuUI", "MenuBtnOptions") != null, 5000, "game menu"):
		return false
	for name in ["MenuBtnOptions", "OptionsTabinterface", "Choicestatus_text_display%dHit" % value, "OptionsDoneButton"]:
		var target := control("GameMenuUI", name)
		if target == null or not target.is_visible_in_tree():
			fail("Options control absent: " + name)
			return false
		await click(target)
	if control("GameMenuUI", "MenuBtnResume") != null:
		await click(control("GameMenuUI", "MenuBtnResume"))
	var path := OS.get_environment("XDG_CONFIG_HOME").path_join("world-of-osso/options_settings.ron")
	var file := FileAccess.get_file_as_string(path).replace(" ", "")
	if not file.contains("statusTextDisplay:" + saved):
		fail("Options did not save statusTextDisplay %s: %s" % [saved, path])
		return false
	await press(KEY_F1)
	return await wait_until(func(): return client.target_state().target_name == character, 5000, "self target")

## Every named font string shown and matching its pattern.
func expect_texts(mode: String, patterns: Dictionary) -> bool:
	var matched := func():
		for name in patterns:
			var label := control("UnitFramesUI", name) as Label
			if label == null or not label.is_visible_in_tree() or RegEx.create_from_string(patterns[name]).search(label.text) == null:
				return false
		return true
	if not await wait_until(matched, 5000, mode + " texts"):
		for name in patterns:
			var label := control("UnitFramesUI", name) as Label
			push_error("%s %s visible=%s text=%s" % [mode, name, label.is_visible_in_tree() if label else null, label.text if label else null])
		return false
	var shown := []
	for name in patterns:
		shown.append("%s=%s" % [name, (control("UnitFramesUI", name) as Label).text])
	print("FIXTURE %s %s" % [mode.to_upper(), " ".join(shown)])
	return true

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

func capture(file: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var error := image.save_png(shots + file)
	if error != OK:
		fail("Could not save " + file + ": " + str(error))

func wait_frames(count: int) -> void:
	for frame in range(count):
		await process_frame

func fail(message: String) -> void:
	push_error(message)
	root.get_texture().get_image().save_png(shots + "fail.png")
	quit(1)
