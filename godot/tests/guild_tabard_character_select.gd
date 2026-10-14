extends SceneTree

func _initialize() -> void:
	call_deferred("capture")

func capture() -> void:
	var directory := OS.get_environment("TABARD_CAPTURES")
	assert(not directory.is_empty())
	for skin in ["modern", "forever"]:
		var fixture: Node = ClassDB.instantiate("GuildTabardFixture")
		root.add_child(fixture)
		assert(fixture.call("show_character_selection", skin == "forever") == "")
		var ready := false
		for frame in range(2400):
			var error: String = fixture.call("tick", 0.0)
			if not error.is_empty():
				push_error("Registered roster model: " + error)
				quit(1)
				return
			if fixture.call("receipt")["character_select_ready"]:
				ready = true
				break
			await process_frame
		assert(ready, "Registered roster model and authored campsite must load")
		assert(fixture.get_node("CharacterSelectStage/CharacterSelectScene/SelectedCharacter") != null)
		var ui := fixture.get_node("CharacterSelectUI")
		assert(ui.find_child("CharSelectCharacterName", true, false).text == "Guildmaster")
		for frame in range(6):
			await process_frame
			await RenderingServer.frame_post_draw
		var image := root.get_texture().get_image()
		assert(image.get_size() == Vector2i(1920, 1080))
		assert(image.save_png(directory.path_join(skin + "-tabard-character-select.png")) == OK)
		fixture.queue_free()
		await process_frame
	print("PASS: both-skin production character-selection roster projection/model/campsite with persisted tabard IDs; offline fixture")
	quit(0)
