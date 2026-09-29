extends "res://tests/world_spell_click_flow.gd"

const VENDOR := "Fixture Vendor"

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	var config := OS.get_environment("XDG_CONFIG_HOME")
	if not endpoint.begins_with("127.0.0.1:") or endpoint.ends_with(":0") or not config.contains("/data/native-reset-fixture-"):
		fail("Merchant-click fixture requires owned UDP endpoint and isolated options")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_screen(client, "Loading", 15000):
		return
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
	var tab := ui.find_child("MerchantFrameTab2", true, false) as Control
	if tab == null or not await assert_click(client, effects, completed, tab, "merchant tab"):
		return
	var close := ui.find_child("MerchantFrameCloseButton", true, false) as Control
	if close == null or not await assert_click(client, effects, completed, close, "merchant close"):
		return
	if client.merchant_state().open:
		fail("Merchant close action did not close vendor")
		return
	if not await open_vendor(client, vendor):
		return
	await wait_frames(8)
	if effects.is_playing() or completed[0] != 2:
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
	print("FIXTURE MERCHANT_CLICK_DONE")
	client.free()
	quit(0)

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
	event.button_index = button
	event.pressed = down
	root.push_input(event, true)
