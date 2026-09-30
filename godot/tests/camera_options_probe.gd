extends RefCounted

const LOOK_POINT := Vector2(640, 360)
const VERTICAL_MOTION := Vector2(0, 4)

var fixture

func run(flow: SceneTree, client: Node) -> bool:
	fixture = flow
	if client.get_node_or_null("GameMenuUI") != null or client.target_state().target != null:
		return reject("Camera probe requires menu-closed/target-none")
	for inverted in [false, true]:
		if not await set_invert_y(client, inverted):
			return false
		if not await expect_vertical_look(client, inverted):
			return false
	if not await set_invert_y(client, false):
		return false
	if client.get_node_or_null("GameMenuUI") != null or not client.account_state().gameplay_input_allowed:
		return reject("Camera probe did not return to menu-closed gameplay")
	print("PASS: authored invert-Y Off/On persists and reverses gameplay pitch sign; default Off restored, input released")
	return true

func set_invert_y(client: Node, inverted: bool) -> bool:
	# A real RMB press can select the unit under the pointer. Original Escape
	# clears that selection before it can open the menu.
	if client.target_state().target != null:
		await tap_escape()
		if client.target_state().target != null or client.get_node_or_null("GameMenuUI") != null:
			return reject("Escape did not clear mouse-look selection before menu")
	await tap_escape()
	if not await fixture.wait_menu(client) or not fixture.menu_authored(client):
		return false
	for name in ["MenuBtnOptions", "OptionsTabcamera"]:
		if not await click_option(client, name):
			return false
	var hit := "ToggleSwitchinvert_yRightHit" if inverted else "ToggleSwitchinvert_yLeftHit"
	var menu := client.get_node("GameMenuUI")
	# Authored toggle_widget emits a hitbox only for the inactive segment.
	# If already selected (initial Off), round-trip through the opposite
	# segment so this case still exercises an authored edit and persistence.
	if menu.find_child(hit, true, false) == null:
		var opposite := "ToggleSwitchinvert_yLeftHit" if inverted else "ToggleSwitchinvert_yRightHit"
		if not await click_option(client, opposite):
			return false
	if not await click_option(client, hit):
		return false
	var config := OS.get_environment("XDG_CONFIG_HOME")
	if config.is_empty():
		return reject("Camera probe requires fixture-owned XDG_CONFIG_HOME")
	var path := config.path_join("world-of-osso/options_settings.ron")
	if not FileAccess.file_exists(path):
		return reject("Authored camera edit did not create canonical settings file: " + path)
	var saved := FileAccess.get_file_as_string(path).replace(" ", "").replace("\t", "").replace("\n", "").replace("\r", "")
	var expected := "invert_y:true" if inverted else "invert_y:false"
	if not saved.contains(expected):
		return reject("Authored camera edit did not persist %s before closing Options" % expected)
	if not await click_option(client, "OptionsDoneButton"):
		return false
	if client.get_node_or_null("GameMenuUI") != null:
		if not await click_option(client, "MenuBtnResume"):
			return false
	return await fixture.wait_menu_closed(client, null)

func expect_vertical_look(client: Node, inverted: bool) -> bool:
	var state: Dictionary = client.account_state()
	if not state.gameplay_input_allowed:
		return reject("Resumed camera probe has gameplay input blocked")
	var before := float(state.camera_pitch)
	# Fresh fixture starts at -0.3 radians. Small motion keeps both cases far
	# from the original +/-88-degree clamp; never reposition the camera.
	if not is_finite(before) or absf(before) >= 1.0:
		return reject("Camera probe pitch is not safely inside clamp: " + str(before))
	push_right(true)
	await fixture.process_frame
	push_vertical_motion()
	for frame in range(3):
		await fixture.process_frame
	push_right(false)
	for frame in range(3):
		await fixture.process_frame
	var after := float(client.account_state().camera_pitch)
	var delta := after - before
	# Original apply_camera_input contract: positive relative Y subtracts
	# pitch with invert false, adds with true. No native-derived golden.
	if not is_finite(after) or absf(after) >= 1.0 or (delta <= 0.0 if inverted else delta >= 0.0):
		return reject("invert_y=%s: positive relative Y expected %s pitch delta, observed before=%s after=%s delta=%s" % [inverted, "positive" if inverted else "negative", before, after, delta])
	# Same motion without RMB must not rotate: exercises actual release,
	# rather than trusting that the synthetic release was consumed.
	push_vertical_motion()
	for frame in range(3):
		await fixture.process_frame
	var released := float(client.account_state().camera_pitch)
	if not is_finite(released) or absf(released - after) > 0.000001:
		return reject("Released RMB still changes gameplay pitch: %s -> %s" % [after, released])
	print("CAMERA_INVERT_PROBE invert_y=", inverted, " before=", before, " after=", after, " delta=", delta)
	return true

func push_right(pressed: bool) -> void:
	var event := InputEventMouseButton.new()
	event.button_index = MOUSE_BUTTON_RIGHT
	event.position = LOOK_POINT
	event.pressed = pressed
	fixture.root.push_input(event, true)

func push_vertical_motion() -> void:
	var event := InputEventMouseMotion.new()
	event.position = LOOK_POINT + VERTICAL_MOTION
	event.relative = VERTICAL_MOTION
	fixture.root.push_input(event, true)

func click_option(client: Node, name: String) -> bool:
	var menu := client.get_node_or_null("GameMenuUI")
	var control := menu.find_child(name, true, false) as Control if menu != null else null
	if control == null or not control.is_visible_in_tree():
		return reject("Authored camera Options control absent/hidden: " + name)
	await fixture.click(control)
	return true

func tap_escape() -> void:
	fixture.push_key(KEY_ESCAPE, true)
	await fixture.process_frame
	fixture.push_key(KEY_ESCAPE, false)
	for frame in range(3):
		await fixture.process_frame

func reject(message: String) -> bool:
	fixture.fail(message)
	return false
