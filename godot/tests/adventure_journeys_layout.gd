extends SceneTree
# Actual private realm -> registry -> native pointer/tooltip/wheel boundary, both skins.
var client: Node
var shots: String

func _initialize() -> void:
	call_deferred("run")

func stop(reason: String) -> void:
	var state: Dictionary = client.adventure_guide_state() if client != null else {}
	var summary := {}
	for key in ["visible", "loaded", "tab", "renown_faction_count", "renown_received_faction_count", "journal_spell_catalog_ready", "journal_item_catalog_ready", "journey", "journey_level"]:
		summary[key] = state.get(key)
	push_error("JOURNEYS_LAYOUT_FAIL: " + reason + " journal=" + str(summary))
	quit(1)

func settle() -> void:
	for _frame in range(8):
		await process_frame

func control(name: String) -> Control:
	return client.find_child(name, true, false) as Control

func move_to(name: String) -> bool:
	var node := control(name)
	if node == null or not node.is_visible_in_tree():
		stop("Missing visible control " + name)
		return false
	var point := node.get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await settle()
	return true

func click(name: String) -> bool:
	if not await move_to(name):
		return false
	var point := control(name).get_global_rect().get_center()
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await settle()
	return true

func wheel(name: String, button: int) -> bool:
	if not await move_to(name):
		return false
	var point := control(name).get_global_rect().get_center()
	var event := InputEventMouseButton.new()
	event.position = point
	event.global_position = point
	event.button_index = button
	event.pressed = true
	root.push_input(event, true)
	await settle()
	return true

func pointer_button(point: Vector2, pressed: bool) -> void:
	var event := InputEventMouseButton.new()
	event.position = point
	event.global_position = point
	event.button_index = MOUSE_BUTTON_LEFT
	event.pressed = pressed
	root.push_input(event, true)
	await process_frame

func held_arrow() -> bool:
	if not await move_to("JourneyLevelNext"):
		return false
	var start := control("JourneyRenownLevel2").get_global_rect().position.x
	await pointer_button(control("JourneyLevelNext").get_global_rect().get_center(), true)
	await create_timer(0.8).timeout
	var moving := control("JourneyRenownLevel2").get_global_rect().position.x
	if moving >= start - 0.1:
		stop("Held arrow did not slide before release start=%s moving=%s" % [start, moving])
		return false
	if not await shot("journeys-held-arrow"):
		return false
	# Release outside the arrow: physical release must stop scrolling everywhere.
	await pointer_button(Vector2(5,5), false)
	await settle()
	var selected: int = client.adventure_guide_state().journey_level
	if selected < 3:
		stop("Right release did not snap forward")
		return false
	await create_timer(0.8).timeout
	if client.adventure_guide_state().journey_level != selected:
		stop("Renown sliding continued after release")
		return false
	print("JOURNEYS_HOLD_PASS start=%s moving=%s released=%s" % [start,moving,selected])
	return await click("JourneyRenownLevel2")

func held_left_fractional() -> bool:
	if not await move_to("JourneyLevelPrev"):
		return false
	await pointer_button(control("JourneyLevelPrev").get_global_rect().get_center(), true)
	var deadline := Time.get_ticks_msec() + 10000
	while client.adventure_guide_state().journey_track_offset >= 53. and Time.get_ticks_msec() < deadline:
		await process_frame
	await RenderingServer.frame_post_draw
	var state: Dictionary = client.adventure_guide_state()
	var arrow := control("JourneyLevelPrev") as BaseButton
	if not state.journey_scrolling or state.journey_level != 1 or state.journey_track_offset <= 0 or arrow == null or arrow.disabled:
		stop("Left arrow disabled before zero offset=%s level=%s" % [state.journey_track_offset, state.journey_level])
		return false
	if not await shot("journeys-held-left"):
		return false
	await pointer_button(Vector2(5,5), false)
	await settle()
	if client.adventure_guide_state().journey_level != 1 or client.adventure_guide_state().journey_scrolling:
		stop("Left release did not snap1 and stop")
		return false
	if not await click("JourneyLevelNext"):
		return false
	await create_timer(0.8).timeout
	if client.adventure_guide_state().journey_level != 2 or client.adventure_guide_state().journey_scrolling:
		stop("Short right press must step exactly once")
		return false
	print("JOURNEYS_LEFT_HOLD_PASS fractional=%s arrow-enabled/release1/short2" % state.journey_track_offset)
	return true

func held_boundary() -> bool:
	if not await click("JourneyRenownLevel24") or not await move_to("JourneyLevelNext"):
		return false
	await pointer_button(control("JourneyLevelNext").get_global_rect().get_center(), true)
	var deadline := Time.get_ticks_msec() + 15000
	while client.adventure_guide_state().journey_level != 25 and Time.get_ticks_msec() < deadline:
		await process_frame
	await settle()
	var arrow := control("JourneyLevelNext") as BaseButton
	if client.adventure_guide_state().journey_level != 25 or client.adventure_guide_state().journey_scrolling or arrow == null or not arrow.disabled:
		stop("Right boundary must disable and stop while held")
		return false
	if not await shot("journeys-held-boundary"):
		return false
	await pointer_button(Vector2(5,5), false)
	await create_timer(0.8).timeout
	if client.adventure_guide_state().journey_level != 25:
		stop("Boundary release stepped beyond25")
		return false
	print("JOURNEYS_BOUNDARY_PASS held-stop25/release25")
	return true

func trace_open(sequence: int, action: String) -> void:
	var state: Dictionary = client.adventure_guide_state()
	print("RENOWN_REPRO utc=%s ticks_usec=%s event_id=%s action=%s nominal_request_type=RenownRequest::Query reply_type=RenownSnapshot displayed=%s received=%s spells_ready=%s items_ready=%s" % [Time.get_datetime_string_from_system(true), Time.get_ticks_usec(), sequence, action, state.renown_faction_count, state.renown_received_faction_count, state.journal_spell_catalog_ready, state.journal_item_catalog_ready])

func journal_key() -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = KEY_J
		event.physical_keycode = KEY_J
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame

func snapshot_repro() -> void:
	for sequence in range(1,5):
		trace_open(sequence, "before-open")
		await journal_key()
		trace_open(sequence, "after-open")
		await create_timer(20.).timeout
		trace_open(sequence, "after20s-before-close")
		await journal_key()
		trace_open(sequence, "after-close")
		await create_timer(1.).timeout
	print("RENOWN_REPRO_DONE event_ids=1..4 local-observation-only; protocol-has-no-request-id")
	quit(0)

func label_text(name: String) -> String:
	var node := client.find_child(name, true, false)
	return node.text if node != null and "text" in node else ""

func shot(label: String) -> bool:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	if image.get_width() != 1920 or image.get_height() != 1080:
		stop("Expected original1920x1080, got %sx%s" % [image.get_width(), image.get_height()])
		return false
	var path := shots.path_join("%s-%s.png" % [client.account_state().ui_skin.to_lower(), label])
	if image.save_png(path) != OK:
		stop("Cannot save " + path)
		return false
	print("JOURNEYS_LAYOUT_SHOT " + path)
	return true

func masked_icon(name: String, inset: bool) -> bool:
	var owner := client.find_child(name, true, false) as Control
	var node := owner.find_child("Part0", true, false) as TextureRect if owner != null else null
	if node == null or node.texture == null:
		stop("Missing masked icon " + name)
		return false
	var image := node.texture.get_image()
	if image.is_compressed() and image.decompress() != OK:
		stop("Cannot decode mask proof " + name)
		return false
	if image.get_pixel(0,0).a > 0.01 or image.get_pixel(image.get_width()/2,image.get_height()/2).a < 0.7:
		stop("Mask corner/centre wrong " + name)
		return false
	if inset and image.get_pixel(0,image.get_height()/2).a > 0.01:
		stop("40px choice mask must be inset inside45px reward icon")
		return false
	print("JOURNEYS_MASK_PASS " + name)
	return true

func wait_screen(screen: String) -> bool:
	var deadline := Time.get_ticks_msec() + 180000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if client.account_state().screen == screen:
			return true
	stop("Screen not ready " + screen)
	return false

func run() -> void:
	var server := OS.get_environment("ADVENTURE_GUIDE_SERVER")
	var account := OS.get_environment("ADVENTURE_GUIDE_ACCOUNT")
	var expected_count := int(OS.get_environment("ADVENTURE_GUIDE_FACTION_COUNT"))
	shots = OS.get_environment("ADVENTURE_GUIDE_SHOTS")
	if server != "127.0.0.1:5771" or not account.begins_with("fb_") or shots.is_empty() or expected_count <= 0:
		stop("Explicit private5771/fb_* account, published catalog count and screenshot directory required")
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
	if not await wait_screen("CharacterSelect"):
		return
	# CharacterSelect is entered before its asynchronous preview reveals EnterWorld.
	deadline = Time.get_ticks_msec() + 180000
	while Time.get_ticks_msec() < deadline:
		var enter := control("EnterWorld")
		if enter != null and enter.is_visible_in_tree():
			break
		await process_frame
	if not await click("EnterWorld"):
		return
	if not await wait_screen("InWorld"):
		return
	if OS.get_environment("ADVENTURE_GUIDE_REPRO_LOOP") == "1":
		await snapshot_repro()
		return
	trace_open(1, "before-first-open")
	await journal_key()
	trace_open(1, "after-first-open")
	deadline = Time.get_ticks_msec() + 90000
	var received_logged := false
	while (not client.adventure_guide_state().loaded or client.adventure_guide_state().renown_faction_count < 0) and Time.get_ticks_msec() < deadline:
		if not received_logged and client.adventure_guide_state().renown_received_faction_count >= 0:
			trace_open(1, "received-cache-first-observed")
			received_logged = true
		await process_frame
	trace_open(1, "snapshot-wait-complete")
	if client.adventure_guide_state().renown_faction_count != expected_count:
		stop("Journeys did not become display-ready; received authority and local prerequisites are separate")
		return
	if not await click("EncounterJournalJourneysTab") or not await shot("journeys-midnight"):
		return
	if not await click("JournalJourneysExpansion") or not await shot("journeys-expansion-menu"):
		return
	if not await click("JournalJourneysExpansion9") or not await shot("journeys-dragonflight"):
		return
	if not await move_to("JourneyCard12"):
		return
	if control("JourneyCard12WatchCheckbox") != null:
		stop("Unevaluated Maruuk gate exposed a watched-faction toggle")
		return
	if not await click("JourneyCard12") or not await click("JourneyRenownLevel2"):
		return
	if client.adventure_guide_state().journey != 12 or client.adventure_guide_state().journey_level != 2:
		stop("Physical faction/level selection did not resolve Maruuk2")
		return
	if not masked_icon("JourneyRenownLevel2*Icon", false) or not masked_icon("JourneyReward622Icon", true):
		return
	var details := control("JourneyProgressDetails")
	if control("JourneyProgressUnlock") != null or details == null or not details.is_visible_in_tree():
		stop("Received progress hidden by unknown unlock or developer notice")
		return
	if label_text("JourneyProgressLevel") != "2" or label_text("JourneyProgressCurrent") != "Current Progress: 1000 / 2500":
		stop("Authoritative current level/progress header missing")
		return
	if not await move_to("JourneyProgressName") or not await shot("journeys-progress-header"):
		return
	if not await move_to("JourneyReward622Card"): 
		return
	var tooltip: Dictionary = client.tooltip_state()
	if not tooltip.visible or tooltip.title != "Centaur Hunting Trophies":
		stop("Authored reward tooltip absent: " + str(tooltip))
		return
	if not await shot("journeys-reward-tooltip"):
		return
	if not await move_to("JourneyProgressName") or not await shot("journeys-renown-progress"):
		return
	if not await held_arrow() or not await held_left_fractional():
		return
	if not await wheel("JourneyRenownTrackClip", MOUSE_BUTTON_WHEEL_DOWN):
		return
	if client.adventure_guide_state().journey_level != 3:
		stop("Renown progress wheel did not select3; actual=" + str(client.adventure_guide_state().journey_level))
		return
	if not await click("JourneyLevelFirst"):
		return
	if client.adventure_guide_state().journey_level != 1:
		stop("Jump-left did not select1")
		return
	if not await click("JourneyLevelLast"):
		return
	if client.adventure_guide_state().journey_level != 25:
		stop("Jump-right did not select authoritative Maruuk maximum25")
		return
	if not await held_boundary() or not await click("EncounterJournalBack"):
		return
	var card := control("JourneyCard12")
	var scale := control("EncounterJournal").get_global_rect().size / Vector2(800,496)
	if card == null or card.get_global_rect().size.distance_to(Vector2(374,112)*scale)>1.5:
		stop("Shared Home did not return to source-sized Maruuk card")
		return
	print("JOURNEYS_LAYOUT_PASS skin=%s wheel3/jump1/25/tooltip/Home" % client.account_state().ui_skin)
	quit(0)
