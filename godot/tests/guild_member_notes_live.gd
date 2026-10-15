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

# Record the actual native input boundary; bank_live's older helper has no
# selection/readiness receipt and can time out after a no-op card click.
func enter_world(character: String) -> bool:
	var deadline := Time.get_ticks_msec() + 120000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		await RenderingServer.frame_post_draw
		var state: Dictionary = client.account_state()
		if state.reply_received and state.screen == "CharacterSelect" and state.character_count >= 1 and client.get_node_or_null("CharacterSelectUI") != null and not state.assets_starting and shown("CharCard_0") and shown("EnterWorld"):
			break
	var ui = client.get_node_or_null("CharacterSelectUI")
	if ui == null or not shown("CharCard_0") or not shown("EnterWorld"):
		await entry_diagnostic("not-interactive")
		return fail("Character select controls not interactive: " + str(client.account_state()))
	await entry_diagnostic("before-card")
	await click(ui.find_child("CharCard_0", true, false))
	await entry_diagnostic("after-card")
	if ui.find_child("CharSelectCharacterName", true, false).text != character:
		return fail("Card 0 is not " + character)
	await click(ui.find_child("EnterWorld", true, false))
	await entry_diagnostic("after-enter")
	deadline = Time.get_ticks_msec() + 180000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "InWorld" and state.local_player_position != null and state.terrain.pending_count == 0 and not state.terrain.parsed_tiles.is_empty():
			await wait_frames(120)
			return true
	return fail("Timed out entering the world: " + str(client.account_state()))

func entry_diagnostic(stage: String) -> void:
	await RenderingServer.frame_post_draw
	var receipt := {"stage": stage, "window": str(root.size), "viewport": str(root.get_visible_rect()), "state": client.account_state(), "hover": str(root.gui_get_hovered_control()), "pointer": str(root.get_mouse_position())}
	for name in ["CharCard_0", "EnterWorld"]:
		var node := control(name)
		if node != null:
			receipt[name] = {"class": node.get_class(), "mouse_filter": node.mouse_filter, "rect": str(node.get_global_rect()), "canvas_transform": str(node.get_global_transform_with_canvas()), "visible": node.is_visible_in_tree(), "disabled": node.disabled if node is BaseButton else false}
	var directory := OS.get_environment("GUILD_NOTE_DIAGNOSTICS")
	assert(not directory.is_empty(), "Owned diagnostic directory required")
	var prefix := directory.path_join(role + "-" + stage + "-" + str(Time.get_unix_time_from_system()))
	FileAccess.open(prefix + ".json", FileAccess.WRITE).store_string(JSON.stringify(receipt, "\t"))
	root.get_texture().get_image().save_png(prefix + ".png")
	print("GUILD_ENTRY_DIAGNOSTIC ", prefix, " ", JSON.stringify(receipt))

func capture(name: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	assert(image.get_size() == Vector2i(1920, 1080), "Full-HD guild acceptance requires actual1920x1080 pixels")
	assert(image.save_png("%s/%s-%s.png" % [shots, role, name]) == OK)

func write_note(officer: bool, value: String) -> bool:
	await press("GuildMemberDetailOfficerNote" if officer else "GuildMemberDetailNote")
	if not await wait_until(func(): return shown("GuildMemberNoteEditBox"), "note popup"):
		return false
	var editor := control("GuildMemberNoteEditBox") as LineEdit
	if not editor.has_focus():
		return fail("Mainline note popup did not focus its input on show")
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
	await press("MenuBtnOptions")
	await press("OptionsTabhud")
	await press("Choiceui_layout")
	await press("Choiceui_layout0Hit" if skin == "Modern" else "Choiceui_layout1Hit")
	await press("OptionsCloseButton")
	if shown("MenuBtnResume"):
		await press("MenuBtnResume")
	return await wait_until(func(): return str(client.account_state().ui_skin) == skin, "skin switch")

func press(name: String) -> void:
	if not await wait_until(func(): return shown(name), "visible control " + name):
		await entry_diagnostic("missing-" + name)
		return
	await click(control(name))
