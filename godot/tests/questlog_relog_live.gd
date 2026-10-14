extends "res://tests/questlog_fidelity_live.gd"

# Setup uses native_quest_journal_peer's real query/accept requests, then disconnects.
# This fresh rendered process never talks to a giver or stages quest/detail snapshots.
func check_relog_details() -> bool:
	if not await wait_quest(func(state): return in_log(state, QUEST), "previously accepted quest in fresh owned snapshot", 60000):
		return false
	if not await show_owned_quest(QUEST):
		return false
	await capture("quest-relog-details.png")
	var description := client.find_child("QuestLogDetailsDescription", true, false) as Label
	var has_story := description != null and description.text.contains("Marshal McBride")
	var money := client.find_child("QuestInfoMoneyText", true, false) as Label
	var has_rewards := money != null and not money.text.is_empty()
	var share := share_button()
	var share_disabled := share != null and share.disabled
	print("QUEST_RELOG_OBSERVED story=", has_story, " rewards=", has_rewards, " solo_nonpushable_share_disabled=", share_disabled)
	if not has_story or not has_rewards or not share_disabled:
		fail("Fresh quest log lost authoritative story/rewards or enabled an ineligible Share button")
		return false
	var scroll := client.find_child("QuestMapDetailsScrollFrame", true, false) as Control
	if scroll == null:
		fail("Owned details have no source scrolling viewport")
		return false
	for step in range(24):
		await click(scroll.get_global_rect().get_center(), MOUSE_BUTTON_WHEEL_DOWN, 1)
	print("QUEST_RELOG_SCROLL_RECTS viewport=", scroll.get_global_rect(), " money=", money.get_global_rect())
	if not scroll.get_global_rect().encloses(money.get_global_rect()):
		fail("Fresh-login rewards are not reachable by native wheel scrolling")
		return false
	await capture("quest-relog-rewards-scrolled.png")
	print("PASS QUEST_RELOG_DETAILS skin=", OS.get_environment("QF_SKIN"), " source story/rewards; native wheel reaches rewards; no giver interaction; Share disabled")
	return true

func check_live_party_share() -> bool:
	if not await show_owned_quest(FARGODEEP):
		return false
	if not await wait_frames(func(): return share_button() != null and share_button().disabled and not client.group_state().in_group, "pushable quest solo Share disabled"):
		return false
	await capture("quest-share-pushable-solo.png")
	if not await wait_frames(func(): return FileAccess.file_exists(OS.get_environment("QF_PEER_DIR").path_join("ready")), "real headless peer ready", 60000):
		return false
	if not peer_command("invite"):
		return false
	if not await wait_frames(func(): return client.group_state().get("pending_invite", "") != "", "real party invite"):
		return false
	var yes := quest_control("StaticPopupUI", "StaticPopup1Button1") as BaseButton
	if yes == null or not yes.is_visible_in_tree():
		fail("Party invite has no visible native Yes button")
		return false
	await capture("quest-share-real-invite.png")
	await click_control(yes)
	if not await wait_frames(func(): return client.group_state().in_group and share_button() != null and not share_button().disabled, "roster join enables selected pushable quest"):
		return false
	await capture("quest-share-pushable-grouped.png")
	if not await show_owned_quest(QUEST):
		return false
	if not await wait_frames(func(): return share_button() != null and share_button().disabled, "grouped nonpushable Share disabled"):
		return false
	await capture("quest-share-nonpushable-grouped.png")
	if not await show_owned_quest(FARGODEEP):
		return false
	if not await wait_frames(func(): return share_button() != null and not share_button().disabled, "selected pushable Share enabled"):
		return false
	if not peer_command("leave"):
		return false
	if not await wait_frames(func(): return not client.group_state().in_group and share_button() != null and share_button().disabled, "roster leave disables Share without reopening"):
		return false
	await capture("quest-share-after-leave.png")
	if not peer_command("stop"):
		return false
	print("PASS QUEST_LIVE_SHARE skin=", OS.get_environment("QF_SKIN"), " solo-disabled/grouped-enabled/nonpushable-disabled/leave-disabled; native invite accepted")
	return true

func show_owned_quest(quest_id: int) -> bool:
	if not client.quest_state().log_open:
		await tap(KEY_L)
	if client.world_map_state().get("quest_details", false):
		await click_control(quest_control("WorldMapUI", "QuestMapFrameBackButton"))
		if not await wait_frames(func(): return not client.world_map_state().get("quest_details", true), "quest list restored"):
			return false
	var name := "QuestLogTitle%d" % quest_id
	if not await wait_frames(func(): return client.find_child(name, true, false) != null, "owned quest row"):
		return false
	await click_control(client.find_child(name, true, false) as Control)
	if not await wait_frames(func(): return client.world_map_state().get("quest_details", false), "owned quest details"):
		return false
	await frames(5)
	return true

func share_button() -> BaseButton:
	return client.find_child("QuestMapFrameShareButton", true, false) as BaseButton

func peer_command(command: String) -> bool:
	var directory := OS.get_environment("QF_PEER_DIR")
	if directory.is_empty():
		fail("Private peer control directory missing")
		return false
	var file := FileAccess.open(directory.path_join(command), FileAccess.WRITE)
	if file == null:
		fail("Cannot send private peer command: " + command)
		return false
	file.store_string("native fixture intent")
	return true
