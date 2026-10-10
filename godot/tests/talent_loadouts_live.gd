extends SceneTree

# Private real-server proof. Supply TALENT_LOADOUT_ENDPOINT, ACCOUNT, PASSWORD,
# CAPTURE_PATH and SKIN; never defaults to the shared realm.
var client: Node
var evidence: Array = []
var directory: String
var skin: String

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	directory = OS.get_environment("TALENT_LOADOUT_CAPTURE_PATH")
	skin = OS.get_environment("TALENT_LOADOUT_SKIN")
	var endpoint := OS.get_environment("TALENT_LOADOUT_ENDPOINT")
	if directory.is_empty() or endpoint.is_empty() or endpoint.ends_with(":5000"):
		abort("Requires an explicit private endpoint and capture directory")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await frames(120)
	await login(endpoint)
	if skin == "modern":
		await modern_lifecycle()
	elif skin == "forever":
		await forever_relog()
	else:
		abort("Unknown explicit skin")
		return
	var file := FileAccess.open(directory.path_join(skin + "-proof.json"), FileAccess.WRITE)
	file.store_string(JSON.stringify(evidence, "\t"))
	file.close()
	print("TALENT_LOADOUT_LIVE_PASS ", skin)
	quit(0)

func login(endpoint: String) -> void:
	var error: String = client.connect_account(endpoint, OS.get_environment("TALENT_LOADOUT_ACCOUNT"), OS.get_environment("TALENT_LOADOUT_PASSWORD"), false)
	if not error.is_empty():
		abort(error)
		return
	await wait_screen("CharacterSelect")
	await click("EnterWorld")
	await wait_screen("InWorld")
	# Session readiness precedes the independently loaded spell catalog. Give the
	# cold worker its observed startup interval before sending an opening edge.
	await frames(1800)
	# Mainline ToggleTalents default N (input_bindings_data.rs); independent of HUD visibility.
	await key(KEY_N)
	await control("TalentLoadoutDropDown")
	await frames(120)

func modern_lifecycle() -> void:
	if OS.get_environment("TALENT_LOADOUT_PHASE") == "rename":
		await expect_caption("Raid")
		await rename_delete_relog()
		return
	await expect_caption("Default Loadout")
	await new_loadout("Raid")
	await purchase_first_class_node()
	await new_loadout("Dungeons")
	await menu()
	await capture("named-rows")
	await click("TalentLoadoutRow1")
	await expect_caption("Raid")
	await expect_rank("0/1")
	await capture("raid-empty")
	await purchase_first_class_node()
	await click("TalentApply")
	await expect_rank("1/1")
	await frames(60)
	await rename_delete_relog()

func rename_delete_relog() -> void:
	await menu()
	await click("TalentLoadoutEdit1")
	await capture("edit-dialog")
	await type_name("Raid Updated")
	print("TALENT_INPUT_BEFORE_ENTER focus=",(root.find_child("TalentLoadoutNameInput",true,false) as LineEdit).has_focus())
	await key(KEY_ENTER)
	await expect_caption("Raid Updated")
	await new_loadout("Temporary")
	await menu()
	await click("TalentLoadoutEdit3")
	await click("TalentLoadoutDelete")
	await capture("delete-confirmation")
	await click("TalentLoadoutSave")
	await expect_caption("Default Loadout")
	await menu()
	if root.find_child("TalentLoadoutRow3", true, false) != null:
		abort("Deleted row remains")
		return
	await click("TalentLoadoutRow1")
	await expect_caption("Raid Updated")
	await login(OS.get_environment("TALENT_LOADOUT_ENDPOINT"))
	await expect_caption("Raid Updated")
	await expect_rank("1/1")
	await menu()
	await capture("relog-persisted")

func forever_relog() -> void:
	await expect_caption("Raid Updated")
	await menu()
	await capture("named-rows")
	await click("TalentLoadoutEdit1")
	await capture("edit-dialog")
	await key(KEY_ESCAPE)
	await frames(30)
	if root.find_child("TalentLoadoutDialog", true, false) != null:
		abort("Escape did not cancel the name dialog")
		return
	await menu()
	await click("TalentLoadoutRow2")
	await expect_caption("Dungeons")
	await login(OS.get_environment("TALENT_LOADOUT_ENDPOINT"))
	await expect_caption("Dungeons")
	await expect_rank("1/1")
	await menu()
	await capture("relog-persisted")

func new_loadout(name_text: String) -> void:
	await menu()
	await click("TalentNewLoadout")
	await control("TalentLoadoutNameInput")
	await frames(15)
	await type_name(name_text)
	await click("TalentLoadoutSave")
	await expect_caption(name_text)

func menu() -> void:
	await click("TalentLoadoutDropDown")
	await control("TalentLoadoutMenu")

func purchase_first_class_node() -> void:
	var candidates := root.find_children("TalentNode62122Entry80181Spell*Button", "Button", true, false)
	if candidates.size() != 1:
		abort("Expected one authentic Mage class node62122/entry80181")
		return
	await click_control(candidates[0])
	await expect_rank("1/1")

func type_name(text: String) -> void:
	var line := await control("TalentLoadoutNameInput") as LineEdit
	line.grab_focus()
	line.select_all()
	for character in text:
		var event := InputEventKey.new()
		event.pressed = true
		event.unicode = character.unicode_at(0)
		root.push_input(event)
		await frames(1)
	await frames(15)
	if line.text != text:
		abort("Native text mismatch: " + line.text)

func key(code: Key) -> void:
	var event := InputEventKey.new()
	event.pressed = true
	event.keycode = code
	event.physical_keycode = code
	root.push_input(event)
	await frames(2)
	event.pressed = false
	root.push_input(event)
	await frames(15)

func click(name_text: String) -> void:
	var node := await control(name_text)
	if node == null:
		return
	await click_control(node)

func click_control(node: Control) -> void:
	var event := InputEventMouseButton.new()
	event.position = node.get_global_rect().get_center()
	event.button_index = MOUSE_BUTTON_LEFT
	event.pressed = true
	root.push_input(event)
	await frames(2)
	event.pressed = false
	root.push_input(event)
	await frames(20)

func control(name_text: String) -> Control:
	for frame in range(3600):
		var node := root.find_child(name_text, true, false) as Control
		if node != null and node.is_visible_in_tree():
			return node
		await frames(1)
	abort("Native control timeout: " + name_text)
	return null

func expect_caption(text: String) -> void:
	for frame in range(1800):
		var label := root.find_child("TalentLoadoutName", true, false) as Label
		if label != null and label.text == text:
			evidence.append({"caption":text})
			print("LOADOUT_CAPTION ",text)
			return
		await frames(1)
	abort("Loadout acknowledgement timeout: " + text)

func expect_rank(text: String) -> void:
	for frame in range(1800):
		var label := root.find_child("TalentNode62122Ranks", true, false) as Label
		if label != null and label.text == text:
			evidence.append({"node":62122,"rank":text})
			return
		await frames(1)
	abort("Talent rank timeout: " + text)

func wait_screen(screen: String) -> void:
	for frame in range(10800):
		if client.account_state().screen == screen:
			evidence.append({"screen":screen})
			return
		await frames(1)
	abort("Screen timeout: " + screen + " " + str(client.account_state()))

func capture(suffix: String) -> void:
	await frames(30)
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var filename := skin + "-" + suffix + ".png"
	if image.save_png(directory.path_join(filename)) != OK:
		abort("PNG save failed: " + filename)
		return
	var geometry := {}
	for node in root.find_children("TalentLoadout*", "Control", true, false):
		if node.is_visible_in_tree():
			var rect: Rect2 = node.get_global_rect()
			geometry[node.name] = [rect.position.x, rect.position.y, rect.size.x, rect.size.y]
	evidence.append({"png":filename,"size":[image.get_width(),image.get_height()],"geometry":geometry})

func frames(count: int) -> void:
	for frame in range(count):
		await process_frame

func abort(message: String) -> void:
	root.print_tree_pretty()
	print("LOADOUT_FAILURE_STATE ",client.account_state())
	push_error("TALENT_LOADOUT_LIVE_FAIL " + message)
	quit(1)
