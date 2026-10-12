extends SceneTree

## Run on private UDP5881 with normal --screen inworld startup and fb_* credentials.
## PROBE_X/PROBE_Z are world X/Z; PROBE_PNG is a durable output path.
## Setup must use game-server/scripts/place-private-character.py, not legacy NPC Z.

func _initialize() -> void:
	call_deferred("run_probe")

func run_probe() -> void:
	var arguments := OS.get_cmdline_user_args()
	var server_index := arguments.find("--server")
	if server_index < 0 or server_index + 1 >= arguments.size() or arguments[server_index + 1] != "127.0.0.1:5881":
		push_error("Terrain proof requires explicit private --server 127.0.0.1:5881")
		quit(1)
		return
	if OS.get_environment("PROBE_PNG") == "" or OS.get_environment("PROBE_X") == "" or OS.get_environment("PROBE_Z") == "":
		push_error("PROBE_PNG, PROBE_X and PROBE_Z are required")
		quit(1)
		return
	root.size = Vector2i(1280, 720)
	var client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var deadline := Time.get_ticks_msec() + 180000
	var next_trace := 0
	var entered := -1
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if Time.get_ticks_msec() >= next_trace:
			next_trace = Time.get_ticks_msec() + 1000
			var x := float(OS.get_environment("PROBE_X"))
			var z := float(OS.get_environment("PROBE_Z"))
			print("TERRAIN_PROBE t=%d screen=%s predicted=%s server=%s height=%s pending=%s" % [Time.get_ticks_msec(), state.screen, state.get("local_player_position"), state.get("local_server_position"), client.terrain_height_at(x,z), state.get("terrain",{}).get("pending_count")])
		if state.screen == "InWorld":
			if entered < 0:
				entered = Time.get_ticks_msec()
			if Time.get_ticks_msec() - entered > 15000:
				var surface = client.terrain_height_at(float(OS.get_environment("PROBE_X")), float(OS.get_environment("PROBE_Z")))
				var predicted := coordinates(state.local_player_position)
				var authority := coordinates(state.local_server_position)
				var valid := surface != null and absf(predicted.y - float(surface)) < 0.01 and absf(authority.y - float(surface)) < 0.01 and float(state.local_player_health) > 0.0
				var proof := FileAccess.open(OS.get_environment("PROBE_PNG") + ".json", FileAccess.WRITE)
				proof.store_string(JSON.stringify({"pass": valid, "surface": surface, "state": state}, "\t"))
				await RenderingServer.frame_post_draw
				root.get_texture().get_image().save_png(OS.get_environment("PROBE_PNG"))
				print("TERRAIN_ACCEPTANCE pass=%s predicted=%s authority=%s surface=%s health=%s" % [valid,predicted,authority,surface,state.local_player_health])
				quit(0 if valid else 1)
				return
	push_error("probe timed out")
	quit(1)

func coordinates(value) -> Vector3:
	var parts := str(value).trim_prefix("(").trim_suffix(")").split(",")
	return Vector3(float(parts[0]), float(parts[1]), float(parts[2]))
