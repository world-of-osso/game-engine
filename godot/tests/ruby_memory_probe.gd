extends SceneTree

## Diagnostic only: sample the real client while its normal startup flags enter a map.
var client: Node
var started := 0
var next_sample := 0

func _initialize() -> void:
	call_deferred("start_probe")

func start_probe() -> void:
	root.size = Vector2i(1280, 720)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	started = Time.get_ticks_msec()
	while Time.get_ticks_msec() - started < 180_000:
		await process_frame
		var elapsed := Time.get_ticks_msec() - started
		if elapsed >= next_sample:
			next_sample = elapsed + 2000
			print("MEMORY_SAMPLE ", JSON.stringify({
				"elapsed_ms": elapsed,
				"static": Performance.get_monitor(Performance.MEMORY_STATIC),
				"static_max": Performance.get_monitor(Performance.MEMORY_STATIC_MAX),
				"texture": Performance.get_monitor(Performance.RENDER_TEXTURE_MEM_USED),
				"buffer": Performance.get_monitor(Performance.RENDER_BUFFER_MEM_USED),
				"objects": Performance.get_monitor(Performance.OBJECT_COUNT),
				"resources": Performance.get_monitor(Performance.OBJECT_RESOURCE_COUNT),
				"nodes": Performance.get_monitor(Performance.OBJECT_NODE_COUNT),
				"orphans": Performance.get_monitor(Performance.OBJECT_ORPHAN_NODE_COUNT),
				"world_objects": JSON.parse_string(client.preview_objects_debug()),
			}))
	print("MEMORY_PROBE_DONE")
	quit(0)
