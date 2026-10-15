extends "res://tests/adventure_journeys_watch.gd"
# Genuine realm suggestions only; no seeded eligibility, rotation or activation.

func check_suggestions_layout() -> bool:
	var pane := control("EncounterJournalSuggestedFrame")
	if pane == null:
		stop("Source Suggested pane missing")
		return false
	var scale := pane.get_global_rect().size.x / 784.0
	var expected := [Rect2(393,194,344,161), Rect2(393,22,344,161), Rect2(28,18,335,337)]
	for index in range(3):
		var slot := control("JournalSuggestionSlot" + str(index + 1))
		if slot == null:
			stop("Source suggestion slot missing")
			return false
		var actual := slot.get_global_rect()
		var rect: Rect2 = expected[index]
		if actual.position.distance_to(pane.get_global_rect().position + rect.position * scale) > 2.0 or actual.size.distance_to(rect.size * scale) > 2.0:
			stop("Suggestion slot differs from source " + str(index + 1))
			return false
	var titles := client.find_children("JournalSuggestion*Name", "Label", true, false)
	var count := int(client.adventure_guide_state().suggestion_count)
	if count < 3 or titles.size() != 3:
		stop("Provider must supply genuine primary and both small suggestions")
		return false
	for title in titles:
		var limit := 2 if control("JournalSuggestionSlot3").is_ancestor_of(title) else 1
		if title.max_lines_visible != limit:
			stop("Native title lost source line limit")
			return false
		if title.get_theme_font("font").get_font_name() != "Morpheus" or title.text.is_empty():
			stop("Authored suggestion title/font missing")
			return false
	for description in client.find_children("JournalSuggestion*Description", "Label", true, false):
		var limit := -1 if control("JournalSuggestionSlot3").is_ancestor_of(description) else 4
		if description.max_lines_visible != limit:
			stop("Native description lost source line limit")
			return false
	var icons := client.find_children("JournalSuggestion*Texture", "Control", true, false)
	if icons.is_empty():
		stop("No authentic suggestion icon available for mask proof")
		return false
	for icon in icons:
		if not masked_icon(icon.name, false):
			return false
	if not pane.clip_contents:
		stop("Suggested native pane must clip source art")
		return false
	if control("EncounterJournalContent") != null:
		stop("Generic scrolling groups remain in Suggested")
		return false
	print("SUGGESTED_AUTHORITY_BOUNDARY total_eligible=%s shown=%s primary_offset_selection_unproven ranked_first_three_selection_inference reward_identity_and_activation_unavailable" % [count, titles.size()])
	return true

func run() -> void:
	var server := OS.get_environment("ADVENTURE_GUIDE_SERVER")
	var account := OS.get_environment("ADVENTURE_GUIDE_ACCOUNT")
	shots = OS.get_environment("ADVENTURE_GUIDE_SHOTS")
	if server != "127.0.0.1:5771" or not account.begins_with("fb_") or shots.is_empty():
		stop("Explicit private5771/fb_* account and shots required")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var deadline := Time.get_ticks_msec() + 180000
	while client.account_state().assets_starting and Time.get_ticks_msec() < deadline:
		await process_frame
	var error: String = client.connect_account(server, account, OS.get_environment("ADVENTURE_GUIDE_PASSWORD"), false)
	if not error.is_empty():
		stop(error)
		return
	if not await enter_world_when_ready():
		return
	await journal_key()
	deadline = Time.get_ticks_msec() + 90000
	while (not client.adventure_guide_state().loaded or not client.adventure_guide_state().activity_snapshot_received) and Time.get_ticks_msec() < deadline:
		await process_frame
	if not await click("EncounterJournalSuggestTab"):
		return
	if not check_suggestions_layout():
		return
	for name in ["JournalSuggestedPrev", "JournalSuggestedNext"]:
		var button := control(name) as Button
		if button == null or not button.disabled:
			stop("Unbacked primary paging must retain disabled source controls")
			return
		if not await click(name) or client.adventure_guide_state().tab != "Suggested":
			stop("Disabled source paging changed selected tab")
			return
	if not await move_to("JournalSuggestionSlot2") or not await shot("suggested-source-layout-paging"):
		return
	print("SUGGESTED_LAYOUT_PASS skin=%s three-source-slots/Morpheus/roundicons/noinventedactions" % client.account_state().ui_skin)
	quit(0)
