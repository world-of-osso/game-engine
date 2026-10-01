extends "res://tests/world_spell_click_flow.gd"

const VENDOR := "Fixture Vendor"

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	var config := OS.get_environment("XDG_CONFIG_HOME")
	if not endpoint.begins_with("127.0.0.1:") or endpoint.ends_with(":0") or not config.contains("/data/native-reset-fixture-"):
		fail("Merchant-click fixture requires owned UDP endpoint and isolated options")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var loading_wait_at := Time.get_ticks_msec()
	if not await wait_screen(client, "Loading", 15000):
		return
	print("MERCHANT CLICK AUTH STARTUP loading_wait_ms=", Time.get_ticks_msec() - loading_wait_at)
	print("FIXTURE MERCHANT_CLICK_LOADING")
	if not await wait_world(client):
		return
	var sound := client.get_node_or_null("NativeSound")
	var effects := sound.get_node_or_null("Effects") as AudioStreamPlayer if sound != null else null
	if effects == null:
		fail("Authenticated GameClient has no owned Effects channel")
		return
	var completed := [0]
	effects.finished.connect(func(): completed[0] += 1)
	var vendor := await find_vendor(client)
	if vendor.is_empty():
		return
	if not await open_vendor(client, vendor):
		return
	var ui := client.get_node_or_null("MerchantUI")
	var bag := ui.find_child("ContainerFrame0", true, false) as Control if ui != null else null
	if bag == null or not bag.is_visible_in_tree():
		fail("Owned InventorySnapshot did not open the merchant backpack")
		return
	if not await check_merchant_placement(client, bag, config):
		return
	if not check_scaled_hosts(client, 1.25):
		return
	print("FIXTURE MERCHANT_CLICK_PLACED")
	var tab := ui.find_child("MerchantFrameTab2", true, false) as Control
	var saved := merchant_root(client).get_global_rect().position
	if tab == null or not await assert_click(client, effects, completed, tab, "merchant tab"):
		return
	var title := ui.find_child("MerchantFrameTitleText", true, false) as Label
	if title == null or title.text != "Merchant Buyback":
		fail("Authored merchant body tab did not select Buyback")
		return
	if merchant_root(client).get_global_rect().position.distance_to(saved) > 2:
		fail("Merchant tab captured title drag")
		return
	var close := ui.find_child("MerchantFrameCloseButton", true, false) as Control
	if close == null or not await assert_click(client, effects, completed, close, "merchant close"):
		return
	if client.merchant_state().open:
		fail("Merchant close action did not close vendor")
		return
	var clicks_after_close: int = completed[0]
	if not await open_vendor(client, vendor):
		return
	await wait_frames(8)
	if merchant_root(client).get_global_rect().position.distance_to(saved) > 2:
		fail("Merchant close/reopen lost saved root position")
		return
	if effects.is_playing() or completed[0] != clicks_after_close:
		fail("Reopening merchant replayed stale pointer effect")
		return
	ui = client.get_node_or_null("MerchantUI")
	tab = ui.find_child("MerchantFrameTab2", true, false) as Control if ui != null else null
	if tab == null or not await assert_quiet_input(effects, completed, tab, MOUSE_BUTTON_RIGHT):
		return
	var before: int = completed[0]
	pointer(tab, MOUSE_BUTTON_LEFT, false)
	await wait_frames(8)
	push_key(KEY_ESCAPE, true)
	await process_frame
	push_key(KEY_ESCAPE, false)
	await wait_frames(8)
	if effects.is_playing() or completed[0] != before:
		fail("Release or keyboard close played pointer-only effect")
		return
	if not await check_merchant_reset(client, vendor, config):
		return
	if not await check_live_options_scale(client, vendor):
		return
	print("FIXTURE MERCHANT_CLICK_DONE")
	client.free()
	quit(0)

func check_scaled_hosts(client: Node, scale: float) -> bool:
	for host_name in ["MerchantUI", "MainActionBarUI", "CastingBarUI", "UnitFramesUI", "GameMenuUI"]:
		var host := client.get_node_or_null(host_name)
		if host == null and host_name in ["MerchantUI", "GameMenuUI"]:
			continue
		var canvas := host.find_child("RegistryCanvas", true, false) as Control if host != null else null
		if canvas == null or not canvas.scale.is_equal_approx(Vector2.ONE * scale) or not canvas.size.is_equal_approx(Vector2(root.size) / scale):
			fail("Merchant world %s canvas not scaled to %s; actual=%s" % [host_name, scale, canvas.scale if canvas != null else null])
			return false
	return true

func check_live_options_scale(client: Node, vendor: Dictionary) -> bool:
	if not await open_menu_after_vendor(client, vendor):
		return false
	await click_menu_action(client, "MenuBtnOptions")
	var menu := client.get_node_or_null("GameMenuUI")
	await click(menu.find_child("OptionsTabaccessibility", true, false) as Control)
	var slider := menu.find_child("Sliderui_scale", true, false) as Control
	if slider == null:
		fail("Authored Options UI scale slider missing")
		return false
	await set_scale_slider(slider, 0.0)
	if not check_scaled_hosts(client, 0.75):
		return false
	var options := menu.find_child("OptionsRoot", true, false) as Control
	if options == null or not await capture_scaled_ui("merchant-options-075", options):
		return false
	if not check_options_description_bounds(menu, 0.75):
		return false
	if not OS.get_environment("GODOT_TEST_CAPTURE_DIR").is_empty():
		print_options_label_geometry(menu, "merchant-options-075")
	await click(menu.find_child("OptionsTabaccessibility", true, false) as Control)
	await set_scale_slider(slider, 0.6666667)
	if not check_scaled_hosts(client, 1.25):
		return false
	if not await capture_scaled_ui("merchant-options-125", options):
		return false
	if not check_options_description_bounds(menu, 1.25):
		return false
	if not OS.get_environment("GODOT_TEST_CAPTURE_DIR").is_empty():
		print_options_label_geometry(menu, "merchant-options-125")
	return true

func check_options_description_bounds(menu: CanvasLayer, scale: float) -> bool:
	var content := menu.find_child("OptionsContentPanel", true, false) as Control
	var options := menu.find_child("OptionsRoot", true, false) as Control
	if content == null or options == null:
		fail("Authored Options content/root missing at scale %s" % scale)
		return false
	var descriptions := {
		"access_text": "Scales the full HUD, menus, and overlays without changing 3D render resolution",
		"access_colorblind": "Swaps red/green status cues to higher-contrast colors for nameplates and debuff borders",
		"access_motion": "Animation dampening hooks reserved",
		"access_subtitles": "Dialog subtitle pipeline not landed yet",
	}
	var previous_bottom := -INF
	for key in descriptions:
		var label := menu.find_child("InfoDetail" + key, true, false) as Label
		if label == null or not label.is_visible_in_tree() or label.text != descriptions[key]:
			fail("Authored Options description missing/changed: %s at scale %s" % [key, scale])
			return false
		var row := label.get_parent() as Control
		var rect := label.get_global_rect()
		var right_limit := minf(row.get_global_rect().end.x, minf(content.get_global_rect().end.x, options.get_global_rect().end.x))
		if rect.size.x > 370.5 * scale or rect.end.x > right_limit + 1 or rect.position.y < previous_bottom - 1:
			fail("Options description exceeds authored row/content or overlaps previous row: %s scale=%s rect=%s limit=%s previous_bottom=%s" % [key, scale, rect, right_limit, previous_bottom])
			return false
		var long_text: bool = key == "access_text" or key == "access_colorblind"
		if long_text and (label.get_line_count() < 2 or label.get_visible_line_count() < 2):
			fail("Options description did not display full multiline text: %s scale=%s lines=%s visible=%s" % [key, scale, label.get_line_count(), label.get_visible_line_count()])
			return false
		if not long_text and (label.get_line_count() != 1 or absf(label.size.x - 370.0) > 0.5):
			fail("Short Options description changed layout: %s scale=%s size=%s lines=%s" % [key, scale, label.size, label.get_line_count()])
			return false
		previous_bottom = rect.end.y
	return true

func print_options_label_geometry(menu: CanvasLayer, capture: String) -> void:
	var options := menu.find_child("OptionsRoot", true, false) as Control
	var content := menu.find_child("OptionsContentPanel", true, false) as Control
	var host := menu.find_child("RegistryCanvas", true, false) as Control
	var options_rect := options.get_global_rect()
	var content_rect := content.get_global_rect() if content != null else Rect2()
	print("OPTIONS_LABEL_BOUNDS capture=%s root=%s content=%s content_present=%s" % [capture, options_rect, content_rect, content != null])
	# Projection can place labels beside, not below, their authored frame controls.
	for node in host.find_children("*", "Label", true, false):
		var label := node as Label
		var parent := label.get_parent() as Control
		var frame: Control = label
		if parent == null or not label.is_visible_in_tree() or not (str(label.name).begins_with("InfoDetail") or str(label.name).begins_with("GhostDetail")):
			continue
		var rect := label.get_global_rect()
		print("OPTIONS_LABEL capture=%s name=%s path=%s text=%s size=%s minimum=%s rect=%s parent=%s parent_rect=%s frame_rect=%s align=%s grow=%s clip=%s autowrap=%s beyond_content=%s beyond_root=%s" % [capture, frame.name, label.get_path(), label.text, label.size, label.get_combined_minimum_size(), rect, parent.get_path(), parent.get_global_rect(), frame.get_global_rect(), label.horizontal_alignment, label.grow_horizontal, label.clip_text, label.autowrap_mode, rect.end.x > content_rect.end.x, rect.end.x > options_rect.end.x])

func set_scale_slider(slider: Control, percent: float) -> void:
	var rect := slider.get_global_rect()
	var position := Vector2(lerpf(rect.position.x, rect.end.x, percent), rect.get_center().y)
	pointer_at(position, MOUSE_BUTTON_LEFT, true)
	await process_frame
	pointer_at(position, MOUSE_BUTTON_LEFT, false)
	await wait_frames(4)

func merchant_root(client: Node) -> Control:
	var ui := client.get_node_or_null("MerchantUI")
	return ui.find_child("MerchantFrame", true, false) as Control if ui != null else null

func check_merchant_placement(client: Node, bag: Control, config: String) -> bool:
	var frame := merchant_root(client)
	var scale := frame.get_global_transform().get_scale().x
	if absf(scale - 1.25) > 0.01 or frame.get_global_rect().position.distance_to(Vector2(16, 104) * scale) > 2:
		fail("Merchant default placement or effective UI scale incorrect: " + str(frame.get_global_rect()))
		return false
	var bag_rect := bag.get_global_rect()
	var start := frame.get_global_rect().position + Vector2(105, 12)
	var moved_target := Vector2(1020, 480)
	await drag_title(start, start + moved_target - frame.get_global_rect().position)
	var moved := frame.get_global_rect().position
	if moved.distance_to(moved_target) > 2 or bag.get_global_rect() != bag_rect:
		fail("Merchant title move shifted backpack or did not move root: " + str(moved))
		return false
	var layout := FileAccess.get_file_as_string(config.path_join("world-of-osso/ui_layout.ron"))
	if not layout.contains("MerchantFrame") or not layout.contains('"17"') or not other_character_unchanged(layout):
		fail("Merchant position not saved for selected character or another character changed: " + layout)
		return false
	if not await check_merchant_clamp(client, frame, bag, bag_rect):
		return false
	return true

func check_merchant_clamp(client: Node, frame: Control, bag: Control, original_bag: Rect2) -> bool:
	root.size = Vector2i(900, 600)
	await wait_frames(5)
	if not check_scaled_hosts(client, 5.0 / 6.0):
		return false
	var rect := frame.get_global_rect()
	var limit := Vector2(root.size) - rect.size
	var scale := frame.get_global_transform().get_scale().x
	var saved := Vector2(1020, 480) / 1.25 * scale
	var expected := Vector2(minf(saved.x, limit.x), minf(saved.y, limit.y))
	if saved.x <= limit.x or saved.y <= limit.y or rect.position.distance_to(expected) > 2:
		fail("Merchant saved position did not clamp at scaled viewport: " + str(rect))
		return false
	root.size = Vector2i(1920, 1080)
	await wait_frames(5)
	if frame.get_global_rect().position.distance_to(Vector2(1020, 480)) > 2 or bag.get_global_rect() != original_bag:
		fail("Merchant restored placement or backpack changed after resize")
		return false
	return true

func drag_title(start: Vector2, target: Vector2) -> void:
	pointer_at(start, MOUSE_BUTTON_LEFT, true)
	await process_frame
	var motion := InputEventMouseMotion.new()
	motion.position = target
	motion.relative = target - start
	root.push_input(motion, true)
	await process_frame
	pointer_at(target, MOUSE_BUTTON_LEFT, false)
	await process_frame

func check_merchant_reset(client: Node, vendor: Dictionary, config: String) -> bool:
	if not await reset_merchant_options(client, vendor, config):
		return false
	if not await open_vendor(client, vendor):
		return false
	var frame := merchant_root(client)
	if frame.get_global_rect().position.distance_to(Vector2(16, 104) * frame.get_global_transform().get_scale().x) > 2:
		fail("Reset merchant reopened at saved instead of default position")
		return false
	push_key(KEY_ESCAPE, true)
	await process_frame
	push_key(KEY_ESCAPE, false)
	await wait_frames(8)
	if client.merchant_state().open:
		fail("Reset merchant Escape did not close vendor")
		return false
	return true

func reset_merchant_options(client: Node, vendor: Dictionary, config: String) -> bool:
	if not await open_menu_after_vendor(client, vendor):
		return false
	await click_menu_action(client, "MenuBtnOptions")
	var menu := client.get_node_or_null("GameMenuUI")
	await click(menu.find_child("OptionsTabinterface", true, false) as Control)
	await click(menu.find_child("ActionButtonreset_window_positions", true, false) as Control)
	var layout := FileAccess.get_file_as_string(config.path_join("world-of-osso/ui_layout.ron"))
	var options := FileAccess.get_file_as_string(config.path_join("world-of-osso/options_settings.ron"))
	if layout.contains('"17"') or not other_character_unchanged(layout) or not options.contains("modal_offset:Some((80.0,-32.0))"):
		fail("Merchant reset changed other character or Options modal: " + layout + " / " + options)
		return false
	await click(menu.find_child("OptionsDoneButton", true, false) as Control)
	if client.get_node_or_null("GameMenuUI") != null:
		await click_menu_action(client, "MenuBtnResume")
	return true

func open_menu_after_vendor(client: Node, vendor: Dictionary) -> bool:
	if client.merchant_state().open or client.target_state().target != vendor.id:
		fail("Merchant must close before clearing selected vendor: " + str(client.target_state()))
		return false
	push_key(KEY_ESCAPE, true)
	await process_frame
	push_key(KEY_ESCAPE, false)
	await process_frame
	if client.target_state().target != null or client.get_node_or_null("GameMenuUI") != null:
		fail("First Escape did not clear selected vendor before menu")
		return false
	push_key(KEY_ESCAPE, true)
	await process_frame
	push_key(KEY_ESCAPE, false)
	return await wait_menu(client)

func other_character_unchanged(layout: String) -> bool:
	var keyed := RegEx.new()
	var expression := '"18"\\s*:\\s*\\{[^}]*"CharacterFrame"\\s*:\\s*\\(\\s*75\\.0\\s*,\\s*80\\.0\\s*\\)'
	if keyed.compile(expression) != OK:
		return false
	return keyed.search(layout) != null

func wait_world(client: Node) -> bool:
	var deadline := Time.get_ticks_msec() + WORLD_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen != "InWorld" or state.selected_character_name != NAME or state.unit_count != 3:
			continue
		var terrain: Dictionary = state.terrain
		if terrain.map == "azeroth" and terrain.pending_count == 0 and terrain.failures.is_empty() and not terrain.parsed_tiles.is_empty():
			return true
	fail("Timed out waiting for owned vendor world and terrain: " + str(client.account_state()))
	return false

func find_vendor(client: Node) -> Dictionary:
	var deadline := Time.get_ticks_msec() + 15000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var camera := root.get_viewport().get_camera_3d()
		var units := client.get_node_or_null("WorldUnits")
		if camera == null or units == null:
			continue
		for unit in units.get_children():
			if str(unit.name) != VENDOR:
				continue
			var area := unit.find_child("UnitPick", true, false) as Area3D
			if area == null:
				continue
			var center := (area.get_child(0) as Node3D).global_position
			if not camera.is_position_in_frustum(center):
				continue
			var point := camera.unproject_position(center)
			var id = area.get_meta("unit_server_id")
			if UnitPicker.pick(camera, point) == id:
				return {"id": id, "point": point}
	fail("Owned vendor not replicated, visible and ray-pickable")
	return {}

func open_vendor(client: Node, vendor: Dictionary) -> bool:
	pointer_at(vendor.point, MOUSE_BUTTON_RIGHT, true)
	await process_frame
	if client.target_state().target != vendor.id or client.target_state().auto_attack != null:
		fail("Vendor right-click targeted an attack instead of interaction: " + str(client.target_state()))
		return false
	pointer_at(vendor.point, MOUSE_BUTTON_RIGHT, false)
	var deadline := Time.get_ticks_msec() + 5000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.merchant_state()
		if state.open and state.vendor_name == VENDOR and state.items == ["Fixture Bread"]:
			var ui := client.get_node_or_null("MerchantUI")
			if ui != null and ui.find_child("MerchantFrame", true, false) != null:
				return true
	fail("Interaction/vendor/inventory did not open authored merchant: " + str(client.merchant_state()))
	return false

func pointer_at(point: Vector2, button: MouseButton, down: bool) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	root.push_input(motion, true)
	var event := InputEventMouseButton.new()
	event.position = point
	event.global_position = point
	event.button_index = button
	event.pressed = down
	root.push_input(event, true)
