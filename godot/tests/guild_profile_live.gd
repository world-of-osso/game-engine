extends "res://tests/bank_live.gd"

## Private guild master fixture. BANK_ACCOUNT/BANK_CHARACTER, GODOT_TEST_SERVER,
## BANK_SHOTS; uses real typed input and the authoritative profile reply.
const MOTD := "Guilds3 live MOTD"

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	shots = OS.get_environment("BANK_SHOTS")
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(OS.get_environment("GODOT_TEST_SERVER"), OS.get_environment("BANK_ACCOUNT"), PASSWORD, false)
	if error != "":
		fail(error)
		return
	if not await enter_world(OS.get_environment("BANK_CHARACTER")):
		return
	if shown("GuildMicroButton"):
		await press("GuildMicroButton")
	else:
		await press("MinimapLauncherButton")
		await press("LauncherEntryGuildMicroButton")
	if not await wait_until(func(): return shown("GuildInfoTab"), "Communities"):
		return
	await press("GuildInfoTab")
	if not await wait_until(func(): return shown("GuildEditMOTDButton"), "MOTD permission"):
		return
	await press("GuildEditMOTDButton")
	await press("GuildProfileEditText")
	var editor := control("GuildProfileEditText") as LineEdit
	editor.select_all()
	await tap(KEY_BACKSPACE)
	await type_text(MOTD)
	await wait_frames(30)
	if editor.text != MOTD or editor.caret_column != MOTD.length():
		fail("Typed MOTD/caret: %s / %s" % [editor.text, editor.caret_column])
		return
	# Moving left must survive another native projection update, too.
	await tap(KEY_LEFT)
	await type_text("!")
	var expected := MOTD.left(MOTD.length() - 1) + "!" + MOTD.right(1)
	if editor.text != expected:
		fail("Middle insertion: " + editor.text)
		return
	await press("GuildProfileAcceptButton")
	if not await wait_until(func(): return text("GuildMOTDText") == expected, "authoritative MOTD"):
		return
	print("FIXTURE GUILD_PROFILE_LIVE_DONE typed=", expected)
	client.free()
	quit(0)
