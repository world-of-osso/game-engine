extends "res://tests/bank_live.gd"

## Fresh private realm only. Account/character via BANK_*; GUILD_NOTE_TARGET is a
## real guild member. GUILD_NOTE_MODE=master|self|readback, GUILD_NOTE_SKIN=Modern|Forever.
## Master writes both actual notes; self writes own public note without elevated rights;
## readback after server restart checks persisted public/officer data without a write.
func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	Engine.max_fps = 30
	shots = OS.get_environment("BANK_SHOTS")
	role = OS.get_environment("GUILD_NOTE_SKIN").to_lower() + "-" + OS.get_environment("GUILD_NOTE_MODE")
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await wait_frames(30)
	var error: String = client.connect_account(OS.get_environment("GODOT_TEST_SERVER"), OS.get_environment("BANK_ACCOUNT"), PASSWORD, false)
	if error != "":
		fail(error)
		return
	if not await enter_world(OS.get_environment("BANK_CHARACTER")):
		return
	if not await select_skin(OS.get_environment("GUILD_NOTE_SKIN")):
		return
	if shown("GuildMicroButton"):
		await press("GuildMicroButton")
	else:
		await press("MinimapLauncherButton")
		await press("LauncherEntryGuildMicroButton")
	if not await wait_until(func(): return shown("GuildMember0"), "authoritative guild roster"):
		return
	var row := -1
	for index in range(50):
		if text("GuildMember%dName" % index) == OS.get_environment("GUILD_NOTE_TARGET"):
			row = index
			break
	if row < 0:
		fail("Real target missing from authoritative roster")
		return
	await press("GuildMember%d" % row)
	if not await wait_until(func(): return shown("GuildMemberDetailName"), "left-click member detail"):
		return
	if text("GuildMemberDetailName") != OS.get_environment("GUILD_NOTE_TARGET"):
		fail("Wrong roster target")
		return
	var mode := OS.get_environment("GUILD_NOTE_MODE")
	var skin := OS.get_environment("GUILD_NOTE_SKIN")
	if mode == "master":
		if not await write_note(false, skin + " alchemy") or not await write_note(true, skin + " raid healer"):
			return
	elif mode == "self":
		if shown("GuildMemberDetailOfficerNoteText"):
			fail("Private officer note exposed to ordinary member")
			return
		if not await write_note(false, skin + " cooking"):
			return
	elif mode == "readback":
		if text("GuildMemberDetailNoteText") != skin + " cooking" or text("GuildMemberDetailOfficerNoteText") != skin + " raid healer":
			fail("Restart/relog did not preserve both distinct notes")
			return
	else:
		fail("Unknown note proof mode")
		return
	await capture("member-detail")
	print("PASS GUILD_MEMBER_NOTES_LIVE mode=", mode, " skin=", client.account_state().ui_skin, " public=", text("GuildMemberDetailNoteText"), " officer=", text("GuildMemberDetailOfficerNoteText"))
	client.free()
	quit(0)

func capture(name: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	assert(image.get_size() == Vector2i(1920, 1080), "Full-HD guild acceptance requires actual1920x1080 pixels")
	assert(image.save_png("%s/%s-%s.png" % [shots, role, name]) == OK)

func write_note(officer: bool, value: String) -> bool:
	await press("GuildMemberDetailOfficerNote" if officer else "GuildMemberDetailNote")
	if not await wait_until(func(): return shown("GuildMemberNoteEditBox"), "note popup"):
		return false
	await press("GuildMemberNoteEditBox")
	var editor := control("GuildMemberNoteEditBox") as LineEdit
	editor.select_all()
	await tap(KEY_BACKSPACE)
	await type_text(value)
	if editor.text != value or editor.caret_column != value.length():
		return fail("Physical note input/caret mismatch")
	await capture("officer-editor" if officer else "public-editor")
	await tap(KEY_ENTER)
	var target := "GuildMemberDetailOfficerNoteText" if officer else "GuildMemberDetailNoteText"
	var note_button := "GuildMemberDetailOfficerNote" if officer else "GuildMemberDetailNote"
	return await wait_until(func(): return text(target) == value and not shown("GuildMemberNoteEditBox") and shown(note_button) and not (control(note_button) as BaseButton).disabled, "authoritative note reply and edit re-enable")

func select_skin(skin: String) -> bool:
	if str(client.account_state().ui_skin) == skin:
		return true
	await press("MinimapLauncherButton")
	await press("LauncherEntryMainMenuMicroButton")
	if not await press_label("Options"):
		return false
	await press("OptionsTabhud")
	await press("Choiceui_layout0" if skin == "Modern" else "Choiceui_layout1")
	if not await press_label("Done"):
		return false
	return await wait_until(func(): return str(client.account_state().ui_skin) == skin, "skin switch")

func press_label(value: String) -> bool:
	for node in client.find_children("*", "Control", true, false):
		if node.is_visible_in_tree() and (node is Label or node is Button) and node.text == value:
			await click_at(node.get_global_rect().get_center(), MOUSE_BUTTON_LEFT)
			return true
	return fail("Missing visible label " + value)
