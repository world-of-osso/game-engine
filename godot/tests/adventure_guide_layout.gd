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
	if not await click("EncounterJournalTierDropdown"): return
	if not await capture_layout("tier-menu"): return
	if not await click("EncounterJournalTierDropdown"): return
	if not await click("EncounterJournalSearchBox"): return
	for index in range(50):
		var typed := InputEventKey.new()
		typed.keycode = KEY_A
		typed.unicode = 97
		typed.pressed = true
		root.push_input(typed,true)
		await process_frame
	await settle()
	var edit := client.find_child("EncounterJournalSearchBox",true,false) as LineEdit
	if edit.text.length()!=40:
		stop("Search limit must be40, got%s" % edit.text.length())
		return
	if not await capture_layout("search-filled"): return
	if not await click("EncounterJournalSearchBoxClearButton"): return
	if not edit.text.is_empty() or edit.has_focus():
		stop("Clear must empty search and release focus: text=%s focus=%s view=%s" % [edit.text, edit.has_focus(), str(client.adventure_guide_state())])
		return
	print("GUIDE_LAYOUT_SEARCH limit40+physical-clear PASS")
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
	var current := (client.find_child("EncounterJournalInstanceLore",true,false) as Control).get_global_rect().position.y
	var stepper := client.find_child("EncounterJournalLoreScrollBack",true,false) as Control
	if absf(current-before)>1.0:
		stop("Lore back stepper before=%s current=%s old_node=%s stepper=%s" % [before,current,lore.get_global_rect(),stepper.get_global_rect()])
		return
	print("GUIDE_LAYOUT_LORE_SCROLL wheel+back-stepper PASS")
	if not await click("JournalBoss112"): return
	if not await capture_layout("overview"): return
	if not await click("EncounterJournalPage1"): return
	if not await capture_layout("abilities"): return
	if not await click("EncounterJournalPage2"): return
	if not await capture_layout("loot"): return
	if not await click("JournalDifficultyDropdown"): return
	if not await capture_layout("difficulty-menu"): return
	if not await click("JournalDifficultyDropdown"): return
	if not await click("JournalClassFilter"): return
	if not await capture_layout("class-spec-menu"): return
	if not await click("JournalClassMenu"): return
	if not await capture_layout("class-submenu"): return
	if not await click("JournalClass2"): return
	if client.adventure_guide_state().class!=2:
		stop("Combined class submenu did not select Paladin")
		return
	if not await click("JournalClassFilter"): return
	if not await click("JournalSpec70"): return
	if client.adventure_guide_state().spec!=70:
		stop("Combined specialization menu did not select Retribution")
		return
	if not await click("JournalSlotFilter"): return
	if not await capture_layout("slot-menu"): return
	if not await click("JournalSlot0"): return
	print("GUIDE_LAYOUT_LOOT combined-class2-spec70+slot-menu PASS")
	if not await click("EncounterJournalPage3"): return
	var deadline := Time.get_ticks_msec() + 90000
	while client.adventure_guide_state().model_display != 36475 and Time.get_ticks_msec() < deadline:
		await process_frame
	if client.adventure_guide_state().model_display != 36475:
		stop("Ozruk did not attach")
		return
	if not await painted_model(): return
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
func painted_model() -> bool:
	var view := client.find_child("CharacterModelView",true,false) as TextureRect
	if view==null or view.texture==null:
		stop("Model viewport missing")
		return false
	await RenderingServer.frame_post_draw
	var shown := root.get_texture().get_image()
	var rect := view.get_global_rect()
	view.visible=false
	await RenderingServer.frame_post_draw
	var hidden := root.get_texture().get_image()
	view.visible=true
	await RenderingServer.frame_post_draw
	var changed := 0
	for y in range(int(rect.position.y),int(rect.end.y),4):
		for x in range(int(rect.position.x),int(rect.end.x),4):
			var a := shown.get_pixel(x,y)
			var b := hidden.get_pixel(x,y)
			if absf(a.r-b.r)+absf(a.g-b.g)+absf(a.b-b.b)>0.06: changed+=1
	if changed<100:
		stop("Model must be painted above backdrop, changed samples=%s" % changed)
		return false
	print("GUIDE_LAYOUT_MODEL_PAINTED changed_samples=%s PASS" % changed)
	return true
func clipped_pane(name: String, child_name: String) -> bool:
	var pane := client.find_child(name,true,false) as Control
	var art := client.find_child(child_name,true,false) as Control
	if pane==null or art==null or not pane.clip_contents or art.get_parent()!=pane:
		stop("XML pane must parent and clip its art: "+name)
		return false
	print("GUIDE_LAYOUT_CLIPPING "+name+" PASS")
	return true
func capture_layout(label: String) -> bool:
	for frame in range(30): await process_frame
	if OS.get_environment("GUIDE_LAYOUT_PHASE")=="after":
		if label=="grid":
			if client.find_child("JournalOwnedKeystones",true,false)!=null:
				stop("Unsourced Owned Keystones control")
				return false
			if not geometry("EncounterJournalContent",Rect2(18,112,748,367)): return false
			if not clipped_pane("EncounterJournalInstanceSelect","EncounterJournalInstanceSelectBackground"): return false
		if label=="overview" or label=="abilities":
			if client.find_child("JournalRole0",true,false)!=null:
				stop("Unsourced role row")
				return false
		if label=="loot":
			if not geometry("JournalClassFilter",Rect2(444,77,120,25)): return false
			if not geometry("JournalSlotFilter",Rect2(574,77,90,25)): return false
			if not geometry("EncounterJournalContent",Rect2(422,105,345,382)): return false
		if label=="instance":
			if not clipped_pane("EncounterJournalInstanceFrame","EncounterJournalInstanceLoreArt"): return false
			if not geometry("EncounterJournalInfo",Rect2(7,64,785,425)): return false
			if not geometry("EncounterJournalBosses",Rect2(32,106,338,382)): return false
		if label=="model":
			if not clipped_pane("EncounterJournalModelScene","EncounterJournalInstanceBackground"): return false
			if not geometry("EncounterJournalModelScene",Rect2(399,65,390,423)): return false
			if not geometry("EncounterJournalPage3",Rect2(780,264,63,57)): return false
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var path := shots.path_join("%s-%s-%s.png" % [OS.get_environment("GUIDE_LAYOUT_PHASE"), client.account_state().ui_skin.to_lower(), label])
	if OS.get_environment("GUIDE_LAYOUT_FULLHD")=="1" and image.get_size()!=Vector2i(1920,1080):
		stop("Required1920x1080 capture; got%s" % image.get_size())
		return false
	if image.save_png(path) != OK:
		stop("Cannot save " + path)
		return false
	print("GUIDE_LAYOUT_SHOT " + path)
	return true
