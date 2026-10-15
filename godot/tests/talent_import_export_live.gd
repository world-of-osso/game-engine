extends "res://tests/talent_loadouts_live.gd"

# Clipboard OS delivery requires a Wayland seat serial unavailable in headless
# Cage. account_state records exactly the string handed to clipboard_set; typed
# native editbox input proves the remainder without claiming OS paste delivery.
func _run() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	directory = OS.get_environment("TALENT_LOADOUT_CAPTURE_PATH")
	skin = OS.get_environment("TALENT_LOADOUT_SKIN")
	var endpoint := OS.get_environment("TALENT_LOADOUT_ENDPOINT")
	if directory.is_empty() or endpoint.is_empty() or endpoint.ends_with(":5000"):
		push_error("Explicit private endpoint/captures required")
		quit(1)
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await frames(120)
	await login(endpoint)
	if OS.get_environment("TALENT_LOADOUT_PHASE") == "error-only":
		await menu()
		await capture("current-menu")
		await click("TalentImportLoadout")
		var preview_code := FileAccess.get_file_as_string("res://core/tests/fixtures/talent_mage_full.txt").strip_edges()
		await type_code(preview_code)
		await type_name("Preview")
		await capture("current-import-dialog")
		await type_code("bad!")
		await click("TalentLoadoutSave")
		await control("TalentImportError")
		if root.find_child("TalentError", true, false) != null:
			abort("Import modal duplicated its error in the commit footer")
			return
		await capture("current-invalid-import")
		var receipt := FileAccess.open(directory.path_join(skin + "-error-only-proof.json"), FileAccess.WRITE)
		receipt.store_string(JSON.stringify(evidence, "\t"))
		receipt.close()
		print("TALENT_IMPORT_ERROR_LIVE_PASS ", skin)
		quit(0)
		return
	await expect_caption("Default Loadout")
	await menu()
	await capture("import-export-menu")
	await click("TalentImportLoadout")
	await type_code("bad!")
	await type_name("Seed " + skin)
	await click("TalentLoadoutSave")
	await control("TalentImportError")
	await capture("invalid-import")
	await click("TalentImportErrorOkay")
	var full_code := FileAccess.get_file_as_string("res://core/tests/fixtures/talent_mage_full.txt").strip_edges()
	await type_code(full_code)
	await capture("import-dialog")
	await key(KEY_ENTER)
	await expect_caption("Seed " + skin)
	await expect_rank("1/1")
	var committed_ranks := rank_state()
	if committed_ranks.size() < 60:
		abort("Expected a full real class/spec/hero rank projection")
		return
	await menu()
	await capture("full-committed-export")
	await click("TalentExportLoadout")
	var exported: String = client.account_state().talent_export_clipboard
	if exported != full_code:
		abort("Native clipboard_set handoff differs: " + exported)
		return
	print("TALENT_EXPORT_CLIPBOARD_SET ", exported)
	await key(KEY_N)
	await capture("export-feedback")
	var feedback := false
	for node in root.find_children("ChatFrame1MessagesRow*Run*", "Label", true, false):
		if "Loadout code copied to clipboard." in node.text:
			feedback = true
	if not feedback:
		abort("Retail export system feedback missing")
		return
	await key(KEY_N)
	await control("TalentLoadoutDropDown")
	await open_import()
	await type_code(exported)
	await type_name("Imported " + skin)
	await capture("exported-import-dialog")
	await click("TalentLoadoutSave")
	await expect_caption("Imported " + skin)
	await expect_rank("1/1")
	await capture("import-acknowledged")
	await login(endpoint)
	await expect_caption("Imported " + skin)
	await expect_rank("1/1")
	if rank_state() != committed_ranks:
		abort("Authoritative relog changed the committed rank/entry projection")
		return
	await capture("import-relog")
	evidence.append({"clipboard_set":exported,"retail_feedback":feedback,"authoritative_roundtrip":true,"os_clipboard_delivery":"environment gap: no Cage seat serial"})
	var file := FileAccess.open(directory.path_join(skin + "-import-export-proof.json"), FileAccess.WRITE)
	file.store_string(JSON.stringify(evidence, "\t"))
	file.close()
	print("TALENT_IMPORT_EXPORT_LIVE_PASS ", skin)
	quit(0)

func capture(suffix: String) -> void:
	await frames(30)
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	if image.get_size() != Vector2i(1920, 1080):
		abort("Native1920x1080 capture required, got " + str(image.get_size()))
		return
	var filename := skin + "-" + suffix + ".png"
	if image.save_png(directory.path_join(filename)) != OK:
		abort("PNG save failed: " + filename)
		return
	evidence.append({"png":filename,"size":[image.get_width(),image.get_height()]})

func rank_state() -> Dictionary:
	var result := {}
	for node in root.find_children("TalentNode*Ranks", "Label", true, false):
		if node.is_visible_in_tree():
			result[str(node.name)] = node.text
	for node in root.find_children("TalentNode*Entry*Spell*Button", "Button", true, false):
		if node.is_visible_in_tree():
			result[str(node.name)] = true
	return result

func login(endpoint: String) -> void:
	var error: String = client.connect_account(endpoint, OS.get_environment("TALENT_LOADOUT_ACCOUNT"), OS.get_environment("TALENT_LOADOUT_PASSWORD"), false)
	if not error.is_empty():
		abort(error)
		return
	await wait_screen("CharacterSelect")
	await click("EnterWorld")
	await wait_screen("InWorld")
	await create_timer(5.0).timeout
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	await frames(3)
	await key(KEY_N)
	await control("TalentLoadoutDropDown")
	await frames(30)

func wait_screen(screen: String) -> void:
	var deadline := Time.get_ticks_msec() + 180000
	while Time.get_ticks_msec() < deadline:
		if client.account_state().screen == screen:
			evidence.append({"screen":screen})
			return
		await process_frame
	abort("Screen timeout: " + screen + " " + str(client.account_state()))

func control(name_text: String) -> Control:
	var deadline := Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		var node := root.find_child(name_text, true, false) as Control
		if node != null and node.is_visible_in_tree():
			return node
		await process_frame
	abort("Native control timeout: " + name_text)
	return null

func open_import() -> void:
	await menu()
	await click("TalentImportLoadout")
	await control("TalentLoadoutCodeInput")

func type_code(text: String) -> void:
	var field := await control("TalentLoadoutCodeInput") as TextEdit
	field.grab_focus()
	field.select_all()
	for character in text:
		var event := InputEventKey.new()
		event.pressed = true
		event.unicode = character.unicode_at(0)
		root.push_input(event)
		await frames(1)
	await frames(15)
	if field.text != text:
		abort("Native text input mismatch: " + field.text)
