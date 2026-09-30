extends RefCounted

# Original authored mouse slider and CameraOptionsFile default.
const SENSITIVITY_MIN := 0.001
const SENSITIVITY_MAX := 0.01
const SENSITIVITY_DEFAULT := 0.003
const YAW_TOLERANCE := 0.00001
# Existing account_scene_camera content-plane drag point; never guess a replacement.
const DRAG_POINT := Vector2(640, 300)
const DRAG_PIXELS := 4.0

var fixture
var options
var scene: Node
var ui: Node
var camera: Camera3D
var character: Node3D
var character_pose: Transform3D
var selected_name: String

func run(flow: SceneTree, client: Node) -> bool:
	fixture = flow
	options = load("res://tests/camera_options_probe.gd").new()
	options.fixture = flow
	scene = client.get_node_or_null("CharacterSelectScene")
	ui = client.get_node_or_null("CharacterSelectUI")
	var deadline: int = Time.get_ticks_msec() + fixture.WORLD_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await fixture.process_frame
		camera = client.get_node_or_null("CharacterSelectScene/Camera") as Camera3D
		character = client.get_node_or_null("CharacterSelectScene/SelectedCharacter") as Node3D
		if camera != null and character != null:
			break
	if camera == null or character == null:
		return reject("Character-select camera/selected character did not load")
	for frame in range(3):
		await fixture.process_frame
	character_pose = character.global_transform
	selected_name = client.account_state().selected_character_name
	if not expect_preserved(client):
		return false
	var baseline := camera_yaw()
	if not is_finite(baseline):
		return reject("Character-select baseline camera yaw is nonfinite")
	for sensitivity in [0.003, 0.006]:
		if not await set_sensitivity(client, sensitivity):
			return false
		if not expect_yaw(baseline, "Options changed orbit before drag"):
			return false
		if not await left_drag(DRAG_PIXELS):
			return false
		# scaled_orbit_delta.x = -dx * sensitivity. SelectOrbit.eye uses
		# (sin(yaw), ..., cos(yaw)); look_at_from_position points basis.z
		# away from focus, so atan2(z.x, z.z) has the same yaw sign.
		# Fresh startup orbit and these small deltas stay inside +/-PI/8.
		var expected: float = -DRAG_PIXELS * sensitivity
		if not expect_yaw(baseline + expected, "Left-drag sensitivity=%s delta=%s" % [sensitivity, expected]):
			return false
		if not expect_preserved(client):
			return false
		if not await left_drag(-DRAG_PIXELS):
			return false
		if not expect_yaw(baseline, "Equal opposite drag did not restore orbit") or not expect_preserved(client):
			return false
		print("CHARSELECT_CAMERA_PROBE sensitivity=", sensitivity, " expected_delta=", expected, " restored_yaw=", camera_yaw())
	# Restore only this authored slider, never category defaults.
	if not await set_sensitivity(client, SENSITIVITY_DEFAULT):
		return false
	if not expect_yaw(baseline, "Restoring slider changed orbit") or not expect_preserved(client):
		return false
	print("PASS: authored charselect mouseSensitivity saves numerically and scales physical left-drag camera yaw; orbit/default/roster restored")
	return true

func set_sensitivity(client: Node, sensitivity: float) -> bool:
	var tab := ui.find_child("CharSelectMenuTab", true, false) as Control
	if tab == null or not tab.is_visible_in_tree():
		return reject("Character-select MENU tab absent/hidden")
	await fixture.click(tab)
	if not await fixture.wait_menu(client):
		return false
	if not fixture.menu_authored(client):
		return reject("Character-select MENU did not open authored modal")
	for name in ["MenuBtnOptions", "OptionsTabcamera"]:
		if not await options.click_option(client, name):
			return false
	if not await options.click_slider(client, "Slidermouse_sensitivity", (sensitivity - SENSITIVITY_MIN) / (SENSITIVITY_MAX - SENSITIVITY_MIN)):
		return false
	if not options.expect_saved_number("mouseSensitivity", sensitivity, 0.0000001):
		return false
	if not await options.close_camera_options(client):
		return false
	return expect_preserved(client)

func camera_yaw() -> float:
	var back := camera.global_transform.basis.z
	return atan2(back.x, back.z)

func expect_yaw(expected: float, context: String) -> bool:
	var observed := camera_yaw()
	var error := wrapf(observed - expected, -PI, PI)
	if not is_finite(observed) or absf(error) > YAW_TOLERANCE:
		return reject("%s: expected_yaw=%s observed_yaw=%s error=%s tolerance=%s" % [context, expected, observed, error, YAW_TOLERANCE])
	return true

func expect_preserved(client: Node) -> bool:
	if not fixture.roster_unchanged(client, scene, ui) or client.get_node_or_null("CharacterSelectUI") != ui or client.account_state().selected_character_name != selected_name:
		return reject("Character-select sensitivity probe changed scene/UI/roster selection")
	if client.get_node_or_null("CharacterSelectScene/Camera") != camera or client.get_node_or_null("CharacterSelectScene/SelectedCharacter") != character or not character.global_transform.is_equal_approx(character_pose):
		return reject("Character-select sensitivity probe replaced camera/character or changed character pose")
	if client.get_node_or_null("GameMenuUI") != null:
		return reject("Character-select sensitivity probe left menu open")
	return true

func left_drag(pixels: float) -> bool:
	var motion := InputEventMouseMotion.new()
	motion.position = DRAG_POINT
	fixture.root.push_input(motion, true)
	await fixture.process_frame
	var hovered: Control = fixture.root.gui_get_hovered_control()
	if hovered != null:
		return reject("Unknown unobstructed content-plane point: account_scene_camera point %s overlaps UI %s; no alternate guessed" % [DRAG_POINT, hovered.get_path()])
	var button := InputEventMouseButton.new()
	button.position = DRAG_POINT
	button.global_position = DRAG_POINT
	button.button_index = MOUSE_BUTTON_LEFT
	button.button_mask = MOUSE_BUTTON_MASK_LEFT
	button.pressed = true
	fixture.root.push_input(button, true)
	for frame in range(2):
		await fixture.process_frame
	motion.position = DRAG_POINT + Vector2(pixels, 0)
	motion.global_position = motion.position
	motion.relative = Vector2(pixels, 0)
	motion.button_mask = MOUSE_BUTTON_MASK_LEFT
	fixture.root.push_input(motion, true)
	for frame in range(3):
		await fixture.process_frame
	button.position = motion.position
	button.global_position = button.position
	button.pressed = false
	button.button_mask = 0
	fixture.root.push_input(button, true)
	for frame in range(3):
		await fixture.process_frame
	return true

func reject(message: String) -> bool:
	fixture.fail(message)
	return false
