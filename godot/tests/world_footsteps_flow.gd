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
	var error: String = await load("res://tests/native_footstep_probe.gd").new().check(self, client, player, locomotion)
	if error != "":
		fail(error)
		return
	print("FIXTURE FOOTSTEPS_DONE")
	client.free()
	quit(0)
