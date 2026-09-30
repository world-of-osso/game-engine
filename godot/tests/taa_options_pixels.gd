extends "res://tests/anti_alias_options_pixels.gd"
## Actual saved-Taa consumer regression, not standalone kernel acceptance.
## Perspective opaque edges, exact higher UI, and unrelated authored commit.


func run_test() -> void:
	root.size = SIZE
	aa_mode = "Taa"
	var config := OS.get_environment("XDG_CONFIG_HOME")
	var directory := OS.get_environment("GODOT_TEST_CAPTURE_DIR")
	if not config.contains("/data/diagnostics/") or not directory.contains("/data/diagnostics/"):
		fail("TAA fixture requires owned diagnostic config/captures")
		return
	var path := config.path_join("world-of-osso/options_settings.ron")
	if not expect_saved_aa(path, false, "saved Taa input"):
		return
	if DisplayServer.get_name() == "headless" or RenderingServer.get_rendering_device() == null:
		fail("TAA fixture requires rendered Vulkan")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_for_startup_menu(client):
		return
	if not expect_renderer_aa("saved Taa startup"):
		return
	add_aa_scene()
	var camera := root.get_node("AntiAliasFixtureCamera") as Camera3D
	# Same two-world-unit vertical span at distance three as geometric fixture.
	camera.set_perspective(rad_to_deg(2.0 * atan(1.0 / 3.0)), 0.1, 100.0)
	add_aa_ui()
	for frame in range(32):
		await process_frame
	var startup := await capture_options_pixels(client, directory, "taa-startup.png")
	if not expect_aa_pixels(startup, "saved Taa startup"):
		return
	if camera.projection != Camera3D.PROJECTION_PERSPECTIVE:
		fail("Temporal jitter leaked projection mode outside drawing")
		return
	await click_menu_action(client, "MenuBtnOptions")
	if not await wait_for_graphics(client):
		return
	await click_option(client, "ToggleSwitchframe_rate_limit_enabledRightHit")
	if Engine.max_fps != DEFAULT_FPS or not expect_saved_aa(path, true, "unrelated commit"):
		fail("TAA unrelated Graphics commit failed persistence/frame cap")
		return
	for frame in range(16):
		await process_frame
	var committed := await capture_options_pixels(client, directory, "taa-committed.png")
	if (
		not expect_renderer_aa("unrelated commit")
		or not expect_aa_pixels(committed, "unrelated commit")
	):
		return
	if not await run_follow_up(client, path, directory):
		return
	print(
		"PASS: saved Taa produces temporal edge coverage and preserves exact UI/unrelated options"
	)
	quit(0)


func run_follow_up(_client: Node, _path: String, _directory: String) -> bool:
	return true
