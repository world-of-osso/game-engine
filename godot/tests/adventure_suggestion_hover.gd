extends "res://tests/adventure_suggestions_layout.gd"
# Source-record-fed fixture, no realm eligibility or authority claim.
# Only input row642 is supplied; production screen, truncation, tooltip and pointer paths remain unchanged.
var completion: RefCounted

func run() -> void:
	completion = load("res://tests/completion.gd").new(self, 3)
	completion.finish(await run_hover_proof())

func label_truncated(label: Label) -> bool:
	if label.get_line_count() > label.get_visible_line_count():
		return true
	if label.autowrap_mode != TextServer.AUTOWRAP_OFF:
		return false
	return label.get_theme_font("font").get_string_size(label.text, HORIZONTAL_ALIGNMENT_LEFT, -1, label.get_theme_font_size("font_size")).x > label.size.x

func tooltip_text(state: Dictionary) -> String:
	var parts := PackedStringArray([str(state.get("title", ""))])
	for line in state.get("lines", PackedStringArray()):
		parts.append(str(line).trim_suffix("|"))
	return " ".join(parts)

func await_tooltip(expected: bool) -> bool:
	var deadline := Time.get_ticks_msec() + 10000
	while Time.get_ticks_msec() < deadline:
		if bool(client.tooltip_state().get("visible", false)) == expected:
			await settle()
			return bool(client.tooltip_state().get("visible", false)) == expected
		await process_frame
	return completion.fail("Tooltip visible != %s: %s" % [expected, client.tooltip_state()])

func run_hover_proof() -> bool:
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
	if not completion.require(error.is_empty(), error):
		return false
	if not completion.step(await enter_world_when_ready(), "genuine world readiness"):
		return false
	await journal_key()
	deadline = Time.get_ticks_msec() + 90000
	while (not client.adventure_guide_state().loaded or not client.adventure_guide_state().activity_snapshot_received) and Time.get_ticks_msec() < deadline:
		await process_frame
	if not completion.require(client.adventure_guide_state().activity_snapshot_received, "realm activity snapshot missing"):
		return false
	if not completion.step(await click("EncounterJournalSuggestTab"), "physical Suggested Content tab"):
		return false
	print("SOURCE_FIXTURE_RECEIVED_VECTOR count=%s" % client.adventure_guide_state().suggestion_count)
	var fixture_error: String = client.install_adventure_suggestion_642_fixture()
	if not completion.require(fixture_error.is_empty(), fixture_error):
		return false
	print("SOURCE_RECORD_FED_FIXTURE row=642 no realm eligibility or authority claim")
	await settle()
	if not completion.require(client.account_state().ui_skin == OS.get_environment("ADVENTURE_GUIDE_EXPECTED_SKIN"), "skin mismatch"):
		return false
	if not completion.require(client.adventure_guide_state().suggestion_count >= 3, "genuine three-record source loop required"):
		return false
	var positive: Label
	var negative: Label
	for suffix in ["Name", "Description"]:
		for label in client.find_children("JournalSuggestion*" + suffix, "Label", true, false):
			if control("JournalSuggestionSlot3").is_ancestor_of(label):
				continue
			var truncated := label_truncated(label)
			print("SUGGESTION_NATIVE_LABEL name=%s text=%s truncated=%s lines=%s visible=%s width=%s font_size=%s wrong_key_font_size=%s" % [label.name, label.text, truncated, label.get_line_count(), label.get_visible_line_count(), label.size.x, label.get_theme_font_size("font_size"), label.get_theme_font_size("font")])
			if truncated and positive == null:
				positive = label
			elif not truncated and negative == null:
				negative = label
	if not completion.require(positive != null and negative != null, "authentic truncated/fitting secondary labels missing"):
		return false
	var positive_owner := str(positive.name).trim_suffix("Name") + "TitleFrame" if str(positive.name).ends_with("Name") else str(positive.name) + "Frame"
	if not completion.step(await move_to(positive_owner), "physical truncated-text hover") or not await await_tooltip(true):
		return false
	var state: Dictionary = client.tooltip_state()
	if not completion.require(tooltip_text(state) == positive.text, "Full text mismatch: %s != %s" % [tooltip_text(state), positive.text]):
		return false
	var tooltip_labels := client.find_children("Tooltip*", "Label", true, false)
	if not completion.require(not tooltip_labels.is_empty(), "native Tooltip labels absent"):
		return false
	for label in tooltip_labels:
		if not completion.require(label.get_theme_color("font_color").is_equal_approx(Color.WHITE), "tooltip text is not white"):
			return false
	var tooltip := control("TooltipFrame")
	var owner := control(positive_owner)
	if not completion.require(tooltip != null and tooltip.is_visible_in_tree(), "painted GameTooltip absent"):
		return false
	if not completion.require(tooltip.get_global_rect().position.distance_to(owner.get_global_rect().position + Vector2(owner.size.x * owner.get_global_transform().get_scale().x, -tooltip.get_global_rect().size.y)) < 3.0, "ANCHOR_RIGHT mismatch"):
		return false
	if not completion.step(await shot("source642-fixture-truncated-hover-full-tooltip"), "hover capture") or not completion.case_done():
		return false
	var motion := InputEventMouseMotion.new()
	motion.position = Vector2(100,100)
	motion.global_position = motion.position
	root.push_input(motion, true)
	await settle()
	if not await await_tooltip(false) or not completion.step(await shot("source642-fixture-leave-tooltip-hidden"), "leave capture") or not completion.case_done():
		return false
	var negative_owner := str(negative.name).trim_suffix("Name") + "TitleFrame" if str(negative.name).ends_with("Name") else str(negative.name) + "Frame"
	if not completion.step(await move_to(negative_owner), "physical fitting-text hover"):
		return false
	await create_timer(0.5).timeout
	if not completion.require(not client.tooltip_state().get("visible", false), "untruncated text showed tooltip"):
		return false
	if not completion.step(await shot("source642-fixture-nontruncated-hover-no-tooltip"), "fitting-text capture") or not completion.case_done():
		return false
	print("SUGGESTION_SOURCE642_FIXTURE_PASS no_realm_eligibility_or_authority_claim skin=%s truncated=%s fitting=%s" % [client.account_state().ui_skin, positive.name, negative.name])
	return true
