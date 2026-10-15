extends "res://tests/adventure_journeys_watch.gd"
# Inherits only physical input and genuine login/readiness helpers, not watch execution.

func check_tutorial_layout() -> bool:
	var pane := control("EncounterJournalTutorialsFrame")
	var art := control("EncounterJournalTutorialArt")
	var title := control("EncounterJournalTutorialTitle") as Label
	var body := control("EncounterJournalTutorials") as Label
	var divider := control("EncounterJournalTutorialDivider")
	var start := control("EncounterJournalTutorialStart") as Button
	if pane == null or art == null or title == null or body == null or divider == null or start == null:
		stop("Source Tutorials pane/art/text/divider/button missing")
		return false
	var scale := pane.get_global_rect().size.x / 784.0
	if not pane.clip_contents:
		stop("Tutorial native pane must clip its authored art")
		return false
	if not start.disabled:
		stop("Unsupported Start must be disabled")
		return false
	if title.get_theme_font("font").get_font_name() != "Morpheus":
		stop("Fancy27Font did not load published Morpheus")
		return false
	if title.text.is_empty() or body.text.is_empty() or start.text.is_empty():
		stop("Authored RPE strings missing")
		return false
	if abs(art.get_global_rect().size.x - 700.0 * scale) > 1.0 or abs(art.get_global_rect().size.y - 342.0 * scale) > 1.0:
		stop("Tutorial art stretched away from source700x342")
		return false
	if abs(divider.get_global_rect().position.y - title.get_global_rect().end.y - 16.0 * scale) > 2.0:
		stop("Divider lost title-relative16px anchor")
		return false
	if abs(body.get_global_rect().position.y - divider.get_global_rect().end.y - 16.0 * scale) > 2.0:
		stop("Description lost divider-relative16px anchor")
		return false
	if control("EncounterJournalTutorialUnavailable") != null:
		stop("Custom unavailable explanation remains")
		return false
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
	while not client.adventure_guide_state().loaded and Time.get_ticks_msec() < deadline:
		await process_frame
	if not await click("EncounterJournalTutorialsTab"):
		return
	if not check_tutorial_layout() or not await shot("tutorials-source-layout"):
		return
	if not await click("EncounterJournalTutorialStart"):
		return
	if client.adventure_guide_state().tab != "Tutorials":
		stop("Disabled Start changed authoritative screen/tab")
		return
	if not await shot("tutorials-disabled-start"):
		return
	print("TUTORIALS_LAYOUT_PASS skin=%s sourceart/Morpheus/materialtext/anchors/disabledStart" % client.account_state().ui_skin)
	quit(0)
