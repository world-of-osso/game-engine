extends "res://tests/world_bags_cursor_flow.gd"

# Reuse observation/startup helpers, never execute accepted cursor/actions flows.
# Owned default settings and 1920x1080 viewport give physical == logical UI pixels.
const INITIAL := [[0, 0, 3], [1, 7, 2]]
const MOVED := [[1, 0, 3], [1, 7, 2]]
const DRAG_MIN := 4.0 # Original cursor_item/mod.rs oracle, not a layout policy.

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	var config := OS.get_environment("XDG_CONFIG_HOME")
	if not endpoint.begins_with("127.0.0.1:") or endpoint.ends_with(":0") or not config.contains("/data/diagnostics/native-input-"):
		fail("Bags-drag requires owned loopback peer and isolated defaults")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_screen(client, "Loading", 180000):
		return
	print("FIXTURE BAGS_DRAG_LOADING")
	if not await wait_world(client):
		return
	var account: Dictionary = client.account_state()
	if not account.reply_received or account.character_count != 3:
		fail("Bags-drag must originate from authenticated fixture roster")
		return
	print("FIXTURE BAGS_DRAG_READY")
	if not await wait_inventory(client, INITIAL) or not bags_closed(client):
		fail("Bags-drag must start with original bags fixture snapshot and closed containers")
		return
	for control_name in [BACKPACK, BAG_ONE]:
		var button := await wait_bag_button(client, control_name)
		if button == null:
			return
		await click(button)
	if not await wait_container(client, 0, true) or not await wait_container(client, 1, true):
		return
	if not check_container(client, 0, 16, 0, "3") or not check_container(client, 1, 8, 7, "2"):
		return
	if not await quiet_releases(client):
		return
	if not await foreign_chat_release(client):
		return
	for delivery in ["SEPARATED", "RAPID", "CLICK"]:
		if not await swap_case(client, delivery):
			return
	# Final peer snapshot: guid755001 x3 moved; guid755002 x2 untouched.
	# Public bag getter exposes id/count/location, not guid: no client guid proof.
	while true:
		await process_frame

func point_in_slot(client: Node, control_name: String) -> Vector2:
	var control := authored_control(client, control_name)
	if control == null or not control.is_visible_in_tree() or not control.get_global_rect().has_area():
		fail("Missing authored drag slot: " + control_name)
		return Vector2.INF
	return control.get_global_rect().get_center()

func inert_frame_point(client: Node) -> Vector2:
	# bag_toggle:2 is not a cursor target, and no authoritative bag2 exists.
	# Unlike the mouse-disabled container/title, this authored button blocks World.
	var control := authored_control(client, "CharacterBag1Slot")
	if control == null or not control.is_visible_in_tree() or control.mouse_filter == Control.MOUSE_FILTER_IGNORE:
		fail("Missing authored mouse-blocking, cursor-inert absent-bag button")
		return Vector2.INF
	return control.get_global_rect().get_center()

func move_pointer(point: Vector2, previous: Vector2, held: bool) -> void:
	cursor_pointer = point
	var event := InputEventMouseMotion.new()
	event.position = point
	event.global_position = point
	event.relative = point - previous
	event.button_mask = MOUSE_BUTTON_MASK_LEFT if held else 0
	root.push_input(event, true)

func left_edge(point: Vector2, pressed: bool) -> void:
	cursor_pointer = point
	var event := InputEventMouseButton.new()
	event.position = point
	event.global_position = point
	event.button_index = MOUSE_BUTTON_LEFT
	event.button_mask = MOUSE_BUTTON_MASK_LEFT if pressed else 0
	event.pressed = pressed
	root.push_input(event, true)

func quiet_drag_state(client: Node, expected: Array, held: bool) -> bool:
	var deadline := Time.get_ticks_msec() + CURSOR_QUIET_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var icon := authored_control(client, CURSOR)
		var shown := icon != null and icon.is_visible_in_tree()
		var popup := authored_control(client, "StaticPopup1")
		if shown != held or (popup != null and popup.is_visible_in_tree()) or not inventory_matches(client, expected) or not containers_remain_open(client):
			fail("Drag quiet state: cursor=%s expected=%s popup=%s inventory=%s containers=%s" % [shown, held, popup != null and popup.is_visible_in_tree(), inventory_matches(client, expected), containers_remain_open(client)])
			return false
		for bag in [0, 1]:
			for slot in range(16 if bag == 0 else 8):
				var count := 0
				for item in expected:
					if item[0] == bag and item[1] == slot:
						count = item[2]
				if not slot_render_matches(client, bag, slot, count, held and bag == 0 and slot == 0):
					fail("Drag quiet slot mismatch bag%s/slot%s count%s held%s" % [bag, slot, count, held])
					return false
	return true

func clear_local_pickup(client: Node) -> bool:
	if not await click_slot(client, SOURCE_SLOT) or not await wait_cursor(client, false):
		return false
	return await wait_slot_render(client, 0, 0, 3, false)

func quiet_releases(client: Node) -> bool:
	# Same-target travel >=4: source pickup remains local, release must be quiet.
	var source := point_in_slot(client, SOURCE_SLOT)
	var source_control := authored_control(client, SOURCE_SLOT)
	if source == Vector2.INF or source_control == null:
		return false
	var within := source + Vector2(source_control.get_global_rect().size.x * 0.3, 0)
	if source.distance_to(within) < DRAG_MIN or not source_control.get_global_rect().has_point(within):
		fail("Authored source has insufficient same-slot >=4 logical px travel")
		return false
	move_pointer(source, source, false)
	await process_frame
	left_edge(source, true)
	if not await wait_cursor(client, true):
		return false
	move_pointer(within, source, true)
	await process_frame
	left_edge(within, false)
	if not await wait_cursor(client, true) or not await quiet_drag_state(client, INITIAL, true) or not await clear_local_pickup(client):
		return false
	print("FIXTURE BAGS_DRAG_SAME_SOURCE")

	# Cursor was already held; this press on a cursor-inert button did not pick it up.
	# Release on a different actionable slot must not drop that held cursor.
	if not await click_slot(client, SOURCE_SLOT) or not await wait_cursor(client, true):
		return false
	var inert := inert_frame_point(client)
	var target := point_in_slot(client, SWAP_SLOT)
	if inert == Vector2.INF or target == Vector2.INF or inert.distance_to(target) < DRAG_MIN:
		fail("Authored inert-to-target drag path missing/too short")
		return false
	move_pointer(inert, cursor_pointer, false)
	await process_frame
	left_edge(inert, true)
	await process_frame
	move_pointer(target, inert, true)
	await process_frame
	left_edge(target, false)
	if not await wait_cursor(client, true) or not await quiet_drag_state(client, INITIAL, true) or not await clear_local_pickup(client):
		return false
	print("FIXTURE BAGS_DRAG_HELD_RELEASE")

	# Pickup this press, then release onto a mouse-blocking non-cursor button.
	source = point_in_slot(client, SOURCE_SLOT)
	inert = inert_frame_point(client)
	if source == Vector2.INF or inert == Vector2.INF or source.distance_to(inert) < DRAG_MIN:
		fail("Authored source-to-inert drag path missing/too short")
		return false
	move_pointer(source, cursor_pointer, false)
	await process_frame
	left_edge(source, true)
	if not await wait_cursor(client, true):
		return false
	move_pointer(inert, source, true)
	await process_frame
	left_edge(inert, false)
	if not await wait_cursor(client, true) or not await quiet_drag_state(client, INITIAL, true) or not await clear_local_pickup(client):
		return false
	print("FIXTURE BAGS_DRAG_FRAME_RELEASE")
	return true

func foreign_chat_point(client: Node) -> Vector2:
	var bags := client.get_node_or_null("BagsUI")
	var chat := client.get_node_or_null("ChatFrameUI")
	if bags == null or chat == null or bags == chat:
		fail("Foreign release requires distinct actual BagsUI and ChatFrameUI hosts")
		return Vector2.INF
	var tab := authored_control(chat, "ChatFrame1TabsTab0")
	if tab == null or not chat.is_ancestor_of(tab) or bags.is_ancestor_of(tab) or not tab.is_visible_in_tree() or tab.mouse_filter != Control.MOUSE_FILTER_STOP:
		fail("Foreign chat tab must be visible and mouse-blocking under ChatFrameUI, not BagsUI")
		return Vector2.INF
	var rect := tab.get_global_rect()
	var point := rect.get_center()
	if not rect.has_area() or not rect.has_point(point) or not root.get_visible_rect().has_point(point):
		fail("Foreign chat tab requires valid visible viewport geometry")
		return Vector2.INF
	# RegistryCanvas/full-screen layout parents and visual parts are IGNORE.
	# Check actual mouse hit controls, not every enclosing container rectangle.
	for node in bags.find_children("*", "Control", true, false):
		var control := node as Control
		if control.is_visible_in_tree() and control.mouse_filter != Control.MOUSE_FILTER_IGNORE and control.get_global_rect().has_point(point):
			fail("Foreign chat center overlaps own BagsUI hit control: " + str(control.get_path()))
			return Vector2.INF
	print("BAGS DRAG FOREIGN chat=", tab.get_path(), " rect=", rect, " point=", point, " outside BagsUI hit controls")
	return point

func foreign_chat_release(client: Node) -> bool:
	var source := point_in_slot(client, SOURCE_SLOT)
	var target := foreign_chat_point(client)
	if source == Vector2.INF or target == Vector2.INF or source.distance_to(target) < DRAG_MIN:
		fail("Authored source-to-foreign-chat drag path missing/too short")
		return false
	move_pointer(source, cursor_pointer, false)
	await process_frame
	left_edge(source, true)
	if not await wait_cursor(client, true) or not await wait_slot_render(client, 0, 0, 3, true):
		return false
	if not inventory_matches(client, INITIAL):
		fail("Foreign chat drag pickup mutated authoritative inventory")
		return false
	move_pointer(target, source, true)
	await process_frame
	left_edge(target, false)
	if not await wait_cursor(client, true) or not await quiet_drag_state(client, INITIAL, true) or not await clear_local_pickup(client):
		return false
	# Peer requires this fourth quiet phase before arming any swap request.
	print("FIXTURE BAGS_DRAG_FOREIGN_CHAT")
	return true

func swap_case(client: Node, delivery: String) -> bool:
	if not await wait_inventory(client, INITIAL) or not await wait_slot_render(client, 0, 0, 3, false) or not await wait_slot_render(client, 1, 0, 0, false) or not await wait_cursor(client, false):
		return false
	var source := point_in_slot(client, SOURCE_SLOT)
	var target := point_in_slot(client, SWAP_SLOT)
	if source == Vector2.INF or target == Vector2.INF or source.distance_to(target) < DRAG_MIN:
		fail("Authored different-target travel must be >=4 logical px")
		return false
	print("BAGS DRAG INPUT ", delivery, " source=", source, " target=", target, " logical_travel=", source.distance_to(target))
	print("FIXTURE BAGS_DRAG_%s_ARM" % delivery)
	# Arm/readiness frame is before input, not inserted between the rapid edges.
	await process_frame
	move_pointer(source, cursor_pointer, false)
	await process_frame
	if delivery == "RAPID":
		var frame_before := Engine.get_process_frames()
		left_edge(source, true)
		move_pointer(target, source, true)
		left_edge(target, false)
		if Engine.get_process_frames() != frame_before:
			fail("Rapid press/motion/release unexpectedly crossed a process frame")
			return false
		print("BAGS DRAG RAPID same-process-frame press/motion/release frame=", frame_before)
	elif delivery == "CLICK":
		# Ordinary pickup click; next press on target drops. Its displaced release
		# at the old source must neither pick up/drop again nor issue a second swap.
		left_edge(source, true)
		await process_frame
		left_edge(source, false)
		if not await wait_cursor(client, true):
			return false
		move_pointer(target, source, false)
		await process_frame
		left_edge(target, true)
		await process_frame
		move_pointer(source, target, true)
		left_edge(source, false)
	else:
		left_edge(source, true)
		if not await wait_cursor(client, true) or not await wait_slot_render(client, 0, 0, 3, true):
			return false
		if not inventory_matches(client, INITIAL):
			fail("Drag pickup mutated authoritative inventory")
			return false
		move_pointer(target, source, true)
		await process_frame
		left_edge(target, false)
	# Peer waits for both exact request and this explicit pre-delta proof marker.
	if not await wait_cursor(client, false) or not await quiet_drag_state(client, INITIAL, false):
		return false
	print("FIXTURE BAGS_DRAG_%s_COMMIT" % delivery)
	if not await wait_inventory(client, MOVED) or not await wait_slot_render(client, 0, 0, 0, false) or not await wait_slot_render(client, 1, 0, 3, false):
		return false
	if not await quiet_drag_state(client, MOVED, false):
		return false
	print("FIXTURE BAGS_DRAG_%s_DONE" % delivery)
	return true
