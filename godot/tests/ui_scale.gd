extends "res://tests/startup_game_menu.gd"

func _initialize() -> void:
	Engine.max_fps = 0
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var config_dir := OS.get_environment("XDG_CONFIG_HOME")
	if config_dir.is_empty():
		fail("UI scale fixture requires isolated XDG_CONFIG_HOME")
		return
	var options_path := config_dir.path_join("world-of-osso/options_settings.ron")
	if not FileAccess.file_exists(options_path):
		fail("UI scale fixture requires canonical options file")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_for_startup_menu(client):
		return
	await click_menu_action(client, "MenuBtnOptions")
	var menu := client.get_node("GameMenuUI") as CanvasLayer
	await click_control(menu, "OptionsTabaccessibility")
	var slider := find_control(menu, "Sliderui_scale")
	if slider == null:
		fail("Authored UI scale slider missing")
		return
	await drag_slider(slider, 0.0)
	if not await expect_scale(menu, 0.75, options_path):
		return
	await click_control(menu, "OptionsTabaccessibility")
	await drag_slider(slider, 0.6666667)
	if not await expect_scale(menu, 1.25, options_path):
		return
	root.size = Vector2i(1600, 900)
	for frame in range(4):
		await process_frame
	if not check_geometry(menu, 1.25):
		return
	await click_control(menu, "OptionsTabgraphics")
	var graphics := find_control(menu, "Sliderframe_rate_limit")
	if graphics == null or not graphics.is_visible_in_tree():
		fail("Scaled Graphics tab did not switch category")
		return
	await click_control(menu, "OptionsDoneButton")
	if client.get_node_or_null("GameMenuUI") != null:
		fail("Scaled Done button did not close Options")
		return
	print("PASS: native UI scale .75/1.25, resize recenter, slider and button pixel input")
	quit(0)

func find_control(menu: CanvasLayer, name: String) -> Control:
	return menu.find_child(name, true, false) as Control

func click_control(menu: CanvasLayer, name: String) -> void:
	var control := find_control(menu, name)
	if control == null or not control.is_visible_in_tree():
		fail("Authored control absent: " + name)
		return
	await click(control)

func drag_slider(slider: Control, percent: float) -> void:
	var rect := slider.get_global_rect()
	var position := Vector2(lerpf(rect.position.x, rect.end.x, percent), rect.get_center().y)
	var down := InputEventMouseButton.new()
	down.button_index = MOUSE_BUTTON_LEFT
	down.position = position
	down.global_position = position
	down.pressed = true
	root.push_input(down, true)
	await process_frame
	down.pressed = false
	root.push_input(down, true)
	for frame in range(3):
		await process_frame

func expect_scale(menu: CanvasLayer, wanted: float, path: String) -> bool:
	for frame in range(3):
		await process_frame
	var saved := FileAccess.get_file_as_string(path)
	if not saved.contains("uiScale: " + str(wanted)):
		fail("UI scale slider did not persist %s: %s" % [wanted, saved])
		return false
	return check_geometry(menu, wanted)

func check_geometry(menu: CanvasLayer, scale: float) -> bool:
	var canvas := menu.find_child("RegistryCanvas", true, false) as Control
	var panel := find_control(menu, "OptionsRoot")
	if canvas == null or panel == null:
		fail("Native projection or Options root absent")
		return false
	var physical := Vector2(root.size)
	var logical := physical / scale
	if not canvas.scale.is_equal_approx(Vector2.ONE * scale) or not canvas.size.is_equal_approx(logical):
		fail("Canvas geometry size=%s scale=%s expected size=%s scale=%s" % [canvas.size, canvas.scale, logical, scale])
		return false
	if not panel.get_global_rect().get_center().distance_to(physical / 2.0) < 2.0:
		fail("Options panel did not recenter at %s: %s" % [scale, panel.get_global_rect()])
		return false
	return true
