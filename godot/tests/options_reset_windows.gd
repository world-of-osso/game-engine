extends "res://tests/world_menu_flow.gd"

const LAYOUT_PATH := "world-of-osso/ui_layout.ron"
const OPTIONS_PATH := "world-of-osso/options_settings.ron"

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var config := OS.get_environment("XDG_CONFIG_HOME")
	if config.is_empty():
		fail("Reset fixture requires isolated config")
		return
	var path := config.path_join(LAYOUT_PATH)
	var options := config.path_join(OPTIONS_PATH)
	var initial_options := FileAccess.get_file_as_bytes(options)
	if OS.get_environment("GODOT_TEST_RESET_VERIFY") == "1":
		if not FileAccess.get_file_as_string(options).contains("modal_offset:Some((80.0,-32.0))"):
			fail("Relaunch lost nondefault Options modal offset")
			return
		if check_persistence(path, options, initial_options):
			print("PASS: fresh process read reset layout and retained modal offset")
			quit(0)
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_screen(client, "Loading", 15000):
		return
	print("FIXTURE RESET_LOADING")
	if not await wait_screen(client, "InWorld", WORLD_WAIT_MS):
		return
	var roster: Dictionary = client.account_state()
	if roster.selected_character_id != 17:
		fail("Expected selected server character ID 17: " + str(roster))
		return
	for frame in range(3):
		await process_frame
	if not check_world_host_scales(client, 5.0 / 6.0):
		return
	if OS.get_environment("GODOT_TEST_MAP_VERIFY") == "1":
		if not await expect_book_restored(client, config):
			return
		await tap_map()
		var border := map_border(client)
		var expected_file := config.path_join("map-expected-position")
		var coords := FileAccess.get_file_as_string(expected_file).split(",")
		if border == null or coords.size() != 2:
			fail("Fresh process missing persisted map or expected position")
			return
		var expected := Vector2(float(coords[0]), float(coords[1]))
		if border.get_global_rect().position.distance_to(expected) > 2.0:
			fail("Fresh process map position %s expected %s" % [border.get_global_rect().position, expected])
			return
		print("FIXTURE MAP_REOPENED")
		await tap_map()
	else:
		if not await exercise_book_placement(client, path, config):
			return
		if not await exercise_map_placement(client, path):
			return
		print("FIXTURE MAP_SAVED")
		client.free()
		quit(0)
		return
	push_key(KEY_ESCAPE, true)
	await process_frame
	push_key(KEY_ESCAPE, false)
	if not await wait_menu(client):
		return
	await click_menu_action(client, "MenuBtnOptions")
	await process_frame
	var menu := client.get_node_or_null("GameMenuUI")
	var interface_tab := menu.find_child("OptionsTabinterface", true, false) as Control
	if interface_tab == null:
		fail("Authored Interface tab missing")
		return
	await click(interface_tab)
	var reset := menu.find_child("ActionButtonreset_window_positions", true, false) as Control
	if reset == null or not reset.is_visible_in_tree():
		fail("Authored Reset Window Positions button missing")
		return
	await click(reset)
	await process_frame
	if client.get_node_or_null("GameMenuUI") == null:
		fail("Reset closed Options")
		return
	if not check_persistence(path, options, initial_options):
		return
	await click(menu.find_child("OptionsDoneButton", true, false) as Control)
	await process_frame
	if client.get_node_or_null("GameMenuUI") != null:
		await click_menu_action(client, "MenuBtnResume")
		await process_frame
	if not await expect_book_default(client):
		return
	await tap_map()
	if not await expect_map_position(client):
		return
	await tap_map()
	client.free()
	var reloaded: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(reloaded)
	await process_frame
	if not check_persistence(path, options, initial_options):
		return
	print("PASS: authenticated authored reset cleared only ID 17, retained ID 18/edit layout/modal on reload")
	quit(0)

func check_world_host_scales(client: Node, scale: float) -> bool:
	for host_name in ["MainActionBarUI", "CastingBarUI", "UnitFramesUI"]:
		var host := client.get_node_or_null(host_name)
		var canvas := host.find_child("RegistryCanvas", true, false) as Control if host != null else null
		if canvas == null or not canvas.scale.is_equal_approx(Vector2.ONE * scale) or not canvas.size.is_equal_approx(Vector2(root.size) / scale):
			fail("World %s canvas did not fit viewport at scale %s" % [host_name, scale])
			return false
	for host_name in ["GameTooltipUI", "EntranceBarUI", "MirrorTimers", "WorldMapUI", "SpellBookUI", "MerchantUI"]:
		var host := client.get_node_or_null(host_name)
		if host == null:
			continue
		var canvas := host.find_child("RegistryCanvas", true, false) as Control
		if canvas == null or not canvas.scale.is_equal_approx(Vector2.ONE * scale) or not canvas.size.is_equal_approx(Vector2(root.size) / scale):
			fail("Visible %s canvas did not fit viewport at scale %s" % [host_name, scale])
			return false
	return true

func tap_book() -> void:
	push_key(KEY_P, true)
	await process_frame
	push_key(KEY_P, false)
	for frame in range(4):
		await process_frame

func book_root(client: Node) -> Control:
	var ui := client.get_node_or_null("SpellBookUI")
	return ui.find_child("SpellBookRoot", true, false) as Control if ui != null else null

func expect_book_default(client: Node) -> bool:
	await tap_book()
	var book := book_root(client)
	if book == null:
		fail("SpellBookRoot missing")
		return false
	var rect := book.get_global_rect()
	var scale := book.get_global_transform().get_scale().x
	# The clamp keeps the bottom tabs on screen: FRAME_TOTAL_H (919) = FRAME_H (883) + 36.
	var total_height := rect.size.y * 919.0 / 883.0
	var expected := Vector2(minf(16.0 * scale, root.size.x - rect.size.x), minf(104.0 * scale, root.size.y - total_height))
	if rect.position.distance_to(expected) > 2.0:
		fail("Spellbook default slot %s expected %s" % [rect.position, expected])
		return false
	await tap_book()
	return true

func exercise_book_placement(client: Node, path: String, config: String) -> bool:
	root.size = Vector2i(1920, 1080)
	for frame in range(5):
		await process_frame
	if not check_world_host_scales(client, 1.25):
		return false
	if not await expect_book_default(client):
		return false
	await tap_book()
	var book := book_root(client)
	var original := book.get_global_rect().position
	var scale := book.get_global_transform().get_scale().x
	if absf(scale - 1.25) > 0.01:
		fail("Expected effective nonunit UI scale 5/4, got " + str(scale))
		return false
	var close := client.get_node("SpellBookUI").find_child("SpellBookCloseButton", true, false) as Control
	if close == null:
		fail("Spellbook close button missing")
		return false
	await click(close)
	if client.get_node_or_null("SpellBookUI") != null:
		fail("Spellbook close button captured as drag")
		return false
	await tap_book()
	book = book_root(client)
	original = book.get_global_rect().position
	await drag_map(original + Vector2(100, 12), original + Vector2(112, -8))
	var moved := original + Vector2(12, -20)
	if book.get_global_rect().position.distance_to(moved) > 2.0:
		fail("Spellbook title drag failed: " + str(book.get_global_rect()))
		return false
	var layout := FileAccess.get_file_as_string(path)
	if not layout.contains("SpellBookRoot") or not layout.contains('"17"') or not layout.contains('"18"'):
		fail("Spellbook release did not persist character-scoped root: " + layout)
		return false
	var expected_file := FileAccess.open(config.path_join("book-expected-position"), FileAccess.WRITE)
	if expected_file == null:
		fail("Cannot save expected book position")
		return false
	expected_file.store_string("%f,%f" % [moved.x, moved.y])
	expected_file.close()
	await tap_book()
	await tap_book()
	book = book_root(client)
	if book == null or book.get_global_rect().position.distance_to(moved) > 2.0:
		fail("Spellbook reopen did not restore saved placement")
		return false
	var body := client.get_node("SpellBookUI").find_child("SpellBookNextPageButton", true, false) as Control
	if body == null:
		fail("Spellbook body button missing")
		return false
	await click(body)
	if book.get_global_rect().position.distance_to(moved) > 2.0:
		fail("Spellbook body button dragged frame")
		return false
	root.size = Vector2i(900, 600)
	for frame in range(5):
		await process_frame
	var rect := book.get_global_rect()
	if rect.position.x < -2 or rect.position.y < -2 or rect.end.x > root.size.x + 2 or rect.end.y > root.size.y + 2:
		fail("Resized spellbook escaped screen: " + str(rect))
		return false
	await tap_book()
	root.size = Vector2i(1280, 720)
	for frame in range(5):
		await process_frame
	return true

func expect_book_restored(client: Node, config: String) -> bool:
	root.size = Vector2i(1920, 1080)
	for frame in range(5):
		await process_frame
	await tap_book()
	var book := book_root(client)
	var coords := FileAccess.get_file_as_string(config.path_join("book-expected-position")).split(",")
	if book == null or coords.size() != 2:
		fail("Fresh process missing spellbook or expected position")
		return false
	var expected := Vector2(float(coords[0]), float(coords[1]))
	if book.get_global_rect().position.distance_to(expected) > 2.0:
		fail("Fresh process spellbook %s expected %s" % [book.get_global_rect().position, expected])
		return false
	await tap_book()
	root.size = Vector2i(1280, 720)
	for frame in range(5):
		await process_frame
	return true

func tap_map() -> void:
	push_key(KEY_M, true)
	await process_frame
	push_key(KEY_M, false)
	for frame in range(4):
		await process_frame

func map_border(client: Node) -> Control:
	var ui := client.get_node_or_null("WorldMapUI")
	return ui.find_child("WorldMapBorderFrame", true, false) as Control if ui != null else null

func expect_map_position(client: Node) -> bool:
	var border := map_border(client)
	if border == null:
		fail("WorldMapBorderFrame missing")
		return false
	var rect := border.get_global_rect()
	var scale := border.get_global_transform().get_scale().x
	if absf(scale - 5.0 / 6.0) > 0.01:
		fail("Expected effective nonunit UI scale 5/6, got " + str(scale))
		return false
	# Left UI panel slot (world_map_frame_component::PANEL_SLOT): LEFT_OFFSET 16, TOP_OFFSET -116.
	var expected := Vector2(minf(16.0 * scale, root.size.x - rect.size.x), minf(116.0 * scale, root.size.y - rect.size.y))
	if rect.position.distance_to(expected) > 2.0:
		fail("Map position %s, expected %s" % [rect.position, expected])
		return false
	return true

func exercise_map_placement(client: Node, path: String) -> bool:
	await tap_map()
	var border := map_border(client)
	if border == null or not await expect_map_position(client):
		return false
	var start := border.get_global_rect().position
	var from := start + Vector2(80, 12)
	var to := from + Vector2(64, -32)
	await drag_map(from, to)
	var moved := start + Vector2(64, -32)
	if border.get_global_rect().position.distance_to(moved) > 2.0:
		fail("Title drag failed: " + str(border.get_global_rect()))
		return false
	if not FileAccess.get_file_as_string(path).contains("WorldMapFrame"):
		fail("Release did not persist WorldMapFrame")
		return false
	var config := OS.get_environment("XDG_CONFIG_HOME")
	var expected_file := FileAccess.open(config.path_join("map-expected-position"), FileAccess.WRITE)
	if expected_file == null:
		fail("Cannot save expected map position")
		return false
	expected_file.store_string("%f,%f" % [moved.x, moved.y])
	expected_file.close()
	await tap_map()
	await tap_map()
	border = map_border(client)
	if border == null or border.get_global_rect().position.distance_to(moved) > 2.0:
		fail("Map reopen did not restore saved placement")
		return false
	var canvas := client.get_node("WorldMapUI").find_child("WorldMapCanvas", true, false) as Control
	if canvas == null:
		fail("Map canvas missing")
		return false
	var previous_map: int = client.world_map_state().map_id
	await click_map_canvas(canvas, MOUSE_BUTTON_RIGHT)
	if client.world_map_state().map_id == previous_map:
		fail("Canvas right-click did not navigate out")
		return false
	if border.get_global_rect().position.distance_to(moved) > 2.0:
		fail("Canvas click dragged map")
		return false
	var close := client.get_node("WorldMapUI").find_child("WorldMapCloseButton", true, false) as Control
	if close == null:
		fail("Map close button missing")
		return false
	await click(close)
	if client.get_node_or_null("WorldMapUI") != null:
		fail("Map close button was captured as a drag")
		return false
	await tap_map()
	root.size = Vector2i(900, 600)
	for frame in range(5):
		await process_frame
	border = map_border(client)
	var rect := border.get_global_rect()
	if rect.position.x < -2 or rect.position.y < -2 or rect.end.x > root.size.x + 2 or rect.end.y > root.size.y + 2:
		fail("Resized map escaped screen: " + str(rect))
		return false
	root.size = Vector2i(1280, 720)
	await tap_map()
	return true

func click_map_canvas(canvas: Control, button_index: MouseButton) -> void:
	var point := canvas.get_global_rect().get_center()
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = button_index
		event.position = point
		event.global_position = point
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame

func drag_map(from: Vector2, to: Vector2) -> void:
	var down := InputEventMouseButton.new()
	down.button_index = MOUSE_BUTTON_LEFT
	down.position = from
	down.global_position = from
	down.pressed = true
	root.push_input(down, true)
	await process_frame
	var motion := InputEventMouseMotion.new()
	motion.position = to
	motion.global_position = to
	motion.relative = to - from
	motion.button_mask = MOUSE_BUTTON_MASK_LEFT
	root.push_input(motion, true)
	await process_frame
	down.position = to
	down.global_position = to
	down.pressed = false
	root.push_input(down, true)
	await process_frame

func check_persistence(path: String, options: String, initial_options: PackedByteArray) -> bool:
	var layout := FileAccess.get_file_as_string(path)
	if layout.contains("\"17\"") or not layout.contains("\"18\"") or not layout.contains("CharacterFrame") or not layout.contains("SpellBookRoot") or not layout.contains("70.0") or not layout.contains("90.0") or not layout.contains("75.0") or not layout.contains("80.0") or not layout.contains("PlayerFrame") or not layout.contains("12.0") or not layout.contains("24.0") or not layout.contains("active_layout") or not layout.contains("Layout 1"):
		fail("Reset changed other character or edit-mode layout: " + layout)
		return false
	if FileAccess.get_file_as_bytes(options) != initial_options:
		fail("Reset changed saved Options modal position")
		return false
	return true
