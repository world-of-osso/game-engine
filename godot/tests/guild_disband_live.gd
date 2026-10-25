extends "res://tests/guild_member_notes_live.gd"

## Two real native clients. No supplied guild state; only native controls and typed chat.
var actor := ""
var skin := ""
var directory := ""

func run_completion() -> void:
	var success := await run_disband()
	if success:
		print("PASS GUILD_DISBAND_NATIVE actor=", actor, " skin=", skin)
		completion.case_done()
		client.free()
	completion.finish(success)

func run_disband() -> bool:
	root.size = Vector2i(1920, 1080)
	Engine.max_fps = 30
	actor = OS.get_environment("GUILD_DISBAND_ACTOR")
	skin = OS.get_environment("GUILD_DISBAND_SKIN")
	directory = OS.get_environment("GUILD_DISBAND_SYNC")
	shots = OS.get_environment("BANK_SHOTS")
	role = skin.to_lower() + "-" + actor
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await wait_frames(30)
	var error: String = client.connect_account(OS.get_environment("GODOT_TEST_SERVER"), OS.get_environment("BANK_ACCOUNT"), PASSWORD, false)
	if error != "":
		return fail(error)
	if not await enter_world(OS.get_environment("BANK_CHARACTER")) or not await select_skin(skin):
		return false
	if shown("GuildMicroButton"):
		if not await press_checked("GuildMicroButton"):
			return false
	else:
		if not await press_checked("MinimapLauncherButton") or not await press_checked("LauncherEntryGuildMicroButton"):
			return false
	if OS.get_environment("GUILD_DISBAND_MODE") == "readback":
		if not await wait_until(func(): return shown("CommunitiesFrame") and text("CommunitiesGuildEntryName").is_empty() and not shown("GuildMember0"), "fresh-process unguilded state"):
			return false
		if shown("GuildDisbandButton") or shown("GuildDisbandPopup"):
			return fail("Fresh client retains guild authority after disband")
		return await capture_checked("restart-unguilded")
	if not await wait_until(func(): return shown("GuildMember1"), "two actual guild members"):
		return false
	if not await press_checked("GuildInfoTab"):
		return false
	if actor == "peer":
		if shown("GuildDisbandButton"):
			return fail("Ordinary member has a disband control")
		if not await slash("/gdisband"):
			return false
		await wait_frames(30)
		if shown("GuildDisbandPopup") or text("CommunitiesGuildEntryName").is_empty():
			return fail("Nonleader slash changed guild state or opened confirmation")
		if not await capture_checked("member-no-disband"):
			return false
		mark("ready")
	else:
		if not await other("ready"):
			return false
		if not await press_checked("GuildDisbandButton") or not await wait_until(func(): return shown("GuildDisbandPopup"), "UI confirmation"):
			return false
		if not await capture_checked("confirmation"):
			return false
		if not await press_checked("GuildDisbandNo") or not await wait_until(func(): return not shown("GuildDisbandPopup") and not text("CommunitiesGuildEntryName").is_empty(), "No leaves guild intact"):
			return false
		if not await slash("/gdisband") or not await wait_until(func(): return shown("GuildDisbandPopup"), "slash confirmation"):
			return false
		await tap(KEY_ESCAPE)
		if not await wait_until(func(): return not shown("GuildDisbandPopup") and not text("CommunitiesGuildEntryName").is_empty(), "Escape leaves guild intact"):
			return false
		if not await capture_checked("cancelled"):
			return false
		if not await press_checked("GuildDisbandButton") or not await press_checked("GuildDisbandYes"):
			return false
	if not await wait_until(func(): return not shown("GuildMember0") and text("CommunitiesGuildEntryName").is_empty(), "authoritative removal on both clients"):
		return false
	if shown("GuildDisbandButton") or shown("GuildDisbandPopup"):
		return fail("Disband controls survived removal")
	if not await capture_checked("disbanded"):
		return false
	mark("done")
	return await other("done")

func slash(line: String) -> bool:
	await tap(KEY_ENTER)
	if not await wait_until(func(): return shown("ChatFrame1EditBox"), "native main chat"):
		return false
	await type_text(line)
	if text("ChatFrame1EditBox") != line:
		return fail("Typed slash differs from expected input")
	await tap(KEY_ENTER)
	return true

func mark(stage: String) -> void:
	FileAccess.open(directory.path_join(actor + "-" + stage), FileAccess.WRITE).store_string("actual native actor")

func other(stage: String) -> bool:
	var peer := "peer" if actor == "leader" else "leader"
	return await wait_until(func(): return FileAccess.file_exists(directory.path_join(peer + "-" + stage)), "other actual actor " + stage)
