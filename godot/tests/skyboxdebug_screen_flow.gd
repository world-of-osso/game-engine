extends "res://tests/debug_screen_flow_base.gd"

# `--screen skyboxdebug` with its default authored source (original
# `src/scenes/skybox_debug/mod.rs`): the orbit camera around the skybox M2. Needs no
# server. This flow checks the public CLI's `export-scene` snapshot; the screen's
# rendering is covered by skybox_debug_screen.gd.
# Run: native_debug_screen_fixture skyboxdebug

var scene: Node3D

func check_live() -> bool:
	scene = await wait_node("SkyboxDebug") as Node3D
	if scene == null:
		return false
	if client.account_state().reply_received:
		fail("Skybox debug screen contacted a server")
		return false
	await settle(300)
	return true

func sky_node() -> Node3D:
	for child in scene.get_children():
		if child is Node3D and child.has_meta("m2_source_path"):
			return child
	fail("SkyboxDebug has no authored sky M2")
	return null

# `export-scene` writes the original SkyboxDebugScene: the camera with its live FOV and
# the skybox object with its loader input.
func check_cli() -> bool:
	var exported := exported_scene()
	if exported.is_empty():
		return false
	if exported.label != "SkyboxDebugScene" or exported.props != "Scene" or child_labels(exported) != ["Camera", "Skybox"]:
		fail("Exported skybox debug scene %s %s %s" % [exported.label, exported.props, child_labels(exported)])
		return false
	var camera := scene.get_node("Camera") as Camera3D
	var exported_camera := scene_child(exported, "Camera")
	if absf(props_of(exported_camera, "Camera").fov - camera.fov) > 0.001 or not transform_matches(exported_camera, camera.transform):
		fail("Exported camera %s, live fov %.2f" % [exported_camera, camera.fov])
		return false
	var sky := sky_node()
	if sky == null:
		return false
	var skybox := scene_child(exported, "Skybox")
	var object := props_of(skybox, "Object")
	if object.kind != "Skybox" or object.model != sky.get_meta("m2_source_path") or not transform_matches(skybox, sky.transform):
		fail("Exported skybox %s, loader input %s" % [skybox, sky.get_meta("m2_source_path")])
		return false
	print("FIXTURE SKYBOXDEBUG_EXPORT %s" % object.model)
	return true
