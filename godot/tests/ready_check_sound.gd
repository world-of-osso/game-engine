extends SceneTree
# Native decoded dispatch only: shared HUD visibility/edge helpers and real UiKits.
# This is not a live server/receiver session or hardware audibility proof.
func _initialize() -> void:
	call_deferred("run_sound")

func settle() -> void:
	for frame in range(4):
		await process_frame
		await RenderingServer.frame_post_draw

func players(fixture: Node) -> Array[Dictionary]:
	var observations: Array[Dictionary] = []
	for node in fixture.find_children("*", "AudioStreamPlayer", true, false):
		var player := node as AudioStreamPlayer
		var stream := player.stream
		assert(stream != null and stream is AudioStreamOggVorbis)
		assert(stream.resource_name == "567478")
		assert(stream.get_length() > 0)
		assert(is_equal_approx(player.volume_linear, 1.0))
		assert(is_equal_approx(player.pitch_scale, 1.0))
		observations.append({"file": stream.resource_name, "length": stream.get_length(), "playing": player.playing, "gain": player.volume_linear, "pitch": player.pitch_scale})
	return observations

func run_sound() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	var output := OS.get_environment("READY_SOUND_CAPTURE_PATH")
	assert(not output.is_empty())
	var receipts: Array[Dictionary] = []
	for forever in [false, true]:
		var skin := "forever" if forever else "modern"
		var fixture := ClassDB.instantiate("ReadyCheckSoundFixture") as Node
		root.add_child(fixture)
		assert(fixture.initialize(forever) == "")
		var phases := [["hidden", 0, false], ["initiator", 0, false], ["listener", 1, true], ["refresh", 1, true], ["answered", 1, false], ["reopen", 2, true], ["refresh", 2, true], ["finished", 2, false], ["no-viewer", 2, false], ["hidden", 2, false]]
		for phase in phases:
			assert(fixture.step(phase[0]) == "")
			await settle()
			var state: Dictionary = fixture.state()
			assert(state.played.size() == phase[1])
			assert(state.shown == phase[2])
			for kit in state.played:
				assert(kit == 8960)
			var active := players(fixture)
			if phase[0] == "listener" or phase[0] == "reopen":
				assert(not active.is_empty() and active[-1].playing, "Decoded kit actually dispatched to a playing native player")
				assert(root.get_texture().get_image().save_png(output.path_join(skin + "-" + phase[0] + "-audio-context.png")) == OK)
			receipts.append({"skin": skin, "phase": phase[0], "shown": state.shown, "played": state.played, "players": active})
		# Natural stream completion, not a hardware-output assertion.
		await create_timer(5.2).timeout
		for player in players(fixture):
			assert(not player.playing)
		fixture.free()
		await settle()
		assert(root.find_children("*", "AudioStreamPlayer", true, false).is_empty())
	var receipt := FileAccess.open(OS.get_environment("READY_SOUND_RECEIPT"), FileAccess.WRITE)
	assert(receipt != null)
	receipt.store_string(JSON.stringify(receipts, "\t"))
	receipt.close()
	print("PASS ready-check-sound both-skin decoded dispatch, lifecycle silence and cleanup")
	quit(0)
