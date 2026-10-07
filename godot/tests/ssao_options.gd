extends "res://tests/display_options.gd"
## Options -> real world resources. Run saved Off and On in isolated diagnostics.

var world_environment: WorldEnvironment
var world_camera: Camera3D
var original_environment: Environment
var original_attributes: CameraAttributesPractical
var portrait_environment: WorldEnvironment
var portrait_original: Environment
var options_path: String
var original_prepass: bool

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	options_path = OS.get_environment("XDG_CONFIG_HOME").path_join("world-of-osso/options_settings.ron")
	if not options_path.contains("/data/diagnostics/"):
		fail("SSAO fixture requires owned diagnostic config")
		return
	original_prepass = ProjectSettings.get_setting("rendering/driver/depth_prepass/enable")
	var start_enabled := saved_option_value(options_path, "ssaoEnabled") == "true"
	if original_prepass != start_enabled:
		fail("Saved SSAO must select depth prepass before renderer startup")
		return
	add_world()
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_for_startup_menu(client):
		return
	if not expect_resources(start_enabled, "saved startup"):
		return
	await click_menu_action(client, "MenuBtnOptions")
	if not await wait_for_graphics(client):
		return
	for enabled in [true, false, true, false]:
		if not await commit_effects(client, enabled) or not expect_resources(start_enabled, "restart-pending commit"):
			return
	# Nodes may arrive after startup (world entry/re-entry).
	for enabled in [false, true]:
		if not await commit_effects(client, enabled):
			return
		world_camera.free()
		world_environment.get_parent().free()
		add_world()
		await process_frame
		await process_frame
		if not expect_resources(start_enabled, "replacement world"):
			return
	if not await commit_effects(client, false) or not expect_resources(start_enabled, "final pending Off"):
		return
	client.free()
	var controller := root.get_node_or_null("NativeWorldEffects")
	if controller != null:
		controller.free()
	if ProjectSettings.get_setting("rendering/driver/depth_prepass/enable") != original_prepass:
		fail("Controller disposal changed depth prepass")
		return
	print("PASS: saved startup SSAO/prepass, restart-labeled UI persistence without live changes, replacement world, untouched camera/portrait")
	quit(0)

func add_world() -> void:
	original_attributes = CameraAttributesPractical.new()
	original_attributes.exposure_multiplier = 0.65
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
	world_camera.attributes = original_attributes
	root.add_child(world_camera)
	world_camera.current = true
	if portrait_environment == null:
		var portrait := SubViewport.new()
		portrait.own_world_3d = true
		root.add_child(portrait)
		var portrait_lighting := Node3D.new()
		portrait_lighting.name = "WorldLighting"
		portrait.add_child(portrait_lighting)
		portrait_original = Environment.new()
		portrait_environment = WorldEnvironment.new()
		portrait_environment.name = "Environment"
		portrait_environment.environment = portrait_original
		portrait_lighting.add_child(portrait_environment)

func expect_resources(ssao: bool, stage: String) -> bool:
	var environment := world_environment.environment
	if environment.ssao_enabled != ssao or not is_equal_approx(environment.ambient_light_energy, 0.7):
		fail(stage + ": SSAO or unrelated ambient energy differs")
		return false
	if ssao and (environment.ssao_radius != 1.0 or environment.ssao_intensity != 2.0 or environment.ssao_power != 1.5 or environment.ssao_light_affect != 1.0 or environment.ssao_ao_channel_affect != 1.0):
		fail(stage + ": SSAO mapping differs")
		return false
	if not ssao and environment != original_environment:
		fail(stage + ": Off replaced original environment")
		return false
	var prepass: bool = ProjectSettings.get_setting("rendering/driver/depth_prepass/enable")
	if prepass != original_prepass:
		fail(stage + ": required depth prepass changed")
		return false
	if world_camera.attributes != original_attributes:
		fail(stage + ": SSAO changed camera attributes")
		return false
	if portrait_environment.environment != portrait_original:
		fail(stage + ": SSAO leaked into portrait viewport")
		return false
	return true

func commit_effects(client: Node, ssao: bool) -> bool:
	var old_environment := world_environment.environment
	var label := option_control(client, "ToggleLabelssao_enabled") as Label
	if label == null or not label.text.contains("Requires Restart"):
		fail("SSAO Graphics row must disclose restart requirement")
		return false
	var current := saved_option_value(options_path, "ssaoEnabled") == "true"
	if current != ssao:
		await click_option(client, "ToggleSwitchssao_enabledRightHit" if ssao else "ToggleSwitchssao_enabledLeftHit")
	# Unrelated commits must also retain startup SSAO.
	var cap := Engine.max_fps != 0
	await click_option(client, "ToggleSwitchframe_rate_limit_enabledLeftHit" if cap else "ToggleSwitchframe_rate_limit_enabledRightHit")
	if saved_option_value(options_path, "ssaoEnabled") != str(ssao).to_lower():
		fail("Authored Graphics commit lost saved SSAO boolean")
		return false
	if world_environment.environment != old_environment:
		fail("Restart-pending Options changed live Environment identity")
		return false
	if FileAccess.get_file_as_string(options_path).contains("depthOfField"):
		fail("Removed option was serialized by live Options commit")
		return false
	return true
