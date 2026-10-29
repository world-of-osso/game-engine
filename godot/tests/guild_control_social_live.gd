extends "res://tests/guild_control_live.gd"

## Two private clients; role a is the Guild Master, role b a Veteran. An offline
## Initiate, Gcrecruit, is the target. The Guild Master edits only the Veteran rank
## through Guild Control; the Veteran proves each edit through what it may do:
##   no rights: the recruit's member menu offers no rank action;
##   Promote and Demote: the recruit goes Initiate -> Member -> Initiate;
##   Remove: the recruit leaves the guild;
##   Guild Chat Speak cleared: the server refuses /g; set again: /g is delivered.

const VETERAN_RANK := 2
const INITIATE_RANK := 4
const RECRUIT := "Gcrecruit"
const SPEAK := "GuildControlUIRankSettingsFrameCheckbox2"
const PROMOTE := "GuildControlUIRankSettingsFrameCheckbox5"
const DEMOTE := "GuildControlUIRankSettingsFrameCheckbox6"
const REMOVE := "GuildControlUIRankSettingsFrameCheckbox8"

func run_completion() -> void:
	managed_rank = VETERAN_RANK
	managed_rank_name = "Veteran"
	var success := await run_social()
	if success:
		print("PASS GUILD_CONTROL_SOCIAL_LIVE role=", role, " skin=", skin)
		completion.case_done()
		client.free()
	completion.finish(success)

func run_social() -> bool:
	root.size = Vector2i(1920, 1080)
	Engine.max_fps = 30
	role = OS.get_environment("BANK_ROLE")
	actor = "leader" if role == "a" else "peer"
	skin = OS.get_environment("GUILD_BANK_SKIN")
	sync_dir = OS.get_environment("BANK_SYNC")
	directory = sync_dir
	trial = sync_dir.get_file()
	shots = OS.get_environment("BANK_SHOTS")
	DirAccess.make_dir_recursive_absolute(shots)
	if role == "a" and not provision_social_guild():
		return false
	if not await barrier("guild-ready"):
		return false
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await wait_frames(30)
	var error: String = client.connect_account(OS.get_environment("GODOT_TEST_SERVER"), OS.get_environment("BANK_ACCOUNT"), PASSWORD, false)
	if error != "":
		return fail(error)
	if not await enter_world(OS.get_environment("BANK_CHARACTER")) or not await select_skin(skin):
		return false
	if not await open_roster():
		return false
	return await rank_actions() and await removal() and await chat_speak() and await barrier("done")

## Fresh guild: Guild Master, Veteran peer and offline Initiate recruit.
func provision_social_guild() -> bool:
	var leader := OS.get_environment("BANK_CHARACTER")
	var member := OS.get_environment("GUILD_PEER_CHARACTER")
	if not ensure_recruit():
		return false
	var id := str(int(Time.get_unix_time_from_system()))
	for step in [
		["create-guild", leader, id, GUILD_NAME],
		["create-guild", member, id, GUILD_NAME],
		["create-guild", RECRUIT, id, GUILD_NAME],
		["set-guild-rank", member, str(VETERAN_RANK)],
		["set-guild-rank", RECRUIT, str(INITIATE_RANK)],
	]:
		if not admin(step):
			return false
	return true

## The recruit is an offline character on the Guild Master's account.
func ensure_recruit() -> bool:
	var output := []
	var arguments := ["create-character", OS.get_environment("BANK_ACCOUNT"), RECRUIT, "1", "2"]
	var code := OS.execute(OS.get_environment("BANK_ADMIN"), arguments, output, true)
	print("FIXTURE ADMIN ", arguments, " ", code, " ", output)
	return code == 0 or "".join(output).contains("already taken") or fail("create recruit failed: %s" % [output])

func open_roster() -> bool:
	await open_communities()
	if not await wait_until(func(): return shown("GuildRosterTab"), "Communities"):
		return false
	await press("GuildRosterTab")
	if not await wait_until(func(): return shown("CommunitiesMemberListShowOfflineButton"), "roster"):
		return false
	# Saved client options may hide offline members; the recruit is offline.
	if not shown("CommunitiesMemberListShowOfflineButtonCheck"):
		await press("CommunitiesMemberListShowOfflineButton")
	return await wait_until(func(): return recruit_rank() == "Initiate", "offline Initiate recruit in the roster")

# --- cases -----------------------------------------------------------------

## Promote and Demote through the member menu once Guild Control grants them.
func rank_actions() -> bool:
	if role == "b":
		await open_recruit_menu()
		await wait_frames(30)
		for item in ["GuildMemberPromote", "GuildMemberDemote", "GuildMemberRemove"]:
			if not completion.require(not shown(item), "Veteran starts without " + item):
				return false
	if not await step("rank-actions-absent"):
		return false
	if role == "a":
		if not await open_guild_control(1) or not await toggle(PROMOTE, true) or not await toggle(DEMOTE, true):
			return false
	if not await step("rank-actions-granted"):
		return false
	if role == "b":
		if not await menu_action("GuildMemberPromote", ["GuildMemberRemove"]):
			return false
	if not await wait_until(func(): return recruit_rank() == "Member", "recruit promoted to Member"):
		return false
	if not await step("promoted"):
		return false
	if role == "b":
		if not await menu_action("GuildMemberDemote", ["GuildMemberRemove"]):
			return false
	if not await wait_until(func(): return recruit_rank() == "Initiate", "recruit demoted to Initiate"):
		return false
	return await step("demoted")

## Remove through the member menu and its confirmation once Guild Control grants it.
func removal() -> bool:
	if role == "a" and not await toggle(REMOVE, true):
		return false
	if not await step("remove-granted"):
		return false
	if role == "b":
		if not await menu_action("GuildMemberRemove", []):
			return false
		if not await wait_until(func(): return shown("GuildMemberRemoveYes"), "removal confirmation"):
			return false
		await press("GuildMemberRemoveYes")
	if not await wait_until(func(): return recruit_row() < 0, "recruit removed from the guild"):
		return false
	return await step("removed")

## Guild Chat Speak cleared, then set again.
func chat_speak() -> bool:
	if role == "a":
		if not await toggle(SPEAK, false):
			return false
	else:
		await press("CommunitiesChatTab")
		if not await wait_until(func(): return shown("CommunitiesChatEditBox"), "Veteran chat"):
			return false
	if not await step("speak-revoked"):
		return false
	if role == "b":
		if not await wait_until(func(): return not composer_enabled(), "composer disabled without Guild Chat Speak"):
			return false
		await tap(KEY_ENTER)
		if not await wait_until(func(): return shown("ChatFrame1EditBox"), "main-chat composer"):
			return false
		await type_text("/g " + message("speak-refused"))
		await tap(KEY_ENTER)
		if not await wait_until(func(): return text("CommunitiesChatError") == PERMISSIONS, "guild chat refused"):
			return false
	if not await step("speak-refused"):
		return false
	if role == "a":
		if not await toggle(SPEAK, true):
			return false
		await press("GuildControlUICloseButton")
		if not await wait_until(func(): return not shown("GuildControlUI"), "Guild Control closed"):
			return false
		await press("CommunitiesChatTab")
		if not await wait_until(func(): return shown("CommunitiesChatEditBox"), "Guild Master chat"):
			return false
	if not await step("speak-granted"):
		return false
	if role == "b":
		if not await wait_until(composer_enabled, "composer enabled by Guild Chat Speak"):
			return false
		if not await compose(message("speak-delivered")):
			return false
	elif not await received(message("speak-delivered")):
		return false
	return await step("speak-delivered")

# --- roster ----------------------------------------------------------------

func composer_enabled() -> bool:
	return control("CommunitiesChatEditBox").mouse_filter != Control.MOUSE_FILTER_IGNORE

func recruit_row() -> int:
	for index in range(0, 40):
		if text("GuildMember%dName" % index) == RECRUIT:
			return index
	return -1

func recruit_rank() -> String:
	var row := recruit_row()
	return text("GuildMember%dRank" % row) if row >= 0 else ""

func open_recruit_menu() -> void:
	await click_at(control("GuildMember%d" % recruit_row()).get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)

## Right-click the recruit, require `item` and none of `absent`, then choose `item`.
func menu_action(item: String, absent: Array) -> bool:
	await open_recruit_menu()
	if not await wait_until(func(): return shown(item), item + " offered"):
		return false
	for other_item in absent:
		if not completion.require(not shown(other_item), other_item + " offered without its right"):
			return false
	await press(item)
	return true
