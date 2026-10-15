extends "res://tests/adventure_journeys_watch.gd"
# Physical navigation and authored creatures; no catalog/model injection.

func expect_model(creature: int, display: int, title: String) -> bool:
	var deadline := Time.get_ticks_msec() + 90000
	while client.adventure_guide_state().model_display != display and Time.get_ticks_msec() < deadline:
		await process_frame
	var state: Dictionary = client.adventure_guide_state()
	if state.creature != creature or state.model_display != display or label_text("EncounterJournalModelTitle") != title:
		stop("Selected creature/model/title mismatch: " + str(state))
		return false
	for frame in 30:
		await process_frame
	print("MODEL_SELECTOR_ATTACHED creature=%s display=%s title=%s" % [creature, display, title])
	return true

func reveal_instance(name: String) -> bool:
	for attempt in 40:
		var row := control(name)
		var list := control("EncounterJournalContent")
		if row != null and list.get_global_rect().has_point(row.get_global_rect().get_center()):
			return true
		if not await wheel("EncounterJournalContent", MOUSE_BUTTON_WHEEL_DOWN):
			return false
	stop("Instance not revealed: " + name)
	return false

func open_instance(tab: String, instance: int) -> bool:
	if not await click(tab) or not await click("EncounterJournalTierDropdown") or not await click("EncounterJournalTier73"):
		return false
	var name := "JournalInstance%s" % instance
	return await reveal_instance(name) and await click(name)

func verify_selection_sizes(selected: String, other: String) -> bool:
	var selected_button := control(selected) as BaseButton
	var other_button := control(other) as BaseButton
	if selected_button == null or other_button == null:
		stop("Authored creature selector missing")
		return false
	var scale := selected_button.size.x / 64.0
	if not selected_button.disabled or other_button.disabled or absf(selected_button.size.y - 61.0 * scale) > 1.0 or other_button.size.distance_to(Vector2(50,49) * scale) > 1.0:
		stop("Source selected64x61/other50x49 or disabled state mismatch")
		return false
	return true

func verify_rendered_portrait(name: String, display: int) -> bool:
	var deadline := Time.get_ticks_msec() + 30000
	var state: Dictionary = client.unit_portrait_state(name)
	while not state.get("model_shown", false) and Time.get_ticks_msec() < deadline:
		await process_frame
		state = client.unit_portrait_state(name)
	if not state.get("model_shown", false) or not state.get("mask_loaded", false) or state.get("mask_fdid", 0) != 130924 or state.get("appearance", "") != "creature display %s items 0" % display:
		stop("Authored display portrait/mask missing for %s: %s" % [name, state])
		return false
	await RenderingServer.frame_post_draw
	state = client.unit_portrait_state(name)
	var image: Image = state.get("image")
	if image == null or image.is_empty():
		stop("Rendered portrait image absent: " + name)
		return false
	var painted := 0
	for y in image.get_height():
		for x in image.get_width():
			var pixel := image.get_pixel(x,y)
			if maxf(pixel.r,maxf(pixel.g,pixel.b)) > 0.08 and pixel.a > 0.5:
				painted += 1
	if painted < 20:
		stop("Rendered portrait image is blank: " + name)
		return false
	print("MODEL_RENDERED_PORTRAIT %s display=%s painted=%s" % [name,display,painted])
	return true

func verify_portrait_corners(name: String) -> bool:
	# Compare actual framebuffer against the same frame without this portrait.
	# Circle mask must clear corners while retaining visible central art.
	var portrait := control(name)
	var rect := portrait.get_global_rect()
	await RenderingServer.frame_post_draw
	var painted := root.get_texture().get_image()
	portrait.hide()
	await process_frame
	await RenderingServer.frame_post_draw
	var backdrop := root.get_texture().get_image()
	portrait.show()
	await settle()
	var changed_corners := 0
	var changed_center := 0
	for y in range(2, int(rect.size.y) - 2):
		for x in range(2, int(rect.size.x) - 2):
			var point := Vector2i(rect.position) + Vector2i(x,y)
			var difference := painted.get_pixelv(point) - backdrop.get_pixelv(point)
			var changed := Vector3(difference.r, difference.g, difference.b).length() > 0.04
			var normalized := Vector2(x,y) / rect.size - Vector2(0.5,0.5)
			if normalized.length() > 0.55 and changed:
				changed_corners += 1
			if normalized.length() < 0.3 and changed:
				changed_center += 1
	print("MODEL_PORTRAIT_PIXELS %s corners=%s center=%s" % [name, changed_corners, changed_center])
	if changed_corners > 0 or changed_center == 0:
		stop("Source circular portrait corners/visible center failed: " + name)
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
	await settle()
	if not await open_instance("EncounterJournalDungeonTab", 63):
		return
	if client.adventure_guide_state().difficulty != 1:
		if not await click("JournalDifficultyDropdown") or not await click("JournalDifficulty1"):
			return
	if not await click("JournalBoss90") or not await click("EncounterJournalPage3"):
		return
	if not await expect_model(359, 33002, "Helix Gearbreaker") or not verify_selection_sizes("JournalCreature359", "JournalCreature630"):
		return
	if not await move_to("JournalCreature630") or not await shot("model-selector-helix"):
		return
	if not await verify_rendered_portrait("JournalCreature359Portrait", 33002) or not await verify_rendered_portrait("JournalCreature630Portrait", 33003):
		return
	if not await verify_portrait_corners("JournalCreature359Portrait") or not await verify_portrait_corners("JournalCreature630Portrait"):
		return
	if not await move_to("JournalCreature630"):
		return
	if client.tooltip_state().get("title", "") != "Lumbering Oaf":
		stop("Authored Oaf tooltip absent")
		return
	if not await click("JournalCreature630") or not await expect_model(630, 33003, "Lumbering Oaf") or not verify_selection_sizes("JournalCreature630", "JournalCreature359"):
		return
	if not await move_to("EncounterJournalModelTitle") or not await shot("model-selector-oaf"):
		return
	if not await click("EncounterJournalPage1") or not await click("EncounterJournalPage3") or not await expect_model(359, 33002, "Helix Gearbreaker"):
		return
	if not await open_instance("EncounterJournalRaidTab", 74) or not await click("JournalBoss154") or not await click("EncounterJournalPage3"):
		return
	if not await click("JournalCreature555") or not await expect_model(555, 35232, "Nezir") or not await move_to("JournalCreature555"):
		return
	var tooltip: Dictionary = client.tooltip_state()
	if tooltip.get("title", "") != "Nezir" or not str(tooltip.get("lines", "")).contains("Lord of the North Wind"):
		stop("Authored Nezir tooltip description absent")
		return
	if not await shot("model-selector-authored-tooltip"):
		return
	if not await verify_rendered_portrait("JournalCreature555Portrait", 35232) or not await verify_portrait_corners("JournalCreature555Portrait"):
		return
	print("MODEL_SELECTOR_PASS skin=%s physicalHelix/Oaf/reset/Nezir-tooltip" % client.account_state().ui_skin)
	quit(0)
