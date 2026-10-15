extends "res://tests/talent_loadouts_live.gd"

# Clipboard OS delivery requires a Wayland seat serial unavailable in headless
# Cage. account_state records exactly the string handed to clipboard_set; typed
# native editbox input proves the remainder without claiming OS paste delivery.
func _run() -> void:
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
	for node in root.find_children("*", "RichTextLabel", true, false):
		if "Loadout code copied to clipboard." in node.get_parsed_text():
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
	await capture("import-relog")
	evidence.append({"clipboard_set":exported,"retail_feedback":feedback,"authoritative_roundtrip":true,"os_clipboard_delivery":"environment gap: no Cage seat serial"})
	var file := FileAccess.open(directory.path_join(skin + "-import-export-proof.json"), FileAccess.WRITE)
	file.store_string(JSON.stringify(evidence, "\t"))
	file.close()
	print("TALENT_IMPORT_EXPORT_LIVE_PASS ", skin)
	quit(0)

func login(endpoint: String) -> void:
	var error: String = client.connect_account(endpoint, OS.get_environment("TALENT_LOADOUT_ACCOUNT"), OS.get_environment("TALENT_LOADOUT_PASSWORD"), false)
	if not error.is_empty():
		abort(error)
		return
	await wait_screen("CharacterSelect")
	await click("EnterWorld")
	await wait_screen("InWorld")
	await create_timer(5.0).timeout
	await key(KEY_N)
	await control("TalentLoadoutDropDown")
	await frames(30)

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
