extends SceneTree

## Offline multi-map stream: load several maps in one client and report the object queue
## when each one settles. Used to check that per-map resources are released between maps.

const SETTLE_LIMIT_MS := 180_000

func _initialize() -> void:
	call_deferred("run_census")

func run_census() -> void:
	root.size = Vector2i(1280, 720)
	var client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await process_frame
	# Directory, engine-space position (WoW x, z, -y) for a pose inside the map.
	var maps := [
		["2993", Vector3(1249.664, -1949.646, 47.73596)],
		["2859", Vector3(1239.979, -1739.592, -72.21029)],
		["2521", Vector3(1441.94, 161.278, -127.2535)],
		["2993", Vector3(1249.664, -1949.646, 47.73596)],
	]
	for entry in maps:
		var directory: String = entry[0]
		var requested: String = client.preview_world_map(directory, entry[1])
		if requested != "":
			push_error("preview %s: %s" % [directory, requested])
			quit(1)
			return
		var deadline := Time.get_ticks_msec() + SETTLE_LIMIT_MS
		var state := {}
		while Time.get_ticks_msec() < deadline:
			await process_frame
			state = JSON.parse_string(client.preview_objects_debug())
			var idle := int(state.get("pending", 0)) == 0 \
				and int(state.get("first", 0)) == 0 \
				and int(state.get("ready", 0)) == 0 \
				and int(state.get("waiting", 0)) == 0 \
				and int(state.get("arrived", 0)) == 0 \
				and int(state.get("loading", 0)) == 0
			if idle and (int(state.get("spawned_doodads", 0)) + int(state.get("spawned_wmos", 0)) > 0):
				break
		print("MAP_SETTLED ", directory, " ", JSON.stringify(state))
	print("MAP_CENSUS_DONE")
	client.free()
	quit(0)
