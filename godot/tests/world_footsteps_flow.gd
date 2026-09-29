extends "res://tests/world_menu_flow.gd"

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	if not endpoint.begins_with("127.0.0.1:") or endpoint.ends_with(":0"):
		fail("Footsteps require owned loopback UDP server")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_screen(client, "Loading", 15000):
		return
	print("FIXTURE FOOTSTEPS_LOADING")
	if not await wait_world(client):
		return
	var player := client.get_node_or_null("WorldUnits/" + NAME) as Node3D
	var locomotion = load("res://tests/player_locomotion_probe.gd").new()
	var deadline := Time.get_ticks_msec() + WORLD_WAIT_MS
	while player != null and locomotion.bind(player) != "":
		if Time.get_ticks_msec() > deadline:
			fail("Authored player animation did not load")
			return
		await process_frame
	if player == null:
		fail("Selected local player absent after world readiness")
		return
	var probe = load("res://tests/native_footstep_probe.gd").new()
	var error: String = await probe.check(self, client, player, locomotion)
	if error != "":
		fail(error)
		return
	var sound := client.get_node("NativeSound")
	var footsteps := sound.get_node("Footsteps")
	if not await open_sound_options(client):
		return
	if not await click_sound_option(client, "ToggleSwitchmusic_enabledLeftHit") or not await close_sound_options(client):
		return
	var music := sound.get_node("Music") as AudioStreamPlayer
	if music.is_playing():
		fail("Music remained enabled after disabling music for footstep independence check")
		return
	error = await probe.check_run(self, player, locomotion, footsteps, 0.8, true)
	if error != "":
		fail("Music-disabled Run: " + error)
		return
	if not await open_sound_options(client):
		return
	if not await click_sound_option(client, "ToggleSwitchmutedRightHit") or not await close_sound_options(client):
		return
	error = await probe.check_run(self, player, locomotion, footsteps, 0.0, false)
	if error != "":
		fail(error)
		return
	if not await open_sound_options(client):
		return
	if not await click_sound_option(client, "ToggleSwitchmutedLeftHit"):
		return
	if not await set_master_volume(client, 0.25) or not await close_sound_options(client):
		return
	error = await probe.check_run(self, player, locomotion, footsteps, 0.2, true)
	if error != "":
		fail("Live master/effects gain: " + error)
		return
	print("FIXTURE FOOTSTEPS_OPTIONS")
	if not await remove_playing_player(client, footsteps):
		return
	print("FIXTURE FOOTSTEPS_DONE")
	client.free()
	quit(0)

func open_sound_options(client: Node) -> bool:
	push_key(KEY_ESCAPE, true)
	await process_frame
	push_key(KEY_ESCAPE, false)
	if not await wait_menu(client):
		return false
	await click_menu_action(client, "MenuBtnOptions")
	return await click_sound_option(client, "OptionsTabsound")

func close_sound_options(client: Node) -> bool:
	if not await click_sound_option(client, "OptionsDoneButton"):
		return false
	if client.get_node_or_null("GameMenuUI") != null:
		await click_menu_action(client, "MenuBtnResume")
	return await wait_menu_closed(client, null)

func click_sound_option(client: Node, name: String) -> bool:
	var menu := client.get_node_or_null("GameMenuUI")
	var control := menu.find_child(name, true, false) as Control if menu != null else null
	if control == null:
		fail("Missing authored sound option: " + name)
		return false
	await click(control)
	return true

func set_master_volume(client: Node, value: float) -> bool:
	var menu := client.get_node_or_null("GameMenuUI")
	var slider := menu.find_child("Slidermaster_volume", true, false) as Control if menu != null else null
	if slider == null:
		fail("Missing authored master-volume slider")
		return false
	var rect := slider.get_global_rect()
	var point := rect.position + Vector2(rect.size.x * value, rect.size.y * 0.5)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = MOUSE_BUTTON_LEFT
		event.position = point
		event.global_position = point
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	return true

func remove_playing_player(client: Node, footsteps: Node) -> bool:
	push_key(KEY_W, true)
	var deadline := Time.get_ticks_msec() + 3000
	var playing := false
	while Time.get_ticks_msec() < deadline:
		await process_frame
		for child in footsteps.get_children():
			playing = playing or (child is AudioStreamPlayer3D and child.is_playing())
		if playing:
			break
	push_key(KEY_W, false)
	if not playing:
		fail("Could not trigger active footstep before local-player removal")
		return false
	print("FIXTURE FOOTSTEPS_REMOVE")
	deadline = Time.get_ticks_msec() + 3000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if client.get_node_or_null("WorldUnits/" + NAME) == null:
			if footsteps.get_child_count() != 0:
				fail("Player removal retained 3D footsteps or tracker")
				return false
			return true
	fail("Owned UDP player removal did not reach GameClient")
	return false
