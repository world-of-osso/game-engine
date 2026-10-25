extends "res://tests/guild_member_notes_live.gd"

## Two actual native clients, one guild. The coordinator supplies actual accounts,
## an Officer peer and a Guild Master. No supplied chat/history/rank UI payloads.
var actor := ""
var skin := ""
var trial := ""
var directory := ""

func run_completion() -> void:
	var success := await run_chat()
	if success:
		print("PASS GUILD_CHAT_NATIVE actor=", actor, " skin=", skin)
		completion.case_done()
		client.free()
	completion.finish(success)

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
	if not check_chat_geometry():
		return false
	if not await check_composer_escape():
		return false
	if not await wait_until(func(): return int(observation().history_count) > 0, "actual history receipt on open"):
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

func check_chat_geometry() -> bool:
	var frame := control("CommunitiesFrame").get_global_rect()
	var scale := frame.size.x / 814.0
	var expected := {
		"CommunitiesChatMessageFrame": Rect2(201, 67, 416, 303),
		"CommunitiesChatEditBox": Rect2(197, 374, 423, 32),
		"CommunitiesStreamDropdown": Rect2(195, 31, 160, 25),
		"CommunitiesChatMemberList": Rect2(649, 63, 139, 335),
	}
	for name in expected:
		var source: Rect2 = expected[name]
		var actual := control(name).get_global_rect()
		var scaled := Rect2(frame.position + source.position * scale, source.size * scale)
		if not actual.is_equal_approx(scaled):
			return fail("Actual native source rectangle %s: %s expected %s" % [name, actual, scaled])
	return true

func check_composer_escape() -> bool:
	if not await press_checked("CommunitiesChatEditBox"):
		return false
	if not completion.require(control("CommunitiesChatEditBox").has_focus(), "Actual composer focused before Escape"):
		return false
	await tap(KEY_ESCAPE)
	return completion.require(shown("CommunitiesChatEditBox") and not control("CommunitiesChatEditBox").has_focus(), "Focused composer Escape must clear focus without closing Communities")

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
		if not await other("denied"):
			return false
		if not await compose(message("officer-private-after-demotion")):
			return false
		if not await received(message("officer-private-after-demotion")):
			return false
		if not await capture_checked("officer-private-after-demotion"):
			return false
		mark("private-sent")
		return await other("privacy-checked")
	if not await other("demoted"):
		return false
	if not await wait_until(func(): return text("CommunitiesStreamDropdownText") == "Guild Chat", "authoritative officer access revoked"):
		return false
	if not await main_chat("/o " + message("forbidden-officer")):
		return false
	if not await wait_until(func(): return text("CommunitiesChatError") == "You don't have permission to do that.", "actual officer server refusal"):
		return false
	await press("CommunitiesStreamDropdown")
	if not await wait_until(func(): return shown("CommunitiesGuildChatChoice"), "actual stream menu after demotion"):
		return false
	if shown("CommunitiesOfficerChatChoice"):
		return fail("Officer stream remains available after demotion")
	await press("CommunitiesStreamDropdown")
	if not await capture_checked("officer-denial"):
		return false
	mark("denied")
	if not await other("private-sent"):
		return false
	var before := int(observation().history_count)
	await press("GuildRosterTab")
	await press("CommunitiesChatTab")
	if not await wait_until(func(): return int(observation().history_count) > before, "ordinary member actual history receipt"):
		return false
	if JSON.stringify(observation()).contains(message("officer-private-after-demotion")):
		return fail("Ordinary member received officer data in live delivery or history")
	if not await capture_checked("officer-private-history-refused"):
		return false
	mark("privacy-checked")
	return true

func history_open() -> bool:
	if actor == "leader":
		await press("GuildRosterTab")
		if not await wait_until(func(): return not shown("CommunitiesChatEditBox"), "Chat pane physically closed before history-only delivery"):
			return false
		mark("history-closed")
		if not await other("history-sent"):
			return false
		var before := int(observation().history_count)
		await press("CommunitiesChatTab")
		if not await wait_until(func(): return int(observation().history_count) > before, "new actual history reply after opening"):
			return false
		if not await received(message("history-peer")):
			return false
		if JSON.stringify(observation()).contains(message("forbidden-officer")):
			return fail("Refused officer line entered actual live delivery or history")
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

func press(name: String) -> void:
	if not await press_checked(name):
		completion.fail("Physical chat control failed: " + name)

func observation() -> Dictionary:
	return JSON.parse_string(str(client.guild_chat_observation()))

func capture_checked(name: String) -> bool:
	if not await super.capture_checked(name):
		return false
	var file := FileAccess.open(directory.path_join(role + "-" + name + "-received.json"), FileAccess.WRITE)
	if file == null:
		return fail("Cannot write actual chat receipt: " + str(FileAccess.get_open_error()))
	file.store_string(JSON.stringify(observation(), "\t"))
	return true

func compose(value: String) -> bool:
	await press("CommunitiesChatEditBox")
	var edit := control("CommunitiesChatEditBox") as LineEdit
	edit.select_all()
	await type_text(value)
	await tap(KEY_ENTER)
	return await wait_until(func(): return edit.text.is_empty(), "actual composer send clears input")

func main_chat(value: String) -> bool:
	if not await check_composer_escape():
		return false
	await tap(KEY_ENTER)
	if not await wait_until(func(): return shown("ChatFrame1EditBox"), "actual main-chat composer"):
		return false
	await type_text(value)
	await tap(KEY_ENTER)
	await wait_frames(15)
	return true

func message(kind: String) -> String:
	return "%s %s %s" % [trial, skin, kind]

func transcript() -> String:
	var values := PackedStringArray()
	for node in client.find_children("CommunitiesChatMessage*", "Label", true, false):
		if node.is_visible_in_tree():
			values.append(node.text)
	return "\n".join(values)

func chat_control_observations() -> Array:
	var controls := []
	for index in range(10):
		var node := control("CommunitiesChatMessage%d" % index)
		if node == null:
			continue
		var display := "not a text node"
		if node is Label:
			display = node.text
		elif node is RichTextLabel:
			display = node.get_parsed_text()
		controls.append({"name": node.name, "class": node.get_class(), "visible": node.is_visible_in_tree(), "text": display})
	return controls

func fail(message: String) -> bool:
	if client != null and not directory.is_empty():
		var file := FileAccess.open(directory.path_join(role + "-failure-observation.json"), FileAccess.WRITE)
		if file == null:
			push_error("Could not save actual Chat failure receipt: " + str(FileAccess.get_open_error()))
		else:
			file.store_string(JSON.stringify({"message": message, "receipt": observation(), "painted": transcript(), "controls": chat_control_observations()}, "\t"))
	return super.fail(message)

func received(value: String) -> bool:
	return await wait_until(func(): return " ".join(transcript().replace("\n", " ").split(" ", false)).contains(value) and JSON.stringify(observation()).contains(value), "real received and painted " + value)

func mark(stage: String) -> void:
	FileAccess.open(directory.path_join(role + "-" + stage), FileAccess.WRITE).store_string("actual-native\n")

func other(stage: String) -> bool:
	var peer := "peer" if actor == "leader" else "leader"
	var path := directory.path_join(skin.to_lower() + "-" + peer + "-" + stage)
	var deadline := Time.get_ticks_msec() + 300000
	while Time.get_ticks_msec() < deadline:
		if FileAccess.file_exists(path):
			return true
		await process_frame
	return fail("Other actual actor did not complete " + stage)
