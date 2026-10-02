extends "res://tests/debug_screen_flow_base.gd"

# `--screen m2debug` (original `src/scenes/m2_debug/mod.rs`): the wolf reference model
# 126487 on a grass ground plane under a directional light, framed by an orbit camera
# focused 1 yd above the origin at 6 yd. Needs no server.
# Run: native_debug_screen_fixture m2debug
const MODEL := 126487
# creature/wolf/wolf.m2's preferred CreatureDisplayInfo skins.
const SKINS := [126494, 126495]
const FOCUS := Vector3(0.0, 1.0, 0.0)
const DISTANCE := 6.0
const FOV := 45.0
const MIN_MODEL_PIXELS := 2000

var scene: Node

func check_live() -> bool:
	scene = await wait_node("M2Debug")
	if scene == null:
		return false
	var login := client.get_node_or_null("LoginUI") as CanvasLayer
	if login != null and login.visible:
		fail("Login UI is still visible over the M2 debug scene")
		return false
	if client.account_state().reply_received:
		fail("M2 debug screen contacted a server")
		return false
	var state: Dictionary = scene.debug_state()
	if state.model_fdid != MODEL or state.skin_fdids.slice(0, 2) != PackedInt32Array(SKINS) or not state.missing_textures.is_empty():
		fail("Reference model state: %s" % state)
		return false
	var model := scene.get_node_or_null("M2DebugReferenceModel") as Node3D
	if model == null or not model.has_meta("m2_bounds"):
		fail("Reference model node missing")
		return false
	if absf(model.rotation.y + PI / 2.0) > 0.001:
		fail("Reference model yaw %.4f, expected -PI/2" % model.rotation.y)
		return false
	var camera := scene.get_node("Camera") as Camera3D
	if not camera.current or absf(camera.fov - FOV) > 0.01:
		fail("Camera current=%s fov=%.2f" % [camera.current, camera.fov])
		return false
	await settle(500)
	if absf(camera.global_position.distance_to(FOCUS) - DISTANCE) > 0.01:
		fail("Camera %.3f yd from focus" % camera.global_position.distance_to(FOCUS))
		return false
	if not await check_animation(model):
		return false
	return await check_orbit(camera)

# The model plays its first sequence: bone poses change over time.
func check_animation(model: Node3D) -> bool:
	var skeleton := model.get_node_or_null("Skeleton3D") as Skeleton3D
	if skeleton == null or skeleton.get_bone_count() == 0:
		fail("Reference model has no skeleton")
		return false
	var before := []
	for bone in skeleton.get_bone_count():
		before.append(skeleton.get_bone_pose(bone))
	await settle(400)
	for bone in skeleton.get_bone_count():
		if not skeleton.get_bone_pose(bone).is_equal_approx(before[bone]):
			return true
	fail("Reference model bones did not animate")
	return false

# Left drag orbits about the focus; the wheel zooms toward it.
func check_orbit(camera: Camera3D) -> bool:
	var start := camera.global_position
	drag(MOUSE_BUTTON_LEFT, Vector2(120, 0))
	await settle(200)
	if camera.global_position.distance_to(start) < 0.5 or absf(camera.global_position.distance_to(FOCUS) - DISTANCE) > 0.01:
		fail("Drag moved the camera %s -> %s" % [start, camera.global_position])
		return false
	for notch in 3:
		mouse_button(MOUSE_BUTTON_WHEEL_UP, true)
		mouse_button(MOUSE_BUTTON_WHEEL_UP, false)
	await settle(600)
	var zoomed := camera.global_position.distance_to(FOCUS)
	if zoomed > DISTANCE - 1.0:
		fail("Wheel zoom left the camera %.3f yd away" % zoomed)
		return false
	for notch in 3:
		mouse_button(MOUSE_BUTTON_WHEEL_DOWN, true)
		mouse_button(MOUSE_BUTTON_WHEEL_DOWN, false)
	drag(MOUSE_BUTTON_LEFT, Vector2(-120, 0))
	await settle(1000)
	if camera.global_position.distance_to(start) > 0.05:
		fail("Camera did not return to its framing: %s vs %s" % [camera.global_position, start])
		return false
	return true

func check_cli() -> bool:
	if not check_export():
		return false
	var semantic := tree_reply("scene")
	for expected in ["Camera \"Camera\" fov=45", "current=true", "Light \"Light\" DirectionalLight3D", "Object \"Ground\" MeshInstance3D", "Model \"M2DebugReferenceModel\" @ (0.0, 0.0, 0.0)"]:
		if not semantic.contains(expected):
			fail("CLI dump-scene lacks %s:\n%s" % [expected, semantic])
			return false
	if not tree_reply("tree").contains("M2Debug ("):
		fail("CLI dump-tree lacks the M2Debug node")
		return false
	# The CLI frame shows the wolf: hiding it changes that many pixels.
	var shot := screenshot()
	if shot == null:
		return false
	if shot.get_size() != SIZE:
		fail("CLI screenshot is %s, not the viewport's %s" % [shot.get_size(), SIZE])
		return false
	var model := scene.get_node("M2DebugReferenceModel") as Node3D
	model.visible = false
	var hidden := await capture()
	model.visible = true
	var shown := await capture()
	save(hidden, "model-hidden.png")
	# Lossy WebP noise alone also differs from the hidden frame; count only pixels the
	# live frame shows the wolf at too.
	var live_pixels := changed_pixels(shown, hidden)
	var cli_pixels := shared_changed_pixels(shot, shown, hidden)
	print("FIXTURE M2DEBUG_MODEL_PIXELS cli=%d live=%d" % [cli_pixels, live_pixels])
	if live_pixels < MIN_MODEL_PIXELS or cli_pixels < live_pixels * 0.8:
		fail("CLI screenshot shows %d of the live frame's %d model pixels" % [cli_pixels, live_pixels])
		return false
	return true

# `export-scene` writes the original M2DebugScene (src/scenes/m2_debug/mod.rs): Camera,
# Light, Ground and ReferenceModel, with the live FOV, light energy and loader input.
func check_export() -> bool:
	var exported := exported_scene()
	if exported.is_empty():
		return false
	if exported.label != "M2DebugScene" or exported.props != "Scene" or exported.transform != null:
		fail("Exported root %s %s %s" % [exported.label, exported.props, exported.transform])
		return false
	if child_labels(exported) != ["Camera", "Light", "Ground", "ReferenceModel"]:
		fail("Exported M2DebugScene children: %s" % [child_labels(exported)])
		return false
	var camera := scene.get_node("Camera") as Camera3D
	var light := scene.get_node("Light") as DirectionalLight3D
	var model := scene.get_node("M2DebugReferenceModel") as Node3D
	var exported_camera := scene_child(exported, "Camera")
	if absf(props_of(exported_camera, "Camera").fov - FOV) > 0.001 or not transform_matches(exported_camera, camera.transform):
		fail("Exported camera %s" % exported_camera)
		return false
	var light_props := props_of(scene_child(exported, "Light"), "Light")
	if light_props.kind != "DirectionalLight3D" or absf(light_props.intensity - light.light_energy) > 0.0001:
		fail("Exported light %s, live energy %.3f" % [light_props, light.light_energy])
		return false
	var ground := scene_child(exported, "Ground")
	if ground.props != "Ground" or not transform_matches(ground, (scene.get_node("Ground") as Node3D).transform):
		fail("Exported ground %s" % ground)
		return false
	var reference := scene_child(exported, "ReferenceModel")
	var object := props_of(reference, "Object")
	if object.kind != "reference-model" or object.model != model.get_meta("m2_source_path") or not str(object.model).ends_with("/%d.m2" % MODEL):
		fail("Exported reference model %s, loader input %s" % [object, model.get_meta("m2_source_path")])
		return false
	if not transform_matches(reference, model.transform) or not reference.children.is_empty():
		fail("Exported reference model transform/children %s" % reference)
		return false
	print("FIXTURE M2DEBUG_EXPORT %s" % object.model)
	return true
