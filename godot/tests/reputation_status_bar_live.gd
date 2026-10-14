extends SceneTree
# Offline-seeded private persisted Stormwind watch -> login -> native HUD, no Journeys query.
var client: Node
var shots: String

func _initialize() -> void:
	call_deferred("run")

func fail(reason: String) -> void:
	push_error("REPUTATION_BAR_FAIL " + reason)
	quit(1)

func settle() -> void:
	for _frame in range(8):
		await process_frame

func control(name: String) -> Control:
	return client.find_child(name,true,false) as Control

func move(point: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion,true)
	await settle()

func click(name: String) -> bool:
	var node := control(name)
	if node == null or not node.is_visible_in_tree():
		fail("Missing visible control " + name)
		return false
	var point := node.get_global_rect().get_center()
	await move(point)
	for pressed in [true,false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = pressed
		root.push_input(event,true)
		await process_frame
	await settle()
	return true

func wait_screen(screen: String) -> bool:
	var deadline := Time.get_ticks_msec() + 180000
	while Time.get_ticks_msec() < deadline:
		if client.account_state().screen == screen:
			return true
		await process_frame
	fail("Screen not ready " + screen)
	return false

func capture(label: String) -> bool:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	if image.get_size() != Vector2i(1920,1080):
		fail("Original1920x1080 required")
		return false
	var path := shots.path_join("%s-reputation-%s.png" % [client.account_state().ui_skin.to_lower(),label])
	if image.save_png(path) != OK:
		fail("Cannot save " + path)
		return false
	print("REPUTATION_BAR_SHOT " + path)
	return true

func run() -> void:
	var server := OS.get_environment("ADVENTURE_GUIDE_SERVER")
	var account := OS.get_environment("ADVENTURE_GUIDE_ACCOUNT")
	shots = OS.get_environment("ADVENTURE_GUIDE_SHOTS")
	if server != "127.0.0.1:5771" or not account.begins_with("fb_") or shots.is_empty():
		fail("Explicit private5771/fb_* account and screenshot directory required")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var deadline := Time.get_ticks_msec() + 180000
	while client.account_state().assets_starting and Time.get_ticks_msec() < deadline:
		await process_frame
	var error: String = client.connect_account(server,account,OS.get_environment("ADVENTURE_GUIDE_PASSWORD"),false)
	if not error.is_empty():
		fail(error)
		return
	if not await wait_screen("CharacterSelect") or not await click("EnterWorld") or not await wait_screen("InWorld"):
		return
	deadline = Time.get_ticks_msec() + 90000
	while control("ReputationStatusBar") == null and Time.get_ticks_msec() < deadline:
		await process_frame
	var bar := control("ReputationStatusBar")
	var background := control("ReputationStatusBarBackground")
	var fill := control("ReputationStatusBarFill")
	if bar == null or background == null or fill == null or not bar.is_visible_in_tree():
		fail("Watched reputation missing90s after login; no Guide/retry")
		return
	var journal: Dictionary = client.adventure_guide_state()
	if journal.visible or journal.renown_faction_count != -1:
		fail("Fixture must prove bar without fetching/opening Journeys")
		return
	var scale := bar.get_global_rect().size.x / 571.
	var ratio := fill.get_global_rect().size.x / background.get_global_rect().size.x
	if absf(ratio - 800./6000.) > 0.001 or absf(bar.get_global_rect().size.y - 17.*scale) > 0.5:
		fail("Source geometry/progress mismatch ratio=%s rect=%s" % [ratio,bar.get_global_rect()])
		return
	var xp := control("ExperienceBar")
	var has_xp := xp != null and xp.is_visible_in_tree()
	var offset := 17. if has_xp else 0.
	if absf(bar.get_global_rect().end.y - (1080. - offset*scale)) > 1.:
		fail("Reputation did not occupy source main/secondary container")
		return
	await move(Vector2(5,5))
	await RenderingServer.frame_post_draw
	var pixels := root.get_texture().get_image()
	var painted := pixels.get_pixelv(Vector2i(fill.get_global_rect().get_center()))
	var empty := pixels.get_pixelv(Vector2i(background.get_global_rect().get_center()))
	if painted.g <= painted.r + 0.03 or painted.to_html() == empty.to_html():
		await capture("paint-failure")
		print("REPUTATION_BAR_PAINT_DIAGNOSTIC fill_rect=%s background_rect=%s" % [fill.get_global_rect(),background.get_global_rect()])
		fail("Real green reputation fill not painted: fill=%s track=%s" % [painted,empty])
		return
	if not await capture("restored"):
		return
	await move(background.get_global_rect().get_center())
	var label := client.find_child("ReputationStatusBarText",true,false)
	if label == null or not label.is_visible_in_tree() or label.text != "Stormwind 800 / 6000":
		fail("Physical hover lacks authoritative normalized text")
		return
	if not await capture("hover"):
		return
	await move(Vector2(5,5))
	if label.is_visible_in_tree():
		fail("Text remained after pointer left the bar")
		return
	print("REPUTATION_BAR_PASS skin=%s persistedStormwind72/800of6000/hover-leave/noJourneysQuery xp=%s" % [client.account_state().ui_skin,has_xp])
	quit(0)
