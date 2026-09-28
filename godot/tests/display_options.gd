extends "res://tests/startup_game_menu.gd"

const MIN_FPS := 30
const MAX_FPS := 240
const DEFAULT_FPS := 144

func _initialize() -> void:
	Engine.max_fps = 0
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_for_startup_menu(client):
		return
	if not expect_display(DisplayServer.VSYNC_MAILBOX, 0, "default graphics"):
		return
	await click_menu_action(client, "MenuBtnOptions")
	if not await wait_for_graphics(client):
		return
	await click_option(client, "ToggleSwitchvsync_enabledLeftHit")
	if not expect_display(DisplayServer.VSYNC_DISABLED, 0, "vsync disabled"):
		return
	await click_option(client, "ToggleSwitchframe_rate_limit_enabledRightHit")
	if not expect_display(DisplayServer.VSYNC_DISABLED, DEFAULT_FPS, "frame cap enabled"):
		return
	await set_slider_end(client, false)
	if not expect_display(DisplayServer.VSYNC_DISABLED, MIN_FPS, "frame cap minimum"):
		return
	await set_slider_end(client, true)
	if not expect_display(DisplayServer.VSYNC_DISABLED, MAX_FPS, "frame cap maximum"):
		return
	await click_option(client, "ToggleSwitchvsync_enabledRightHit")
	if not expect_display(DisplayServer.VSYNC_MAILBOX, MAX_FPS, "vsync restored"):
		return
	await click_option(client, "ToggleSwitchframe_rate_limit_enabledLeftHit")
	if not expect_display(DisplayServer.VSYNC_MAILBOX, 0, "frame cap disabled"):
		return
	print("PASS: default, live vsync, live FPS cap, range endpoints, and cap reset")
	quit(0)

func wait_for_graphics(client: Node) -> bool:
	var deadline := Time.get_ticks_msec() + MENU_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var graphics := option_control(client, "OptionsTabgraphics")
		if graphics != null and graphics.is_visible_in_tree():
			await click(graphics)
			if option_control(client, "ToggleSwitchvsync_enabled") != null:
				return true
	fail("Authored graphics options did not open")
	return false

func option_control(client: Node, name: String) -> Control:
	var menu := client.get_node_or_null("GameMenuUI")
	return menu.find_child(name, true, false) as Control if menu != null else null

func click_option(client: Node, name: String) -> void:
	var control := option_control(client, name)
	if control == null or not control.is_visible_in_tree():
		fail("Authored graphics control missing: " + name)
		return
	await click(control)

func set_slider_end(client: Node, maximum: bool) -> void:
	var slider := option_control(client, "Sliderframe_rate_limit")
	if slider == null or not slider.is_visible_in_tree():
		fail("Authored frame rate slider missing")
		return
	var rect := slider.get_global_rect()
	var start := rect.get_center()
	var end := Vector2(rect.end.x + 10.0 if maximum else rect.position.x - 10.0, start.y)
	var press := InputEventMouseButton.new()
	press.position = start
	press.global_position = start
	press.button_index = MOUSE_BUTTON_LEFT
	press.pressed = true
	root.push_input(press, true)
	await process_frame
	var motion := InputEventMouseMotion.new()
	motion.position = end
	motion.global_position = end
	motion.relative = end - start
	motion.button_mask = MOUSE_BUTTON_MASK_LEFT
	root.push_input(motion, true)
	await process_frame
	press.position = end
	press.global_position = end
	press.pressed = false
	root.push_input(press, true)
	await process_frame

func expect_display(vsync: DisplayServer.VSyncMode, fps: int, stage: String) -> bool:
	var actual_vsync := DisplayServer.window_get_vsync_mode()
	var actual_fps := Engine.max_fps
	if actual_vsync != vsync or actual_fps != fps:
		fail("%s: vsync=%s, max_fps=%d; expected vsync=%s, max_fps=%d" % [stage, actual_vsync, actual_fps, vsync, fps])
		return false
	return true
