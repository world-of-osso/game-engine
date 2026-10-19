extends "res://tests/adventure_suggestion_hover.gd"
# Source-record-fed primary input only; no realm primary-offset/eligibility authority.
# Never change title bytes, title width, chosen font or geometry to force truncation.
func run() -> void:
	completion = load("res://tests/completion.gd").new(self, 4)
	completion.finish(await run_primary_hover())

func install_primary(id: int) -> bool:
	var error: String = client.install_adventure_primary_title_fixture(id)
	if not completion.require(error.is_empty(), error):
		return false
	await settle()
	var owner := control("JournalSuggestion%sTitleFrame" % id)
	var label := control("JournalSuggestion%sName" % id) as Label
	if not completion.require(owner != null and label != null and control("JournalSuggestionSlot3").is_ancestor_of(label), "Source row not in production primary pane"):
		return false
	print("PRIMARY_SOURCE_NATIVE_LABEL ", JSON.stringify({"id":id,"text":label.text,"font_size":label.get_theme_font_size("font_size"),"rect":str(label.get_global_rect()),"width":label.size.x,"lines":label.get_line_count(),"visible":label.get_visible_line_count(),"truncated":label_truncated(label),"source_record_fed_no_primary_offset_authority":true}))
	return true

func verify_primary_tooltip(label: Label, owner: Control) -> bool:
	var state: Dictionary = client.tooltip_state()
	if not completion.require(tooltip_text(state) == label.text, "Full primary source title mismatch: %s != %s" % [tooltip_text(state),label.text]):
		return false
	var painted := 0
	for text in client.find_children("Tooltip*", "Label", true, false):
		if text.text.is_empty():
			continue
		painted += 1
		if not completion.require(text.get_theme_color("font_color").is_equal_approx(Color.WHITE), "Primary tooltip text is not white"):
			return false
	var tooltip := control("TooltipFrame")
	if not completion.require(painted > 0 and tooltip != null and tooltip.is_visible_in_tree(), "Painted primary title tooltip missing"):
		return false
	var expected := owner.get_global_rect().position + Vector2(owner.size.x * owner.get_global_transform().get_scale().x, -tooltip.get_global_rect().size.y)
	return completion.require(tooltip.get_global_rect().position.distance_to(expected) < 3.0, "Primary title ANCHOR_RIGHT mismatch")

func run_primary_hover() -> bool:
	var server := OS.get_environment("ADVENTURE_GUIDE_SERVER")
	var account := OS.get_environment("ADVENTURE_GUIDE_ACCOUNT")
	shots = OS.get_environment("ADVENTURE_GUIDE_SHOTS")
	if not completion.require(server == "127.0.0.1:5771" and account.begins_with("fb_") and not shots.is_empty(), "private input boundary"):
		return false
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var deadline := Time.get_ticks_msec() + 180000
	while client.account_state().assets_starting and Time.get_ticks_msec() < deadline:
		await process_frame
	if not completion.require(not client.account_state().assets_starting, "asset startup timeout"):
		return false
	var error: String = client.connect_account(server, account, OS.get_environment("ADVENTURE_GUIDE_PASSWORD"), false)
	if not completion.require(error.is_empty(), error) or not completion.step(await enter_world_when_ready(), "genuine world readiness"):
		return false
	await journal_key()
	deadline = Time.get_ticks_msec() + 90000
	while (not client.adventure_guide_state().loaded or not client.adventure_guide_state().activity_snapshot_received) and Time.get_ticks_msec() < deadline:
		await process_frame
	if not completion.require(client.adventure_guide_state().activity_snapshot_received, "realm activity snapshot missing"):
		return false
	if not completion.step(await click("EncounterJournalSuggestTab"), "physical Suggested Content tab"):
		return false
	if not completion.require(client.account_state().ui_skin == OS.get_environment("ADVENTURE_GUIDE_EXPECTED_SKIN"), "skin mismatch") or not await install_primary(823):
		return false
	var positive := control("JournalSuggestion823Name") as Label
	if not completion.step(await shot("primary823-source-fit-before-hover"), "source fit capture"):
		return false
	if not completion.require(label_truncated(positive), "Authentic primary row823 fits production title; positive hover proof unexercised"):
		return false
	if not completion.step(await move_to("JournalSuggestion823TitleFrame"), "physical primary truncated-title hover") or not await await_tooltip(true):
		return false
	if not verify_primary_tooltip(positive, control("JournalSuggestion823TitleFrame")) or not completion.step(await shot("primary823-truncated-full-white-right-tooltip"), "primary hover capture") or not completion.case_done():
		return false
	var motion := InputEventMouseMotion.new()
	motion.position = Vector2(100,100)
	motion.global_position = motion.position
	root.push_input(motion, true)
	await settle()
	if not await await_tooltip(false) or not completion.step(await shot("primary823-leave-tooltip-hidden"), "primary leave capture") or not completion.case_done():
		return false
	if not await install_primary(881):
		return false
	var negative := control("JournalSuggestion881Name") as Label
	if not completion.require(not label_truncated(negative), "Authentic fitting primary row881 no longer fits"):
		return false
	if not completion.step(await move_to("JournalSuggestion881TitleFrame"), "physical fitting primary title hover"):
		return false
	await create_timer(0.5).timeout
	if not completion.require(not client.tooltip_state().get("visible", false), "Fitting primary title showed tooltip") or not completion.step(await shot("primary881-fitting-no-tooltip"), "fitting title capture") or not completion.case_done():
		return false
	if not completion.step(await move_to("JournalSuggestion881DescriptionFrame"), "physical primary description negative"):
		return false
	await create_timer(0.5).timeout
	if not completion.require(not client.tooltip_state().get("visible", false), "Primary description gained out-of-scope hover") or not completion.case_done():
		return false
	print("PRIMARY_TITLE_SOURCE_FIXTURE_PASS no_realm_primary_offset_authority skin=%s" % client.account_state().ui_skin)
	return true
