extends SceneTree

func _initialize() -> void:
	call_deferred("run_test")

func fail(message: String) -> void:
	push_error(message)
	quit(1)

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	for forever in [false, true]:
		var probe = ClassDB.instantiate("UiAuditProbe")
		root.add_child(probe)
		var error: String = probe.mount_mail(forever)
		if not error.is_empty():
			fail(error)
			return
		await process_frame
		var body = probe.find_child("SendMailBodyEditBox", true, false)
		if not body is TextEdit or body.size != Vector2(270, 134):
			fail("Body must be a full letter-area TextEdit, got %s" % body)
			return
		var ui = probe.find_child("MailAuditUI", true, false)
		body.grab_focus()
		body.insert_text_at_caret("Dear Theron,\nThank you for the wine.\nRegards, Suzetta")
		if body.text != "Dear Theron,\nThank you for the wine.\nRegards, Suzetta" or body.get_line_count() != 3:
			fail("Body discarded newlines")
			return
		ui.pop_action()
		body.text = "Wine\n".repeat(50)
		ui.pop_action()
		body.set_caret_line(49)
		await process_frame
		if body.get_scroll_vertical() <= 0 or not body.get_v_scroll_bar().visible:
			fail("Long letter did not scroll to caret")
			return
		body.text = "é".repeat(501)
		await process_frame
		ui.pop_action()
		if body.text.length() != 500:
			fail("Body did not enforce 500 letters")
			return
		var paper = probe.find_child("SendStationeryBackgroundLeft", true, false)
		var texture: TextureRect = null
		for attempt in range(300):
			for child in paper.get_node("Parts").get_children():
				if child is TextureRect and child.texture != null:
					texture = child
			if texture != null:
				break
			await process_frame
		if texture == null or not texture.is_visible_in_tree() or texture.texture.get_width() <= 0:
			fail("Left stationery FDID 136859 did not load into visible native texture")
			return
		print("PASS uifixes native mail body forever=%s rect=%s lines=3 scrolling=true letters=500 stationery136859=true" % [forever, body.get_global_rect()])
		probe.free()
	quit(0)
