extends "res://tests/startup_game_menu.gd"

const SCALE := 1.25
const EPSILON := 2.0

func _initialize() -> void:
	Engine.max_fps = 0
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1600, 900)
	var config_dir := OS.get_environment("XDG_CONFIG_HOME")
	if config_dir.is_empty():
		fail("Drag fixture requires isolated XDG_CONFIG_HOME")
		return
	var path := config_dir.path_join("world-of-osso/options_settings.ron")
	if not FileAccess.file_exists(path):
		fail("Drag fixture requires canonical options file")
		return
	var initial := FileAccess.get_file_as_string(path)
	if not initial.contains("uiScale: 1.0") or not initial.contains("modal_offset: Some((0.0, 0.0))"):
		fail("Drag fixture requires default UI scale and centered canonical offset")
		return
	write_options(path, initial.replace("uiScale: 1.0", "uiScale: 1.25"))
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await open_options(client):
		return
	var panel := option_control(client, "OptionsRoot")
	var handle := option_control(client, "OptionsDragHandle")
	if panel == null or handle == null or not handle.is_visible_in_tree():
		fail("Authored Options root or drag handle missing")
		return
	if not expect_center(panel, Vector2.ZERO, "initial centered position"):
		return
	var from := handle.get_global_rect().position + Vector2(40, 30)
	await drag(from, from + Vector2(100, 40))
	if not expect_center(panel, Vector2(100, 40), "immediate scaled drag motion"):
		return
	var saved := FileAccess.get_file_as_string(path)
	if not saved.contains("modal_offset: Some((80.0, -32.0))") or saved.contains("modal_position: Some("):
		fail("Release did not persist scaled centered offset and clear legacy top-left: " + saved)
		return
	await click_option(client, "OptionsTabsound")
	await click_option(client, "OptionsDefaultsButton")
	if not expect_center(panel, Vector2(100, 40), "category Defaults preserves placement"):
		return
	var tab := option_control(client, "OptionsTabsound")
	var tab_center := tab.get_global_rect().get_center()
	await drag(tab_center, tab_center + Vector2(100, 30))
	if not expect_center(panel, Vector2(100, 40), "non-title controls cannot drag"):
		return
	from = handle.get_global_rect().position + Vector2(40, 30)
	await drag(from, Vector2(-2000, -2000))
	if not expect_center(panel, Vector2(-210, -70) * SCALE, "negative bound uses virtual 860x580"):
		return
	var handle_rect := handle.get_global_rect()
	from = Vector2(maxf(handle_rect.position.x + 180, 30), maxf(handle_rect.position.y + 50, 10))
	await drag(from, Vector2(4000, 4000))
	if not expect_center(panel, Vector2(210, 70) * SCALE, "positive bound uses virtual 860x580"):
		return
	if not FileAccess.get_file_as_string(path).contains("modal_offset: Some((210.0, -70.0))"):
		fail("Bounded release was not persisted")
		return
	client.free()
	var reloaded: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(reloaded)
	if not await open_options(reloaded):
		return
	if not expect_center(option_control(reloaded, "OptionsRoot"), Vector2(210, 70) * SCALE, "reloaded canonical offset"):
		return
	reloaded.free()
	var legacy := FileAccess.get_file_as_string(path).replace(
		"modal_offset: Some((210.0, -70.0))",
		"modal_offset: None,\n    modal_position: Some((100.0, 50.0))"
	)
	write_options(path, legacy)
	var converted: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(converted)
	if not await open_options(converted):
		return
	if not expect_center(option_control(converted, "OptionsRoot"), Vector2(-110, -20) * SCALE, "legacy top-left converted on load"):
		return
	converted.free()
	write_options(path, legacy.replace("modal_offset: None", "modal_offset: Some((9999.0, 9999.0))"))
	var preferred: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(preferred)
	if not await open_options(preferred):
		return
	if not expect_center(option_control(preferred, "OptionsRoot"), Vector2(210, -70) * SCALE, "saved offset preferred and clamped on load"):
		return
	print("PASS: authored title capture, scale, clamp, release persistence, reload, legacy conversion, preferred offset, Defaults and non-title rejection")
	quit(0)

func open_options(client: Node) -> bool:
	if not await wait_for_startup_menu(client):
		return false
	for frame in range(3):
		await process_frame
	await click_menu_action(client, "MenuBtnOptions")
	for frame in range(4):
		await process_frame
	if option_control(client, "OptionsDoneButton") == null:
		fail("Authored Options did not open")
		return false
	return true

func drag(from: Vector2, to: Vector2) -> void:
	var hover := InputEventMouseMotion.new()
	hover.position = from
	hover.global_position = from
	root.push_input(hover, true)
	await process_frame
	var button := InputEventMouseButton.new()
	button.button_index = MOUSE_BUTTON_LEFT
	button.position = from
	button.global_position = from
	button.pressed = true
	root.push_input(button, true)
	await process_frame
	var motion := InputEventMouseMotion.new()
	motion.position = to
	motion.global_position = to
	motion.relative = to - from
	motion.button_mask = MOUSE_BUTTON_MASK_LEFT
	root.push_input(motion, true)
	await process_frame
	button.position = to
	button.global_position = to
	button.pressed = false
	root.push_input(button, true)
	await process_frame

func expect_center(panel: Control, offset: Vector2, stage: String) -> bool:
	if panel == null:
		fail(stage + ": Options panel missing")
		return false
	var actual := panel.get_global_rect().get_center()
	var wanted := Vector2(root.size) / 2.0 + offset
	if actual.distance_to(wanted) > EPSILON:
		fail("%s: panel center %s expected %s" % [stage, actual, wanted])
		return false
	return true

func option_control(client: Node, name: String) -> Control:
	var menu := client.get_node_or_null("GameMenuUI")
	return menu.find_child(name, true, false) as Control if menu != null else null

func click_option(client: Node, name: String) -> void:
	var control := option_control(client, name)
	if control == null:
		fail("Authored option control missing: " + name)
		return
	await click(control)

func write_options(path: String, contents: String) -> void:
	var file := FileAccess.open(path, FileAccess.WRITE)
	file.store_string(contents)
	file.close()
