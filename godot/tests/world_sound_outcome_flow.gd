extends "res://tests/world_sound_flow.gd"

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	if not endpoint.begins_with("127.0.0.1:") or endpoint.ends_with(":0"):
		fail("Outcome sound fixture requires owned loopback UDP")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_screen(client, "Loading", 15000):
		return
	print("FIXTURE OUTCOME_LOADING")
	if not await wait_world(client):
		return
	var sound := client.get_node_or_null("NativeSound")
	var outcomes := sound.get_node_or_null("OutcomeSpells") if sound != null else null
	if outcomes == null:
		fail("Native original CombatEvent spatial outcome channel missing")
		return
	var music := sound.get_node_or_null("Music") as AudioStreamPlayer
	if music == null or music.is_playing():
		fail("Music-disabled fixture unexpectedly started music")
		return
	var observed: Array[Node] = []
	outcomes.child_entered_tree.connect(func(node: Node): observed.append(node))
	print("FIXTURE OUTCOME_READY")
	var deadline := Time.get_ticks_msec() + 10000
	while observed.size() < 65 and Time.get_ticks_msec() < deadline:
		await process_frame
	if observed.size() != 65:
		fail("Expected 65 original outcome players, got %d" % observed.size())
		return
	var sizes := [10584, 15876, 9702, 13230]
	var local := client.get_node_or_null("WorldUnits/" + NAME) as Node3D
	var remote := client.get_node_or_null("WorldUnits/Remote Fixture") as Node3D
	if local == null or remote == null:
		fail("Replicated emitter units missing")
		return
	for index in range(observed.size()):
		var player := observed[index] as AudioStreamPlayer3D
		var expected := local if index % 4 == 3 else remote
		var gain: float = [0.8, 0.68, 0.44, 0.76][index % 4]
		if player == null or player.stream == null or not player.stream is AudioStreamWAV or player.stream.data.size() != sizes[index % 4] or player.global_position.distance_to(expected.global_position) > 0.1 or absf(player.volume_linear - gain) > 0.001:
			fail("Outcome %d has wrong PCM, source or gain" % index)
			return
	for frame in range(3):
		await process_frame
	if observed.size() != 65:
		fail("Outcome batch replayed on second frame: %d" % observed.size())
		return
	push_key(KEY_ESCAPE, true)
	await process_frame
	push_key(KEY_ESCAPE, false)
	if not await wait_menu(client):
		return
	await click_menu_action(client, "MenuBtnOptions")
	await process_frame
	await click_option(client, "OptionsTabsound")
	await click_slider(client, "Slidermaster_volume", 0.25)
	print("FIXTURE OUTCOME_BATCH")
	if not await wait_outcomes(observed, 66):
		return
	var scaled := observed[65] as AudioStreamPlayer3D
	if scaled == null or absf(scaled.volume_linear - 0.17) > 0.001:
		fail("Changed master did not scale heal to 0.25 * 0.8 * 0.85")
		return
	await click_option(client, "ToggleSwitchmutedRightHit")
	print("FIXTURE OUTCOME_MUTED")
	if not await wait_outcomes(observed, 68):
		return
	var removed_player: WeakRef = weakref(observed[66])
	for index in [66, 67]:
		var muted_player := observed[index] as AudioStreamPlayer3D
		if muted_player == null or muted_player.volume_linear != 0.0:
			fail("Muting did not silence original outcome %d" % index)
			return
	print("FIXTURE OUTCOME_REMOVE")
	deadline = Time.get_ticks_msec() + 5000
	while client.get_node_or_null("WorldUnits/Remote Fixture") != null and Time.get_ticks_msec() < deadline:
		await process_frame
	if client.get_node_or_null("WorldUnits/Remote Fixture") != null or removed_player.get_ref() != null:
		fail("Replicated remote removal left active emitter")
		return
	var local_active := observed[67] as AudioStreamPlayer3D
	if not is_instance_valid(local_active) or not local_active.is_playing():
		fail("Reset fixture has no active local emitter to release")
		return
	print("FIXTURE OUTCOME_RESET")
	deadline = Time.get_ticks_msec() + 5000
	while client.account_state().screen == "InWorld" and Time.get_ticks_msec() < deadline:
		await process_frame
	if client.account_state().screen == "InWorld" or outcomes.get_child_count() != 0:
		fail("Forced disconnect failed to clear outcome emitters")
		return
	var reference: WeakRef = weakref(outcomes)
	client.free()
	if reference.get_ref() != null:
		fail("Sound nodes survived client free")
		return
	print("FIXTURE OUTCOME_DONE")
	quit(0)

func wait_outcomes(observed: Array[Node], count: int) -> bool:
	var deadline := Time.get_ticks_msec() + 5000
	while observed.size() < count and Time.get_ticks_msec() < deadline:
		await process_frame
	if observed.size() != count:
		fail("Expected exactly %d outcome events, got %d" % [count, observed.size()])
		return false
	return true
