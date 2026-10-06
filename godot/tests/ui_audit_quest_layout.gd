extends SceneTree

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	for forever in [false, true]:
		var probe = ClassDB.instantiate("UiAuditProbe")
		root.add_child(probe)
		var error: String = probe.mount_quest(forever)
		if not error.is_empty():
			push_error(error)
			quit(1)
			return
		await process_frame
		var paragraph: Label = probe.find_child("QuestLogDetailsObjectivesText", true, false)
		var counter: Label = probe.find_child("QuestLogDetailsObjective0", true, false)
		if paragraph == null or counter == null or paragraph.get_line_count() < 3:
			push_error("Wine Ticket paragraph/counter missing or not wrapped")
			quit(1)
			return
		var bottom: float = paragraph.get_global_rect().end.y
		var top: float = counter.get_global_rect().position.y
		if top < bottom:
			push_error("Counter overlaps paragraph: %s < %s" % [top, bottom])
			quit(1)
			return
		print("PASS uifixes native quest layout forever=%s paragraph_bottom=%s counter_top=%s" % [forever, bottom, top])
		probe.free()
	quit(0)
