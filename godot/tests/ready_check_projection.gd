extends SceneTree

# Same ReadyCheckUpdate -> group_frames_state -> RegistryUi -> UiProjection path as live.
func _initialize() -> void:
	call_deferred("run_test")

func check(condition: bool, message: String) -> void:
	if not condition:
		push_error(message)
		quit(1)
		assert(condition, message)

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	for forever in [false, true]:
		var fixture = ClassDB.instantiate("PartyPortraitFixture")
		root.add_child(fixture)
		check(fixture.initialize(forever) == "", "initialize group HUD")
		var error: String = fixture.apply_ready_check(false)
		check(error == "", "ready-check native projection: " + error)
		await process_frame
		for pair in [["YesButton", "Ready"], ["NoButton", "Not Ready"]]:
			var button = fixture.find_child("ReadyCheckFrame" + pair[0], true, false)
			check(button is Button and button.is_visible_in_tree(), "visible " + pair[1] + " native button")
			check(button.size.x > 0 and button.size.y > 0, "laid-out " + pair[1] + " button")
			check(button.get_node("Parts/Text").text == pair[1], "button label " + pair[1])
		var title = fixture.find_child("ReadyCheckFrameTitle", true, false)
		var text = fixture.find_child("ReadyCheckFrameText", true, false)
		check(title.text == "Ready Check", "title projected")
		check(text.text == "Ann has initiated a ready check.", "initiator message projected")
		var border = fixture.find_child("ReadyCheckFrameBorder", true, false)
		check(border.get_node_or_null("Parts/Part0") is TextureRect, "Retail dialog art projected")
		check(fixture.apply_ready_check(true) == "", "finished update projects")
		await process_frame
		check(fixture.find_child("ReadyCheckFrame", true, false) == null, "timeout removes popup")
		fixture.free()
		print("PASS ready_check_native_projection ", "Forever" if forever else "Modern")
	quit(0)
