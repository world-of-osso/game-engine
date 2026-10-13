extends SceneTree
# Native Retail frame geometry/input/capture fixture; explicit private realm and fb_* account.
var client: Node
var shots := ""
func _initialize() -> void:
	call_deferred("run")
func stop(reason: String) -> void:
	push_error("ADVENTURE_GUIDE_LAYOUT_FAIL: " + reason)
	quit(1)
func settle() -> void:
	for frame in range(6): await process_frame
func pointer(point: Vector2, button: int) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await process_frame
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = button
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await settle()
func click(name: String) -> bool:
	var control := client.find_child(name, true, false) as Control
	if control == null or not control.is_visible_in_tree():
		var bosses := []
		for node in client.find_children("JournalBoss*", "Control", true, false):
			if node.is_visible_in_tree(): bosses.append(str(node.name))
		stop("Missing control %s state=%s bosses=%s" % [name, str(client.adventure_guide_state()), str(bosses)])
		return false
	await pointer(control.get_global_rect().get_center(), MOUSE_BUTTON_LEFT)
	return true
func key_j() -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = KEY_J
		event.physical_keycode = KEY_J
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await settle()
func wait_screen(screen: String) -> bool:
	var deadline := Time.get_ticks_msec() + 180000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if client.account_state().screen == screen: return true
	stop("Screen not ready: " + screen + " " + str(client.account_state()))
	return false
func run() -> void:
	var server := OS.get_environment("ADVENTURE_GUIDE_SERVER")
	var account := OS.get_environment("ADVENTURE_GUIDE_ACCOUNT")
	shots = OS.get_environment("GUIDE_LAYOUT_SHOTS")
	if not shots.begins_with("/"):
		stop("Explicit absolute GUIDE_LAYOUT_SHOTS directory required")
		return
	if not server.begins_with("127.0.0.1:") or server.ends_with(":5000") or not account.begins_with("fb_"):
		stop("Explicit private endpoint and fb_* account required")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var startup := Time.get_ticks_msec() + 180000
	while client.account_state().assets_starting and Time.get_ticks_msec() < startup:
		await process_frame
	await settle()
	var error: String = client.connect_account(server, account, OS.get_environment("ADVENTURE_GUIDE_PASSWORD"), false)
	if not error.is_empty():
		stop(error)
		return
	if not await wait_screen("CharacterSelect"): return
	if not await click("EnterWorld"): return
	if not await wait_screen("InWorld"): return
	await key_j()
	var open := Time.get_ticks_msec() + 90000
	while not client.adventure_guide_state().loaded and Time.get_ticks_msec() < open:
		await process_frame
	if not client.adventure_guide_state().visible:
		await key_j()
	if not client.adventure_guide_state().loaded or not client.adventure_guide_state().visible:
		stop("Guide did not open")
		return
	if not await capture_layout("grid"): return
	if not await click("JournalInstance67"): return
	if not await capture_layout("instance"): return
	var lore := client.find_child("EncounterJournalInstanceLore",true,false) as Control
	var box := client.find_child("EncounterJournalLore",true,false) as Control
	var before := lore.get_global_rect().position.y
	await pointer(box.get_global_rect().get_center(),MOUSE_BUTTON_WHEEL_DOWN)
	if lore.get_global_rect().position.y >= before:
		stop("Lore wheel did not move wrapped text")
		return
	if not await click("EncounterJournalLoreScrollBack"): return
	if absf(lore.get_global_rect().position.y-before)>1.0:
		stop("Lore back stepper did not return to start")
		return
	print("GUIDE_LAYOUT_LORE_SCROLL wheel+back-stepper PASS")
	if not await click("JournalBoss112"): return
	if not await capture_layout("overview"): return
	if not await click("EncounterJournalPage1"): return
	if not await capture_layout("abilities"): return
	if not await click("EncounterJournalPage2"): return
	if not await capture_layout("loot"): return
	if not await click("EncounterJournalPage3"): return
	var deadline := Time.get_ticks_msec() + 90000
	while client.adventure_guide_state().model_display != 36475 and Time.get_ticks_msec() < deadline:
		await process_frame
	if client.adventure_guide_state().model_display != 36475:
		stop("Ozruk did not attach")
		return
	if not await capture_layout("model"): return
	print("GUIDE_LAYOUT_PASS skin=%s" % client.account_state().ui_skin)
	quit(0)
func geometry(name: String, expected: Rect2) -> bool:
	var frame := client.find_child("EncounterJournal",true,false) as Control
	var control := client.find_child(name,true,false) as Control
	if frame == null or control == null:
		stop("Geometry control missing " + name)
		return false
	var shell := frame.get_global_rect()
	var scale := shell.size / Vector2(800,496)
	var actual := control.get_global_rect()
	var wanted := Rect2(shell.position+expected.position*scale,expected.size*scale)
	if actual.position.distance_to(wanted.position)>1.5 or actual.size.distance_to(wanted.size)>1.5:
		stop("Geometry %s actual=%s expected=%s scale=%s" % [name,actual,wanted,scale])
		return false
	print("GUIDE_LAYOUT_GEOMETRY %s rect=%s scale=%s" % [name,actual,scale])
	return true
func capture_layout(label: String) -> bool:
	for frame in range(30): await process_frame
	if OS.get_environment("GUIDE_LAYOUT_PHASE")=="after":
		if label=="grid" and not geometry("EncounterJournalContent",Rect2(18,112,748,367)): return false
		if label=="instance":
			if not geometry("EncounterJournalInfo",Rect2(7,64,785,425)): return false
			if not geometry("EncounterJournalBosses",Rect2(32,106,338,382)): return false
		if label=="model":
			if not geometry("EncounterJournalModelScene",Rect2(399,65,390,423)): return false
			if not geometry("EncounterJournalPage3",Rect2(780,264,63,57)): return false
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var path := shots.path_join("%s-%s-%s.png" % [OS.get_environment("GUIDE_LAYOUT_PHASE"), client.account_state().ui_skin.to_lower(), label])
	if image.save_png(path) != OK:
		stop("Cannot save " + path)
		return false
	print("GUIDE_LAYOUT_SHOT " + path)
	return true
