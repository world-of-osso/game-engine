extends "res://tests/display_options.gd"
## Options -> real world resources. Run saved Off and On in isolated diagnostics.
## Hidden effect booleans are preserved by authored live Graphics commits.

var world_environment: WorldEnvironment
var world_camera: Camera3D
var original_environment: Environment
var portrait_camera: Camera3D
var options_path: String

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	options_path = OS.get_environment("XDG_CONFIG_HOME").path_join("world-of-osso/options_settings.ron")
	if not options_path.contains("/data/diagnostics/"):
		fail("DOF/SSAO fixture requires owned diagnostic config")
		return
	var start_enabled := saved_option_value(options_path, "depthOfField") == "true"
	add_world()
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_for_startup_menu(client):
		return
	if not expect_resources(start_enabled, start_enabled, "saved startup"):
		return
	await click_menu_action(client, "MenuBtnOptions")
	if not await wait_for_graphics(client):
		return
	for settings in [Vector2i(1, 0), Vector2i(0, 1), Vector2i(1, 1), Vector2i(0, 0)]:
		if not await commit_effects(client, settings.x == 1, settings.y == 1):
			return
		if not expect_resources(settings.x == 1, settings.y == 1, "live commit"):
			return
	# Nodes may arrive after startup (world entry/re-entry).
	world_camera.free()
	world_environment.get_parent().free()
	add_world()
	await process_frame
	await process_frame
	if not expect_resources(false, false, "replacement Off world"):
		return
	if not await commit_effects(client, true, true):
		return
	world_camera.free()
	world_environment.get_parent().free()
	add_world()
	await process_frame
	await process_frame
	if not expect_resources(true, true, "replacement On world"):
		return
	print("PASS: saved and live DOF/SSAO resources, independent switches, Off identity, late world entry, portrait untouched")
	quit(0)

func add_world() -> void:
	original_environment = Environment.new()
	original_environment.ambient_light_energy = 0.7
	var lighting := Node3D.new()
	lighting.name = "WorldLighting"
	root.add_child(lighting)
	world_environment = WorldEnvironment.new()
	world_environment.name = "Environment"
	world_environment.environment = original_environment
	lighting.add_child(world_environment)
	world_camera = Camera3D.new()
	world_camera.name = "WorldCamera"
	root.add_child(world_camera)
	world_camera.current = true
	if portrait_camera == null:
		var portrait := SubViewport.new()
		portrait.own_world_3d = true
		root.add_child(portrait)
		portrait_camera = Camera3D.new()
		portrait_camera.name = "WorldCamera"
		portrait.add_child(portrait_camera)

func expect_resources(dof: bool, ssao: bool, stage: String) -> bool:
	var environment := world_environment.environment
	if environment.ssao_enabled != ssao or not is_equal_approx(environment.ambient_light_energy, 0.7):
		fail(stage + ": SSAO or unrelated ambient energy differs")
		return false
	if ssao and (environment.ssao_radius != 1.0 or environment.ssao_intensity != 2.0 or environment.ssao_power != 1.5 or environment.ssao_light_affect != 1.0 or environment.ssao_ao_channel_affect != 1.0):
		fail(stage + ": SSAO radius/intensity/power differs")
		return false
	if not ssao and environment != original_environment:
		fail(stage + ": Off replaced original environment")
		return false
	if dof:
		var attributes := world_camera.attributes as CameraAttributesPractical
		if attributes == null or not attributes.dof_blur_far_enabled or not attributes.dof_blur_near_enabled:
			fail(stage + ": DOF attributes missing")
			return false
		if attributes.dof_blur_far_distance != 15.0 or attributes.dof_blur_far_transition != 5.0 or attributes.dof_blur_near_distance != 15.0 or attributes.dof_blur_near_transition != 5.0 or attributes.dof_blur_amount != 0.1:
			fail(stage + ": DOF mapping differs")
			return false
	elif world_camera.attributes != null:
		fail(stage + ": Off changed null camera attributes")
		return false
	if portrait_camera.attributes != null:
		fail(stage + ": world DOF leaked into portrait viewport")
		return false
	return true

func commit_effects(client: Node, dof: bool, ssao: bool) -> bool:
	# Match the existing Options path: reread hidden fields, save snapshot, apply.
	# Not live file watching: writing alone must not change the renderer.
	var old_dof := world_camera.attributes != null
	var old_ssao := world_environment.environment.ssao_enabled
	var text := FileAccess.get_file_as_string(options_path)
	for pair in [["depthOfField", dof], ["ssaoEnabled", ssao]]:
		var pattern := RegEx.new()
		pattern.compile("(?m)(^\\s*" + pair[0] + ":\\s*)(true|false)")
		text = pattern.sub(text, "$1" + str(pair[1]).to_lower(), true)
	var file := FileAccess.open(options_path, FileAccess.WRITE)
	if file == null:
		fail("Cannot write owned Options: " + error_string(FileAccess.get_open_error()))
		return false
	file.store_string(text)
	file.close()
	await process_frame
	if (world_camera.attributes != null) != old_dof or world_environment.environment.ssao_enabled != old_ssao:
		fail("Effect file unexpectedly watched before Options commit")
		return false
	var cap := Engine.max_fps != 0
	await click_option(client, "ToggleSwitchframe_rate_limit_enabledLeftHit" if cap else "ToggleSwitchframe_rate_limit_enabledRightHit")
	return saved_option_value(options_path, "depthOfField") == str(dof).to_lower() and saved_option_value(options_path, "ssaoEnabled") == str(ssao).to_lower()
