extends "res://tests/capture_talent_layout.gd"

# Actual focused LineEdit keys/signals -> RegistryUi actions, including Enter/blur ordering.
func _run() -> void:
	OS.set_environment("GODOT_PREVIEW_CLASS", "8")
	OS.set_environment("GODOT_PREVIEW_SPEC", "62")
	OS.set_environment("GODOT_SPELLBOOK_TAB", "talents")
	var ui = ClassDB.instantiate("RegistryUi")
	root.add_child(ui)
	var error: String = ui.call("show_spellbook_preview")
	assert(error.is_empty(), error)
	var search := ui.find_child("TalentSearchBox", true, false) as LineEdit
	assert(search != null)
	search.grab_focus()
	await process_frame
	assert(search.has_focus())
	assert(str(ui.call("sync_input")).is_empty())
	for command in [[KEY_DOWN, "talent:search_move:1"], [KEY_UP, "talent:search_move:-1"], [KEY_ENTER, "talent:search_submit"]]:
		var event := InputEventKey.new()
		event.keycode = command[0]
		event.pressed = true
		Input.parse_input_event(event)
		await process_frame
		var action: String = ui.call("pop_action")
		assert(action == command[1], "Expected %s, got %s" % [command[1], action])
	assert(not search.has_focus(), "Enter must release SearchBox focus after queuing submit")
	print("TALENT_FOOTER_INPUT PASS: Down/Up/Enter actions and submit-before-blur")
	ui.queue_free()
	await process_frame
	await super._run()
