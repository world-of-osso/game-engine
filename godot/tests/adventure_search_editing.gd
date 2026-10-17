extends "res://tests/adventure_search.gd"
# Continued physical typing after short-query Enter; focus alone is insufficient.

func press_key(code: Key) -> void:
	await super.press_key(code)
	if code != KEY_ENTER or label_text("EncounterJournalSearchBox") != "fl":
		return
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = KEY_A
		event.physical_keycode = KEY_A
		event.unicode = "a".unicode_at(0) if pressed else 0
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await settle()
	var actual := label_text("EncounterJournalSearchBox")
	print("SEARCH_CONTINUED_INPUT query=%s focused=%s" % [actual,control("EncounterJournalSearchBox").has_focus()])
	if actual != "fla" or not shown("EncounterJournalSearchPreview"):
		stop("Short-query Enter retained focus but stopped physical typing: " + actual)
