extends "res://tests/world_menu_flow.gd"

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	var config := OS.get_environment("XDG_CONFIG_HOME")
	if not config.contains("/data/diagnostics/native-input-") or not FileAccess.file_exists(config.path_join("world-of-osso/options_settings.ron")):
		fail("Sound fixture requires isolated deterministic options")
		return
	if not endpoint.begins_with("127.0.0.1:") or endpoint.ends_with(":0"):
		fail("Sound fixture requires owned loopback endpoint")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_screen(client, "Loading", 15000):
		return
	print("FIXTURE SOUND_LOADING")
	if not await wait_world(client):
		return
	var state: Dictionary = client.account_state()
	var player := client.get_node_or_null("WorldUnits/" + NAME) as Node3D
	var sound := client.get_node_or_null("NativeSound")
	var music := sound.get_node_or_null("Music") as AudioStreamPlayer if sound != null else null
	var ambient := sound.get_node_or_null("Ambient") as AudioStreamPlayer if sound != null else null
	if player == null or player.position.distance_to(Vector3(-8949.0, 112.879913, 0.0)) > 0.1 or state.area_id != 9 or state.zone_id != 12:
		fail("Spawn/loaded MCNK area/root zone changed: " + str(state))
		return
	if not expect_music(music, "53492", 0.45):
		return
	if ambient == null or ambient.is_playing() or ambient.stream != null:
		fail("Area 9 -> zone 12 unexpectedly has catalogued ambient")
		return
	print("FIXTURE SOUND_WORLD area=9 zone=12 music=53492 ambient=none")
	push_key(KEY_ESCAPE, true)
	await process_frame
	push_key(KEY_ESCAPE, false)
	if not await wait_menu(client):
		return
	await click_menu_action(client, "MenuBtnOptions")
	await process_frame
	await click_option(client, "OptionsTabsound")
	await click_slider(client, "Slidermaster_volume", 0.25)
	if not await expect_volumes(music, ambient, 0.1125, 0.075, 0.025):
		return
	await click_option(client, "OptionsDefaultsButton")
	if not await expect_volumes(music, ambient, 0.45, 0.3):
		return
	await click_option(client, "ToggleSwitchmutedRightHit")
	if not await expect_volumes(music, ambient, 0.0, 0.0):
		return
	await click_option(client, "ToggleSwitchmutedLeftHit")
	if not await expect_volumes(music, ambient, 0.45, 0.3):
		return
	await click_slider(client, "Slidermaster_volume", 0.25)
	if not await expect_volumes(music, ambient, 0.1125, 0.075, 0.025):
		return
	await click_option(client, "ToggleSwitchmusic_enabledLeftHit")
	for frame in range(4):
		await process_frame
	if music.is_playing() or not is_equal_approx(music.volume_linear, 0.1125) or absf(ambient.volume_linear - 0.075) > 0.025:
		fail("Music toggle did not stop music while retaining ambient channel volume")
		return
	var saved := FileAccess.get_file_as_string(config.path_join("world-of-osso/options_settings.ron"))
	if not saved.contains("music_enabled: false") or not saved.contains("muted: false"):
		fail("Sound UI did not persist music toggle/mute in isolated options: " + saved)
		return
	print("FIXTURE SOUND_OPTIONS")
	await click_option(client, "OptionsDoneButton")
	if client.get_node_or_null("GameMenuUI") == null:
		push_key(KEY_ESCAPE, true)
		await process_frame
		push_key(KEY_ESCAPE, false)
		if not await wait_menu(client):
			return
	await click_menu_action(client, "MenuBtnExit")
	var deadline := Time.get_ticks_msec() + MENU_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
	fail("Exit button did not terminate sound fixture")

func expect_music(music: AudioStreamPlayer, track: String, volume: float) -> bool:
	if music == null or not music.is_playing() or music.stream == null or music.stream.resource_name != track or absf(music.volume_linear - volume) > 0.001:
		fail("Expected live world music %s at %.3f; got %s" % [track, volume, music])
		return false
	return true

func expect_volumes(music: AudioStreamPlayer, ambient: AudioStreamPlayer, music_volume: float, ambient_volume: float, tolerance: float = 0.001) -> bool:
	for frame in range(4):
		await process_frame
	if not music.is_playing() or music.stream == null or music.stream.resource_name != "53492" or ambient.is_playing() or ambient.stream != null or absf(music.volume_linear - music_volume) > tolerance or absf(ambient.volume_linear - ambient_volume) > tolerance:
		fail("Options did not reach real players: music=%s volume=%.3f ambient playing=%s volume=%.3f expected %.3f/%.3f" % [music.stream.resource_name if music.stream != null else "none", music.volume_linear, ambient.is_playing(), ambient.volume_linear, music_volume, ambient_volume])
		return false
	return true

func option_control(client: Node, name: String) -> Control:
	var menu := client.get_node_or_null("GameMenuUI")
	return menu.find_child(name, true, false) as Control if menu != null else null

func click_option(client: Node, name: String) -> void:
	var control := option_control(client, name)
	if control == null:
		fail("Missing authored sound control: " + name)
		return
	await click(control)

func click_slider(client: Node, name: String, fraction: float) -> void:
	var slider := option_control(client, name)
	if slider == null:
		fail("Missing authored sound slider: " + name)
		return
	var rect := slider.get_global_rect()
	var point := rect.position + Vector2(rect.size.x * fraction, rect.size.y * 0.5)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = MOUSE_BUTTON_LEFT
		event.position = point
		event.global_position = point
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
