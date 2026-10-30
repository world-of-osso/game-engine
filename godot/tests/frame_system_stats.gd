extends SceneTree

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var clocks: Array[Node] = []
	for index in range(2):
		var clock: Node = ClassDB.instantiate("WowMaterialClock")
		root.add_child(clock)
		clocks.append(clock)
	var deadline := Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var stats: Dictionary = client.frame_stats()
		var systems: Dictionary = stats.systems_ms
		if systems.has("World hover") and systems.has("Culling") and systems.has("Merchant") and stats.nodes_ms.has("WowMaterialClock.process"):
			if not stats.nodes_ms.has("GameClient.process") or stats.nodes_ms["WowMaterialClock.process"] <= 0.0 or clocks[0].elapsed_time_ms() <= 0.0 or clocks[1].elapsed_time_ms() <= 0.0:
				fail("Node callback times missing while clocks advance: " + str(stats))
				return
			var total := 0.0
			for value in systems.values():
				if not is_finite(value) or value < 0.0:
					fail("Invalid system duration: " + str(stats))
					return
				total += value
			if total <= 0.0 or total > stats.client_process_ms + 0.01 or stats.attachment_gates_ms < 0.0:
				fail("Incoherent frame durations: " + str(stats))
				return
			for clock in clocks:
				clock.set_process(false)
			for index in range(3):
				await process_frame
			if client.frame_stats().nodes_ms.has("WowMaterialClock.process"):
				fail("Stopped clocks still reported process time")
				return
			print("FIXTURE FRAME_SYSTEM_STATS_DONE ", JSON.stringify(stats))
			client.free()
			quit(0)
			return
	fail("Frame steps did not start before the deadline")

func fail(message: String) -> void:
	push_error(message)
	quit(1)
