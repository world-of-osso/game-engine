extends SceneTree
# Actual private realm -> registry -> native pointer/tooltip/wheel boundary, both skins.
var client: Node
var shots: String

func _initialize() -> void:
	call_deferred("run")

func stop(reason: String) -> void:
	var state: Dictionary = client.adventure_guide_state() if client != null else {}
	push_error("JOURNEYS_LAYOUT_FAIL: " + reason + " journal=" + str(state))
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
	var node := client.find_child(name, true, false) as TextureRect
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
	shots = OS.get_environment("ADVENTURE_GUIDE_SHOTS")
	if server != "127.0.0.1:5771" or not account.begins_with("fb_") or shots.is_empty():
		stop("Explicit private5771/fb_* account and screenshot directory required")
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
	if not await wait_screen("CharacterSelect") or not await click("EnterWorld"):
		return
	if not await wait_screen("InWorld"):
		return
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = KEY_J
		event.physical_keycode = KEY_J
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	deadline = Time.get_ticks_msec() + 90000
	while (not client.adventure_guide_state().loaded or client.adventure_guide_state().renown_faction_count < 0) and Time.get_ticks_msec() < deadline:
		await process_frame
	if client.adventure_guide_state().renown_faction_count != 29:
		stop("Pinned realm renown catalog did not arrive")
		return
	if not await click("EncounterJournalJourneysTab") or not await shot("journeys-midnight"):
		return
	if not await click("JournalJourneysExpansion") or not await shot("journeys-expansion-menu"):
		return
	if not await click("JournalJourneysExpansion9") or not await shot("journeys-dragonflight"):
		return
	if not await click("JourneyCard12") or not await click("JourneyRenownLevel2"):
		return
	if client.adventure_guide_state().journey != 12 or client.adventure_guide_state().journey_level != 2:
		stop("Physical faction/level selection did not resolve Maruuk2")
		return
	if not masked_icon("JourneyRenownLevel2*Icon", false) or not masked_icon("JourneyReward622Icon", true):
		return
	var notice := control("JourneyProgressUnlock")
	var pane := control("JourneyProgress")
	if notice == null or not notice.is_visible_in_tree() or not pane.get_global_rect().encloses(notice.get_global_rect()):
		stop("Unevaluated unlock notice is outside its clipped progress pane")
		return
	if not await move_to("JourneyProgressUnlock"):
		return
	var unlock_tooltip: Dictionary = client.tooltip_state()
	if not unlock_tooltip.visible or not unlock_tooltip.title.contains("not evaluated") or not str(unlock_tooltip.lines).contains("Maruukai"):
		stop("Authored unevaluated unlock tooltip absent: " + str(unlock_tooltip))
		return
	if not await shot("journeys-unlock-tooltip"):
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
	if not await click("EncounterJournalBack"):
		return
	var card := control("JourneyCard12")
	var scale := control("EncounterJournal").get_global_rect().size / Vector2(800,496)
	if card == null or card.get_global_rect().size.distance_to(Vector2(374,112)*scale)>1.5:
		stop("Shared Home did not return to source-sized Maruuk card")
		return
	print("JOURNEYS_LAYOUT_PASS skin=%s wheel3/jump1/25/tooltip/Home" % client.account_state().ui_skin)
	quit(0)
