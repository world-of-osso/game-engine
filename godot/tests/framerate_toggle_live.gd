extends "res://tests/new_class_live.gd"

## Retail `TOGGLEFPS` (Bindings_Standard.xml: `FramerateFrame:Toggle()`; FramerateFrame.xml
## `hidden="true"`): in the world the FPS overlay starts hidden with default options, Ctrl+R
## shows it, plain R does not, and Ctrl+R hides it again. Environment:
##   GODOT_TEST_SERVER   private test server
##   FRAMERATE_ACCOUNT   account (password fbtest) with FRAMERATE_CHARACTER on its roster
##   XDG_CONFIG_HOME     an owned directory without world-of-osso/options_settings.ron

func run_test() -> void:
	var config := OS.get_environment("XDG_CONFIG_HOME")
	if config == "" or FileAccess.file_exists(config.path_join("world-of-osso/options_settings.ron")):
		fail("XDG_CONFIG_HOME must be owned and hold no saved options, so defaults apply")
		return
	var character := OS.get_environment("FRAMERATE_CHARACTER")
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(OS.get_environment("GODOT_TEST_SERVER"), OS.get_environment("FRAMERATE_ACCOUNT"), PASSWORD, false)
	if error != "":
		fail("Connection: " + error)
		return
	if not await wait_state(func(s): return s.reply_received and s.screen == "CharacterSelect" and s.character_count > 0, 20000, "character select"):
		return
	await wait_frames(30)
	if not await enter_world(character):
		return
	var overlay := client.get_node("FpsOverlay") as CanvasLayer
	if overlay.visible or client.fps_overlay_enabled():
		fail("FPS overlay is shown by default in the world")
		return
	for step in [["ctrl", true], ["plain", true], ["ctrl", false]]:
		await press_r(step[0] == "ctrl")
		await wait_frames(3)
		if overlay.visible != step[1] or client.fps_overlay_enabled() != step[1]:
			fail("after %s R the overlay visible=%s, expected %s" % [step[0], overlay.visible, step[1]])
			return
		print("FRAMERATE %s R -> visible=%s" % [step[0], overlay.visible])
	client.free()
	print("PASS: default-hidden FPS overlay, Ctrl+R shows and hides it, plain R does not")
	quit(0)

func press_r(ctrl: bool) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = KEY_R
		event.physical_keycode = KEY_R
		event.pressed = pressed
		event.ctrl_pressed = ctrl
		root.push_input(event, true)
		await wait_frames(2)
