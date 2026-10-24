extends "res://tests/guild_member_notes_live.gd"

## Two actual native clients, one guild. The coordinator supplies actual accounts,
## an Officer peer and a Guild Master. No supplied chat/history/rank UI payloads.
var actor := ""
var skin := ""
var trial := ""
var directory := ""

func run_test() -> void:
	var success := await run_chat()
	if success:
		print("PASS GUILD_CHAT_NATIVE actor=", actor, " skin=", skin)
		completion.case_done()
		client.free()
		quit(0)

func run_chat() -> bool:
	root.size = Vector2i(1920, 1080)
	Engine.max_fps = 30
	actor = OS.get_environment("GUILD_CHAT_ACTOR")
	skin = OS.get_environment("GUILD_CHAT_SKIN")
	trial = OS.get_environment("GUILD_CHAT_TRIAL")
	directory = OS.get_environment("GUILD_CHAT_SYNC")
	shots = OS.get_environment("BANK_SHOTS")
	role = skin.to_lower() + "-" + actor
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await wait_frames(30)
	var error: String = client.connect_account(OS.get_environment("GODOT_TEST_SERVER"), OS.get_environment("BANK_ACCOUNT"), PASSWORD, false)
	if error != "":
		return fail(error)
	if not await enter_world(OS.get_environment("BANK_CHARACTER")):
		return false
	if not await select_skin(skin):
		return false
	if shown("GuildMicroButton"):
		await press("GuildMicroButton")
	else:
		await press("MinimapLauncherButton")
		await press("LauncherEntryGuildMicroButton")
	if not await wait_until(func(): return shown("CommunitiesChatTab"), "actual guild"):
		return false
	await press("CommunitiesChatTab")
	if not await wait_until(func(): return shown("CommunitiesChatEditBox"), "guild composer"):
		return false
	mark("ready")
	if not await other("ready"):
		return false
	if not await guild_delivery():
		return false
	if not await officer_delivery():
		return false
	if not await privacy_denial():
		return false
	return await history_open()

func guild_delivery() -> bool:
	if actor == "leader":
		if not await compose(message("guild-leader")):
			return false
		if not await received(message("guild-peer")):
			return false
	else:
		if not await received(message("guild-leader")):
			return false
		if not await compose(message("guild-peer")):
			return false
	if not await capture_checked("guild-delivery"):
		return false
	mark("guild")
	return await other("guild")

func officer_delivery() -> bool:
	await press("CommunitiesStreamDropdown")
	if not await wait_until(func(): return shown("CommunitiesOfficerChatChoice"), "actual officer privilege"):
		return false
	await press("CommunitiesOfficerChatChoice")
	if actor == "leader":
		if not await compose(message("officer-leader")):
			return false
		if not await received(message("officer-peer")):
			return false
	else:
		if not await received(message("officer-leader")):
			return false
		if not await compose(message("officer-peer")):
			return false
	if not await capture_checked("officer-delivery"):
		return false
	mark("officer")
	return await other("officer")

func privacy_denial() -> bool:
	if actor == "leader":
		if not await main_chat("/gdemote " + OS.get_environment("GUILD_CHAT_PEER")):
			return false
		mark("demoted")
		return await other("denied")
	if not await other("demoted"):
		return false
	if not await wait_until(func(): return text("CommunitiesStreamDropdownText") == "Guild Chat", "authoritative officer access revoked"):
		return false
	if not await main_chat("/o " + message("forbidden-officer")):
		return false
	if not await wait_until(func(): return text("CommunitiesChatError") == "You don't have permission to do that.", "actual officer server refusal"):
		return false
	await press("CommunitiesStreamDropdown")
	if shown("CommunitiesOfficerChatChoice"):
		return fail("Officer stream remains available after demotion")
	await press("CommunitiesStreamDropdown")
	if not await capture_checked("officer-denial"):
		return false
	mark("denied")
	return true

func history_open() -> bool:
	if actor == "leader":
		await press("GuildRosterTab")
		mark("history-closed")
		if not await other("history-sent"):
			return false
		await press("CommunitiesChatTab")
		if not await received(message("history-peer")):
			return false
		if transcript().contains(message("forbidden-officer")):
			return fail("Refused officer line entered history")
		if not await capture_checked("history-on-open"):
			return false
		mark("history-read")
		return true
	if not await other("history-closed"):
		return false
	if not await compose(message("history-peer")):
		return false
	mark("history-sent")
	return await other("history-read")

func compose(value: String) -> bool:
	await press("CommunitiesChatEditBox")
	var edit := control("CommunitiesChatEditBox") as LineEdit
	edit.select_all()
	await type_text(value)
	await tap(KEY_ENTER)
	return await wait_until(func(): return edit.text.is_empty(), "actual composer send clears input")

func main_chat(value: String) -> bool:
	await tap(KEY_ESCAPE)
	await tap(KEY_ENTER)
	await type_text(value)
	await tap(KEY_ENTER)
	await wait_frames(15)
	return true

func message(kind: String) -> String:
	return "%s %s %s" % [trial, skin, kind]

func transcript() -> String:
	var values := PackedStringArray()
	for index in range(500):
		var value := text("CommunitiesChatMessage%d" % index)
		if not value.is_empty():
			values.append(value)
	return "\n".join(values)

func received(value: String) -> bool:
	return await wait_until(func(): return transcript().contains(value), "real received " + value)

func mark(stage: String) -> void:
	FileAccess.open(directory.path_join(role + "-" + stage), FileAccess.WRITE).store_string("actual-native\n")

func other(stage: String) -> bool:
	var peer := "peer" if actor == "leader" else "leader"
	var path := directory.path_join(skin.to_lower() + "-" + peer + "-" + stage)
	return await wait_until(func(): return FileAccess.file_exists(path), "other actual actor " + stage)
