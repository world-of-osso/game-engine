extends "res://tests/guild_member_notes_live.gd"

const PEER_NAME := "Gclogpeer"
var diagnostics := ""
var last_log: Dictionary = {}
var joined_count := 0

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	Engine.max_fps = 30
	shots = OS.get_environment("BANK_SHOTS")
	diagnostics = OS.get_environment("GUILD_LOG_DIAGNOSTICS")
	role = "guild-lifecycle-entry"
	assert(OS.get_environment("GODOT_TEST_SERVER") == "127.0.0.1:5891", "Private5891 only")
	assert(OS.get_environment("BANK_ACCOUNT").begins_with("fb_guildcompletion_"), "Private test account required")
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
	var mode := OS.get_environment("GUILD_LOG_MODE")
	if mode == "produce":
		if not await produce_history():
			return
		mark("done.go")
	elif mode == "readback":
		if not await readback_history():
			return
	else:
		fail("Unknown lifecycle mode")
		return
	print("PASS GUILD_LIFECYCLE_LIVE mode=", mode, " count=", last_log.events.size(), " build=", OS.get_environment("GUILD_LOG_BUILD"))
	client.free()
	quit(0)

func produce_history() -> bool:
	if not await wait_until(func(): return FileAccess.file_exists(diagnostics.path_join("peer-ready.json")), "real peer world entry"):
		return false
	if read_json("peer-ready.json").name != PEER_NAME:
		return fail("Real target peer entered the wrong character")
	for index in range(2):
		var skin := "Modern" if index == 0 else "Forever"
		if not await select_skin(skin) or not await open_guild():
			return false
		if rank_of(OS.get_environment("BANK_CHARACTER")) != "Officer":
			return fail("Native actor is not the actual authorized Officer")
		if not await query_log(index * 8):
			return false
		if index == 0:
			await capture("empty")
		await press("CommunitiesGuildLogClose")
		if not await eight_lifecycle_actions(index + 1):
			return false
		if not await query_log((index + 1) * 8):
			return false
		if not check_six_kinds(last_log.events.slice(index * 8)):
			return false
		write_json(skin.to_lower() + "-actions.json", last_log)
		await capture("events")
		await close_guild()
	if not await open_guild() or not await invite_peer():
		return false
	for index in range(100):
		if not await change_rank(index % 2 == 0):
			return false
	await close_guild()
	for skin in ["Modern", "Forever"]:
		if not await select_skin(skin) or not await open_guild() or not await query_log(100):
			return false
		if kind(last_log.events.front()) != "Promote" or kind(last_log.events.back()) != "Demote":
			return fail("Retention lost real promotion/demotion insertion order")
		await capture("overflow-top")
		if not await scroll_to_oldest():
			return false
		await capture("overflow-bottom")
		write_json("history.json", last_log)
		await close_guild()
	var ordinary := read_json("member-read.json")
	var denied := read_json("permission-refusal.json")
	if ordinary.is_empty() or ordinary.error != null or ordinary.events.size() < 2 or denied.is_empty() or denied.error != "Permissions":
		return fail("Actual ordinary-member read or rejected promotion receipt missing")
	return true

func eight_lifecycle_actions(leave_sequence: int) -> bool:
	if not await invite_peer() or not await change_rank(true) or not await change_rank(false):
		return false
	await click_at(control("GuildMember%d" % member_row(PEER_NAME)).get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
	await press("GuildMemberRemove")
	if not await wait_until(func(): return shown("GuildMemberRemovePopupText"), "real removal confirmation"):
		return false
	if not text("GuildMemberRemovePopupText").contains(PEER_NAME):
		return fail("Removal popup captured the wrong member")
	await capture("remove-confirmation")
	await press("GuildMemberRemoveYes")
	if not await wait_until(func(): return member_row(PEER_NAME) < 0, "authoritative member removal"):
		return false
	if not await invite_peer():
		return false
	mark("leave-%d.go" % leave_sequence)
	if not await wait_until(func(): return member_row(PEER_NAME) < 0 and FileAccess.file_exists(diagnostics.path_join("privacy-%d.json" % leave_sequence)), "actual peer quit and member-only history refusal"):
		return false
	var private_reply := read_json("privacy-%d.json" % leave_sequence)
	if private_reply.error != "NotInGuild" or not private_reply.events.is_empty():
		return fail("Former member received guild history")
	return true

func invite_peer() -> bool:
	await press("CommunitiesChatTab")
	await press("GuildRosterInviteButton")
	await press("GuildInviteMemberName")
	var edit := control("GuildInviteMemberName") as LineEdit
	edit.select_all()
	await tap(KEY_BACKSPACE)
	await type_text(PEER_NAME)
	if edit.text != PEER_NAME:
		return fail("Physical invitation input mismatch")
	await press("GuildInviteMemberAccept")
	joined_count += 1
	if not await wait_until(func(): return not read_json("joined.json").is_empty() and int(read_json("joined.json").joined) == joined_count, "real invitation acceptance"):
		return false
	await press("GuildRosterTab")
	return await wait_until(func(): return member_row(PEER_NAME) >= 0 and rank_of(PEER_NAME) == "Initiate", "authoritative joining member")

func change_rank(promote: bool) -> bool:
	await press("GuildRosterTab")
	var row := member_row(PEER_NAME)
	if row < 0:
		return fail("Promotion target missing")
	await click_at(control("GuildMember%d" % row).get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
	await press("GuildMemberPromote" if promote else "GuildMemberDemote")
	var expected := "Member" if promote else "Initiate"
	return await wait_until(func(): return rank_of(PEER_NAME) == expected, "authoritative rank " + expected)

func open_guild() -> bool:
	if not shown("CommunitiesFrame"):
		await toggle_guild()
	if not await wait_until(func(): return shown("GuildRosterTab"), "guild source tabs"):
		return false
	await press("GuildRosterTab")
	return await wait_until(func(): return member_row(OS.get_environment("BANK_CHARACTER")) >= 0, "authoritative actor membership")

func close_guild() -> void:
	if shown("CommunitiesFrame"):
		await toggle_guild()
	await wait_until(func(): return not shown("CommunitiesFrame"), "guild window closes")

func toggle_guild() -> void:
	if shown("GuildMicroButton"):
		await press("GuildMicroButton")
	else:
		await press("MinimapLauncherButton")
		await press("LauncherEntryGuildMicroButton")

func query_log(expected_count: int) -> bool:
	await press("GuildInfoTab")
	await press("CommunitiesGuildLogButton")
	if not await wait_until(func(): return shown("CommunitiesGuildLogFrameTitle") or (not observation().is_empty() and observation().error != null), "source Log dialog or real refusal"):
		return false
	if not await wait_until(func(): return not observation().is_empty(), "fresh authoritative event log reply"):
		return false
	last_log = observation()
	if last_log.error != null or last_log.events.size() != expected_count:
		return fail("Real log count/refusal mismatch: " + JSON.stringify(last_log))
	if text("CommunitiesGuildLogFrameTitle") != "Log":
		return fail("Source GlobalStrings Log title missing")
	return true

func observation() -> Dictionary:
	var ui := client.get_node_or_null("GuildRanksUI")
	if ui == null:
		return {}
	var raw := str(ui.call("guild_event_log_observation"))
	if raw.is_empty():
		return {}
	var envelope: Variant = JSON.parse_string(raw)
	assert(envelope is Dictionary and envelope.has("waiting") and envelope.has("reply"), "Real native history receipt required")
	if envelope.waiting or envelope.reply == null:
		return {}
	return envelope.reply

func member_row(name: String) -> int:
	for index in range(50):
		if text("GuildMember%dName" % index) == name:
			return index
	return -1

func rank_of(name: String) -> String:
	var index := member_row(name)
	return text("GuildMember%dRank" % index) if index >= 0 else ""

func scroll_to_oldest() -> bool:
	var viewport := control("CommunitiesGuildLogScrollFrame").get_global_rect()
	if viewport.intersects(entry_bounds(99)):
		return fail("Overflow proof needs a genuine second page")
	for turn in range(160):
		var oldest := entry_bounds(99) # Re-observe after every UI rebuild, never retain freed controls.
		if oldest.position.y >= viewport.position.y and oldest.end.y <= viewport.end.y:
			return true
		await click_at(viewport.get_center(), MOUSE_BUTTON_WHEEL_DOWN)
	return fail("Real wheel input never fully revealed the oldest retained entry")

func entry_bounds(index: int) -> Rect2:
	var bounds := Rect2()
	var ui := client.get_node("GuildRanksUI")
	for node in ui.find_children("CommunitiesGuildLogEntry%dRow*" % index, "Label", true, false):
		var label := node as Control
		var rectangle := label.get_global_rect()
		bounds = bounds.merge(rectangle) if bounds.has_area() else rectangle
	assert(bounds.has_area(), "Actual rendered event labels required")
	return bounds

func kind(event: Dictionary) -> String:
	assert(event.kind is Dictionary and event.kind.size() == 1, "Typed event kind required")
	return str(event.kind.keys()[0])

func check_six_kinds(events: Array) -> bool:
	var expected := ["Invite", "Join", "Promote", "Demote", "Remove", "Invite", "Join", "Quit"]
	if events.size() != expected.size():
		return fail("Wrong real lifecycle action count")
	for index in range(expected.size()):
		if kind(events[index]) != expected[index]:
			return fail("Wrong real lifecycle ordering")
	return true

func readback_history() -> bool:
	var saved := read_json("history.json")
	if saved.is_empty() or saved.events.size() != 100:
		return fail("Original complete history receipt missing")
	for skin in ["Modern", "Forever"]:
		if not await select_skin(skin) or not await open_guild() or not await query_log(100):
			return false
		if last_log.events != saved.events:
			return fail("Actual server restart changed retained events or timestamps")
		await capture("restart")
		await close_guild()
	return true

func select_skin(skin: String) -> bool:
	var selected: bool = await super.select_skin(skin)
	role = skin.to_lower() + "-lifecycle"
	return selected

func mark(filename: String) -> void:
	var file := FileAccess.open(diagnostics.path_join(filename), FileAccess.WRITE)
	assert(file != null, "Create actual peer command marker")
	file.close()

func write_json(filename: String, value: Dictionary) -> void:
	var file := FileAccess.open(diagnostics.path_join(filename), FileAccess.WRITE)
	assert(file != null, "Write lifecycle receipt")
	file.store_string(JSON.stringify(value, "\t"))
	file.close()

func read_json(filename: String) -> Dictionary:
	var path := diagnostics.path_join(filename)
	if not FileAccess.file_exists(path):
		return {}
	var value: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	assert(value is Dictionary, "Complete lifecycle JSON receipt required")
	return value
