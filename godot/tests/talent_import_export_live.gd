extends "res://tests/talent_loadouts_live.gd"

# Real native clipboard + Import dialog -> existing authoritative Create path.
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
	await click("TalentExportLoadout")
	var empty_code := DisplayServer.clipboard_get()
	if not empty_code.begins_with("C4DA"):
		abort("Expected real Arcane62 v2 clipboard export: " + empty_code)
		return
	await purchase_first_class_node()
	await menu()
	await click("TalentExportLoadout")
	var purchased_code := DisplayServer.clipboard_get()
	if purchased_code == empty_code:
		abort("Clipboard export ignored staged purchase")
		return
	await click("TalentUndo")
	await expect_rank("0/1")
	await open_import()
	await paste_code("bad!")
	await type_name("Imported " + skin)
	await click("TalentLoadoutSave")
	await control("TalentImportError")
	await capture("invalid-import")
	await click("TalentImportErrorOkay")
	await control("TalentLoadoutCodeInput")
	await paste_code(purchased_code)
	await capture("import-dialog")
	# Code field owns Enter (Retail ImportControl.OnEnterPressed).
	await key(KEY_ENTER)
	await expect_caption("Imported " + skin)
	await expect_rank("1/1")
	await capture("import-acknowledged")
	await menu()
	await click("TalentExportLoadout")
	if DisplayServer.clipboard_get() != purchased_code:
		abort("Authoritative imported ranks do not re-export identically")
		return
	await login(endpoint)
	await expect_caption("Imported " + skin)
	await expect_rank("1/1")
	await capture("import-relog")
	evidence.append({"empty_export":empty_code,"purchased_export":purchased_code,"authoritative_roundtrip":true})
	var file := FileAccess.open(directory.path_join(skin + "-import-export-proof.json"), FileAccess.WRITE)
	file.store_string(JSON.stringify(evidence, "\t"))
	file.close()
	print("TALENT_IMPORT_EXPORT_LIVE_PASS ", skin)
	quit(0)

func open_import() -> void:
	await menu()
	await click("TalentImportLoadout")
	await control("TalentLoadoutCodeInput")

func paste_code(text: String) -> void:
	var field := await control("TalentLoadoutCodeInput") as TextEdit
	field.grab_focus()
	field.select_all()
	DisplayServer.clipboard_set(text)
	var event := InputEventKey.new()
	event.pressed = true
	event.keycode = KEY_V
	event.physical_keycode = KEY_V
	event.ctrl_pressed = true
	root.push_input(event)
	await frames(2)
	event.pressed = false
	root.push_input(event)
	await frames(15)
	if field.text != text:
		abort("Native paste mismatch: " + field.text)
