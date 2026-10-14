extends "res://tests/questlog_fidelity_live.gd"

# First process: check_skin,check_prepare_relog. Fresh process: check_skin,check_relog_details.
# Both use real server-owned quest 28766; no giver interaction in the fresh process.
func check_prepare_relog() -> bool:
	if in_log(client.quest_state(), QUEST):
		print("PASS QUEST_RELOG_PREPARED existing owned quest")
		return true
	if not await open_quest(GIVER_AT, GIVER, QUEST_TITLE):
		return false
	if not await wait_quest(func(s): return s.get("page") == "Detail" and s.quest_id == QUEST, "real giver details"):
		return false
	await click_control(quest_control("QuestFrameUI", "QuestFrameAcceptButton"))
	if not await wait_quest(func(s): return in_log(s, QUEST) and not s.frame_open, "owned quest accepted"):
		return false
	await capture("quest-relog-prepared.png")
	return true

func check_relog_details() -> bool:
	if not in_log(client.quest_state(), QUEST):
		fail("Relog fixture requires the previously accepted quest")
		return false
	await tap(KEY_L)
	if not await wait_frames(func(): return client.find_child("QuestLogTitle%d" % QUEST, true, false) != null, "relogged quest row"):
		return false
	await click_control(client.find_child("QuestLogTitle%d" % QUEST, true, false) as Control)
	if not await wait_frames(func(): return client.world_map_state().get("quest_details", false), "relogged quest details"):
		return false
	await frames(5)
	await capture("quest-relog-details.png")
	var description := client.find_child("QuestLogDetailsDescription", true, false) as Label
	var has_story := description != null and description.text.contains("Marshal McBride")
	var money := client.find_child("QuestInfoMoneyText", true, false) as Label
	var has_rewards := money != null and not money.text.is_empty()
	var share := client.find_child("QuestMapFrameShareButton", true, false) as BaseButton
	var share_disabled := share != null and share.disabled
	print("QUEST_RELOG_OBSERVED story=", has_story, " rewards=", has_rewards, " solo_nonpushable_share_disabled=", share_disabled)
	if not has_story or not has_rewards or not share_disabled:
		fail("Fresh quest log lost authoritative story/rewards or enabled an ineligible Share button")
		return false
	print("PASS QUEST_RELOG_DETAILS skin=", OS.get_environment("QF_SKIN"), " story/rewards from fresh login; solo nonpushable Share disabled")
	return true
