extends "res://tests/guild_member_notes_live.gd"

## Two real native clients. No supplied guild state; only native controls and typed chat.
var actor := ""
var skin := ""
var directory := ""
var chat_marker := ""

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
	chat_marker = "Guilds2 disband " + OS.get_environment("GUILD_DISBAND_TRIAL")
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
	if not await open_guild():
		return false
	if OS.get_environment("GUILD_DISBAND_MODE") == "readback":
		if not await wait_until(func(): return received_not_in_guild() and shown("CommunitiesFrame") and text("CommunitiesGuildEntryName").is_empty() and not shown("GuildMember0"), "actual fresh-process NotInGuild reply and empty guild state"):
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
		if not await press_checked("CommunitiesChatTab") or not await wait_until(Callable(self, "history_received"), "actual Chat history receipt"):
			return false
		mark("chat-ready")
		if not await wait_until(func(): return chat_text().contains(chat_marker), "actual Guild line before destruction"):
			return false
		if not await capture_checked("chat-before-disband"):
			return false
		mark("ready")
	else:
		if not await other("chat-ready") or not await slash("/g " + chat_marker):
			return false
		if not await other("ready"):
			return false
		if not await press_checked("GuildDisbandButton") or not await wait_until(func(): return shown("GuildDisbandPopup"), "UI confirmation"):
			return false
		if text("GuildDisbandPopupText") != "Do you really want to disband your guild?":
			return fail("Actual confirmation differs from CONFIRM_GUILD_DISBAND")
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
		await tap(KEY_ESCAPE)
		if not await wait_until(func(): return not shown("CommunitiesFrame"), "closed Communities before slash"):
			return false
		if not await slash("/gdisband") or not await wait_until(func(): return shown("GuildDisbandPopup"), "standalone slash confirmation"):
			return false
		if shown("CommunitiesFrame"):
			return fail("Slash opened Communities instead of only the source popup")
		if not await capture_checked("standalone-confirmation"):
			return false
		await tap(KEY_ESCAPE)
		if not await wait_until(func(): return not shown("GuildDisbandPopup") and client.get_node_or_null("GameMenuUI") == null, "standalone Escape consumes only the popup"):
			return false
		if not await open_guild() or not await press_checked("GuildInfoTab"):
			return false
		if not await wait_until(func(): return shown("GuildDisbandButton") and not (control("GuildDisbandButton") as BaseButton).disabled, "fresh rank reply enables the destructive control"):
			return false
		if not await press_checked("GuildDisbandButton") or not await press_checked("GuildDisbandYes"):
			return false
	if not await wait_until(func(): return not shown("GuildMember0") and text("CommunitiesGuildEntryName").is_empty(), "authoritative removal on both clients"):
		return false
	if shown("GuildDisbandButton") or shown("GuildDisbandPopup"):
		return fail("Disband controls survived removal")
	if actor == "peer" and chat_text().contains(chat_marker):
		return fail("Communities retained the actual Guild line after membership removal")
	if not await capture_checked("disbanded"):
		return false
	mark("done")
	return await other("done")

func received_not_in_guild() -> bool:
	for index in range(1, 4):
		if text("UIErrorsFrameLine%d" % index) == "You are not in a guild.":
			return true
	return false

func history_received() -> bool:
	var receipt: Dictionary = JSON.parse_string(str(client.guild_chat_observation()))
	return int(receipt.history_count) > 0

func chat_text() -> String:
	var frame := control("CommunitiesChatMessageFrame")
	if frame == null or not frame.is_visible_in_tree():
		return ""
	var result := ""
	for node in frame.find_children("*", "Label", true, false):
		var label := node as Label
		if label.is_visible_in_tree():
			result += label.text + "\n"
	return result

func open_guild() -> bool:
	if shown("GuildMicroButton"):
		return await press_checked("GuildMicroButton")
	if not await press_checked("MinimapLauncherButton"):
		return false
	return await press_checked("LauncherEntryGuildMicroButton")

func slash(line: String) -> bool:
	if not line.begins_with("/"):
		return fail("Expected a slash command")
	# A focused selection tab owns Enter; the actual slash shortcut opens chat.
	print("GUILD_SLASH_KEY focus=", root.gui_get_focus_owner(), " command=", line)
	await type_text("/")
	if not await wait_until(func(): return shown("ChatFrame1EditBox"), "native main chat from slash shortcut"):
		return false
	var edit := control("ChatFrame1EditBox") as LineEdit
	if edit == null or not edit.has_focus():
		return fail("Main chat edit box does not own input")
	if edit.text != "/":
		return fail("Slash shortcut did not supply the source slash prefix")
	await type_text(line.substr(1))
	if edit.text != line:
		return fail("Typed slash differs from expected input")
	await tap(KEY_ENTER)
	return true

func mark(stage: String) -> void:
	FileAccess.open(directory.path_join(actor + "-" + stage), FileAccess.WRITE).store_string("actual native actor")

func other(stage: String) -> bool:
	var peer := "peer" if actor == "leader" else "leader"
	return await wait_until(func(): return FileAccess.file_exists(directory.path_join(peer + "-" + stage)), "other actual actor " + stage)
