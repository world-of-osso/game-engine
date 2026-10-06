extends SceneTree

# Private-server integration regression. Pending distant scenery must survive the
# transition to InWorld, and subsequently drain without another loading screen.
var client: Node

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if server != "127.0.0.1:5280":
		push_error("Requires owned Stormwind server on UDP 5280")
		quit(1)
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var deadline := Time.get_ticks_msec() + 180000
	while client.account_state().get("assets_starting", true) and Time.get_ticks_msec() < deadline:
		await process_frame
	var error: String = client.connect_account(server, "fb_swload", "fbtest", false)
	if not error.is_empty():
		push_error(error)
		quit(1)
		return
	deadline = Time.get_ticks_msec() + 90000
	while client.account_state().screen != "CharacterSelect" and Time.get_ticks_msec() < deadline:
		await process_frame
	var ui := client.get_node_or_null("CharacterSelectUI")
	if ui == null:
		push_error("Character select unavailable")
		quit(1)
		return
	await click_control(ui.find_child("CharCard_0", true, false))
	await click_control(ui.find_child("EnterWorld", true, false))
	var started := Time.get_ticks_msec()
	deadline = started + 480000
	while client.account_state().screen != "InWorld" and Time.get_ticks_msec() < deadline:
		await process_frame
	var state: Dictionary = client.account_state()
	if state.screen != "InWorld" or state.area_id != 1519:
		push_error("Stormwind entry did not complete: " + str(state))
		quit(1)
		return
	var objects: Dictionary = state.world_objects
	if objects.get("nearby_done", -1) != objects.get("nearby_total", -2) or objects.get("nearby_collision_pending", -1) != 0:
		push_error("Entered before nearby objects/collision ready: " + str(objects))
		quit(1)
		return
	if objects.pending <= 0:
		push_error("Gate waited for distant scenery instead of streaming it")
		quit(1)
		return
	print("SWLOAD TEST ENTRY ms=", Time.get_ticks_msec() - started, " objects=", objects)
	deadline = Time.get_ticks_msec() + 1800000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		state = client.account_state()
		if state.screen != "InWorld":
			push_error("Distant streaming re-entered loading")
			quit(1)
			return
		if state.world_objects.pending == 0 and state.terrain.pending_count == 0:
			print("SWLOAD TEST PASS all objects drained ms=", Time.get_ticks_msec() - started)
			quit(0)
			return
	push_error("Distant scenery did not drain: " + str(state.world_objects))
	quit(1)

func click_control(control: Control) -> void:
	if control == null:
		return
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = control.get_global_rect().get_center()
		event.global_position = event.position
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = pressed
		Input.parse_input_event(event)
		await process_frame
