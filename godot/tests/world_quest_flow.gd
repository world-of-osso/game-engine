extends SceneTree

## Native quest flow in Northshire (docs/specs/quest-ui.md) on a private server:
## Marshal McBride wears the yellow `!` talktome marker; a real right-click opens
## QuestFrame on his greeting; clicking "Beating Them Back!" shows its detail page and
## Accept puts it in the log, the tracker and on the world map and minimap, with the
## grey `?` over McBride; two minutes of Blackrock Worg hunting, then admin
## `quest-complete` fills the six kills, and McBride shows the
## yellow `?`; the turn-in pays money and XP and offers the follow-up "Lions for Lambs",
## which is accepted and then abandoned through the quest log and the ABANDON_QUEST popup.
## Then the reward choice on "Riverpaw Gnoll Bounty" (11): Marshal Dughan's chain
## The Fargodeep Mine (62) -> The Jasperlode Mine (76) -> "Westbrook Garrison Needs
## Help!" (239) in Goldshire, which is turned in to Deputy Rainer, who offers
## 11 as its follow-up; the eight Painted Gnoll Armbands come from the server admin
## `grant-item` (setup, not the gnoll hunt). Complete Quest shows "You must choose a
## reward." until a reward is chosen, and the chosen Urchin's Pants land in the bags.
## Last the fixed-item reward of "Extinguishing Hope" (26391) from Milly Osworth: its
## eight Vineyard Fires (a spell credit the server does not model) come from admin
## `quest-complete`, and both reward items land in the bags. Travel between NPCs uses
## the admin `teleport`, the fixture's stand-in for the walk; short distances are walked
## with W and the turn keys.
## Environment:
##   GODOT_TEST_SERVER  a private test server (never the shared :5000)
##   QF_ACCOUNT         account (password fbtest) with one Human warrior who has not
##                      taken 28766, standing near Marshal McBride
##   QF_SHOTS           screenshot directory
##   QF_STEPS           optional subset of steps, e.g. check_reward_choice,check_fixed_reward
##   QF_ADMIN           game-server-admin binary of that server (GAME_SERVER_ADMIN_SOCKET set)

const PASSWORD := "fbtest"
const GIVER := "Marshal McBride"
const PREY := "Blackrock Worg"
const QUEST := 28766
const QUEST_TITLE := "Beating Them Back!"
const NEXT_QUEST := 28774
const NEXT_TITLE := "Lions for Lambs"
const TALKTOME := 130731
const TALKTOME_QUESTION := 130738
const TALKTOME_QUESTION_GREY := 130735
const WORLD_WAIT_MS := 180000
const WAIT_MS := 20000
const INTERACT_YARDS := 4.0
const MELEE_YARDS := 3.0
const FARGODEEP := 62
const JASPERLODE := 76
const GARRISON_QUEST := 239
const GARRISON_TITLE := "Westbrook Garrison Needs Help!"
const CHOICE_QUEST := 11
const CHOICE_TITLE := "Riverpaw Gnoll Bounty"
const DUGHAN := "Marshal Dughan"
const RAINER := "Deputy Rainer"
const ARMBAND := 782
const URCHINS_PANTS := 2238
const FIXED_QUEST := 26391
const FIXED_TITLE := "Extinguishing Hope"
const MILLY := "Milly Osworth"
const FIXED_REWARDS := {57247: 1, 11475: 1}
const MUST_CHOOSE := "You must choose a reward."
## WoW world positions about three yards from each NPC's spawn.
const DUGHAN_AT := [-9462.5, 74.0, 56.8]
const RAINER_AT := [-9659.8, 694.3, 36.9]
const MILLY_AT := [-8921.0, -138.5, 81.1]
## WoW world positions: the middle of the Blackrock Worg spawns inside the objective
## area, and two yards from Marshal McBride.
const WORG_FIELD_AT := [-8858.0, -112.0, 81.5]
const GIVER_AT := [-8915.4, -137.5, 81.0]
const HUNT_YARDS := 30.0

var client: Node
var shots := "/tmp/claude/quests/shots/"

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if server == "" or server.ends_with(":5000"):
		fail("GODOT_TEST_SERVER must name a private server, not :5000")
		return
	if OS.get_environment("QF_SHOTS") != "":
		shots = OS.get_environment("QF_SHOTS").trim_suffix("/") + "/"
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, OS.get_environment("QF_ACCOUNT"), PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world():
		return
	var steps := [check_pickup, check_map_objectives, check_kills, check_turn_in, check_abandon, check_reward_choice, check_fixed_reward]
	# QF_STEPS (comma-separated method names) reruns a subset on a prepared character.
	if OS.get_environment("QF_STEPS") != "":
		steps = Array(OS.get_environment("QF_STEPS").split(",")).map(func(name): return Callable(self, name))
	for step in steps:
		if not await step.call():
			return
	print("FIXTURE QUEST_FLOW_DONE")
	client.free()
	quit(0)

# --- steps -------------------------------------------------------------------

func check_pickup() -> bool:
	var giver := await locate(GIVER)
	if giver.is_empty():
		return false
	if not await wait_quest(func(s): return s.markers.get(giver.id) == TALKTOME, "yellow ! over " + GIVER):
		return false
	if not await approach(giver.id, INTERACT_YARDS + 4.0):
		fail("Could not walk to " + GIVER)
		return false
	await face_unit(giver.id)
	await capture("01-marker-available.png")
	if not await approach(giver.id, INTERACT_YARDS):
		fail("Could not walk to " + GIVER)
		return false
	giver = await find_unit(GIVER)
	print("FIXTURE MARKER ", GIVER, " ", client.quest_state().markers.get(giver.id))
	if not await open_giver(giver):
		return false
	var state: Dictionary = client.quest_state()
	var index: int = Array(state.greeting_quests).find(QUEST_TITLE)
	if index < 0:
		fail("%s offers %s, not %s" % [GIVER, state.greeting_quests, QUEST_TITLE])
		return false
	await capture("02-greeting.png")
	await click_control(quest_control("QuestFrameUI", "QuestTitleButton%dText" % (index + 1)))
	if not await wait_quest(func(s): return s.get("page") == "Detail" and s.quest_id == QUEST, "detail page"):
		return false
	await capture("03-detail.png")
	await click_control(quest_control("QuestFrameUI", "QuestFrameAcceptButton"))
	if not await wait_quest(func(s): return in_log(s, QUEST) and not s.frame_open, "accepted quest in the log"):
		return false
	if not await wait_quest(func(s): return Array(s.system_lines).has("Quest accepted: " + QUEST_TITLE), "accept line"):
		return false
	if not await wait_quest(func(s): return s.markers.get(giver.id) == TALKTOME_QUESTION_GREY, "grey ? over " + GIVER):
		return false
	var tracker: Dictionary = client.objective_tracker_state()
	if not tracker_has(tracker, QUEST):
		fail("Tracker lacks %d: %s" % [QUEST, tracker])
		return false
	await capture("04-accepted.png")
	print("FIXTURE ACCEPTED ", log_entry(client.quest_state(), QUEST))
	return true

func check_map_objectives() -> bool:
	await tap(KEY_M)
	if not await wait_frames(func(): return client.world_map_state().get("open", false), "world map"):
		return false
	var pins: Array = client.world_map_state().pins
	var pin := {}
	for candidate in pins:
		if candidate.label == QUEST_TITLE:
			pin = candidate
	if pin.is_empty() or pin.type != "QuestObjective":
		fail("World map has no objective pin for %s: %s" % [QUEST_TITLE, pins])
		return false
	print("FIXTURE WORLD_MAP_PIN ", pin)
	await capture("05-world-map-objective.png")
	await tap(KEY_M)
	var minimap: Dictionary = client.minimap_state()
	print("FIXTURE MINIMAP quest_areas ", minimap.get("quest_areas", []))
	if minimap.get("quest_areas", []).is_empty():
		fail("Minimap draws no objective area for %s: %s" % [QUEST_TITLE, minimap])
		return false
	await capture("06-minimap-objective.png")
	return true

func check_kills() -> bool:
	if not await teleport(WORG_FIELD_AT):
		return false
	# Hunt for real for two minutes: a kill shows kill credit in the log and tracker.
	# The worgs mostly fight Northshire soldiers, who take the credit, so admin
	# `quest-complete` fills whatever is left.
	var deadline := Time.get_ticks_msec() + 120000
	while Time.get_ticks_msec() < deadline:
		var entry := log_entry(client.quest_state(), QUEST)
		if not String(entry.objectives[0]).begins_with("0/"):
			await capture("06b-kill-credit.png")
			break
		var prey := nearest_living(PREY)
		if prey.is_empty():
			await frames(30)
			continue
		if await approach(prey.id, MELEE_YARDS):
			await attack(prey.id, entry.objectives[0])
	print("FIXTURE HUNTED ", log_entry(client.quest_state(), QUEST).objectives, " alive ", client.account_state().get("local_player_alive"))
	# The worg pack can kill the level-4 warrior; the dead cannot talk to McBride.
	if not admin(["revive", character_name()]):
		return false
	if not admin(["quest-complete", character_name(), str(QUEST)]):
		return false
	if not await wait_quest(func(s): return log_entry(s, QUEST).get("completed", false), "worg objective complete"):
		return false
	print("FIXTURE COMPLETED ", log_entry(client.quest_state(), QUEST))
	return true

func check_turn_in() -> bool:
	var giver := await locate(GIVER)
	if giver.is_empty():
		return false
	if not await wait_quest(func(s): return s.markers.get(giver.id) == TALKTOME_QUESTION, "yellow ? over " + GIVER):
		return false
	if not await teleport(GIVER_AT) or not await approach(giver.id, INTERACT_YARDS):
		fail("Could not walk back to " + GIVER)
		return false
	await face_unit(giver.id)
	await capture("07-ready-for-turn-in.png")
	var tracker: Dictionary = client.objective_tracker_state()
	print("FIXTURE TRACKER_READY ", tracker.get("quests"))
	if not await open_giver(await find_unit(GIVER)):
		return false
	var state: Dictionary = client.quest_state()
	var index: int = Array(state.greeting_quests).find(QUEST_TITLE)
	if index < 0:
		fail("%s greeting lacks the turn-in: %s" % [GIVER, state.greeting_quests])
		return false
	await click_control(quest_control("QuestFrameUI", "QuestTitleButton%dText" % (index + 1)))
	if not await wait_quest(func(s): return s.get("page") == "Reward" and s.quest_id == QUEST, "reward page"):
		return false
	await capture("08-reward.png")
	var xp_before: int = client.quest_state().get("xp", -1)
	var choices: int = client.quest_state().choices
	if choices > 0:
		await click_control(quest_control("QuestFrameUI", "QuestInfoRewardsFrameQuestInfoItem1"))
	await click_control(quest_control("QuestFrameUI", "QuestFrameCompleteQuestButton"))
	if not await wait_quest(func(s): return not in_log(s, QUEST) and Array(s.system_lines).has(QUEST_TITLE + " completed."), "turn-in"):
		return false
	if not await wait_quest(func(s): return s.get("page") == "Detail" and s.quest_id == NEXT_QUEST, "follow-up offer"):
		return false
	var after: Dictionary = client.quest_state()
	var gained := ""
	for line in after.system_lines:
		if String(line).begins_with("Experience gained: "):
			gained = line
	print("FIXTURE TURNED_IN xp ", xp_before, " -> ", after.get("xp", -1), " '", gained, "' lines ", after.system_lines)
	if gained == "" or after.get("xp", -1) == xp_before:
		fail("Turn-in gave no XP: before %d after %s" % [xp_before, after.get("xp")])
		return false
	await capture("09-follow-up-offered.png")
	await click_control(quest_control("QuestFrameUI", "QuestFrameAcceptButton"))
	if not await wait_quest(func(s): return in_log(s, NEXT_QUEST), "follow-up accepted"):
		return false
	print("FIXTURE FOLLOW_UP_ACCEPTED ", log_entry(client.quest_state(), NEXT_QUEST))
	return true

func check_abandon() -> bool:
	await tap(KEY_L)
	if not await wait_quest(func(s): return s.log_open, "quest log"):
		return false
	await frames(3)
	await click_control(quest_control("QuestLogUI", "QuestLogTitle%dText" % NEXT_QUEST))
	await frames(3)
	await capture("10-quest-log.png")
	await click_control(quest_control("QuestLogUI", "QuestLogAbandonButton"))
	await frames(3)
	var yes := quest_control("StaticPopupUI", "StaticPopup1Button1")
	if yes == null or not yes.is_visible_in_tree():
		fail("No ABANDON_QUEST popup")
		return false
	await capture("11-abandon-popup.png")
	await click_control(yes)
	if not await wait_quest(func(s): return not in_log(s, NEXT_QUEST), "abandoned quest leaves the log"):
		return false
	await capture("12-abandoned.png")
	await tap(KEY_L)
	if not await wait_quest(func(s): return not s.log_open, "quest log closed"):
		return false
	print("FIXTURE ABANDONED ", NEXT_QUEST, " log ", client.quest_state().log)
	return true

func check_reward_choice() -> bool:
	# 239 needs The Jasperlode Mine (76) turned in, which follows The Fargodeep Mine
	# (62); both are Dughan's exploration quests, which the server completes on accept
	# (no area-trigger data).
	if not await take_quest(DUGHAN_AT, DUGHAN, FARGODEEP, "The Fargodeep Mine"):
		return false
	if not await turn_in(DUGHAN_AT, DUGHAN, FARGODEEP, "The Fargodeep Mine"):
		return false
	if not await wait_quest(func(s): return s.get("page") == "Detail" and s.quest_id == JASPERLODE, "The Jasperlode Mine offered as the follow-up"):
		return false
	await click_control(quest_control("QuestFrameUI", "QuestFrameAcceptButton"))
	if not await wait_quest(func(s): return in_log(s, JASPERLODE), "The Jasperlode Mine accepted"):
		return false
	if not await turn_in(DUGHAN_AT, DUGHAN, JASPERLODE, "The Jasperlode Mine"):
		return false
	if not await take_quest(DUGHAN_AT, DUGHAN, GARRISON_QUEST, GARRISON_TITLE):
		return false
	if not await open_quest(RAINER_AT, RAINER, GARRISON_TITLE):
		return false
	if not await reach_reward(GARRISON_QUEST, ""):
		return false
	await click_control(quest_control("QuestFrameUI", "QuestFrameCompleteQuestButton"))
	if not await wait_quest(func(s): return s.get("page") == "Detail" and s.quest_id == CHOICE_QUEST, CHOICE_TITLE + " offered as the follow-up"):
		return false
	await capture("13-choice-quest-detail.png")
	await click_control(quest_control("QuestFrameUI", "QuestFrameAcceptButton"))
	if not await wait_quest(func(s): return in_log(s, CHOICE_QUEST), "choice quest accepted"):
		return false
	if not admin(["grant-item", character_name(), str(ARMBAND), "8"]):
		return false
	if not await wait_quest(func(s): return log_entry(s, CHOICE_QUEST).get("completed", false), "eight armbands complete " + CHOICE_TITLE):
		return false
	var rainer := await locate(RAINER)
	if not await wait_quest(func(s): return s.markers.get(rainer.id) == TALKTOME_QUESTION, "yellow ? over " + RAINER):
		return false
	if not await open_quest(RAINER_AT, RAINER, CHOICE_TITLE):
		return false
	if not await reach_reward(CHOICE_QUEST, "14-progress.png"):
		return false
	if client.quest_state().choices != 2:
		fail("Reward page without the two choices: " + str(client.quest_state()))
		return false
	await capture("15-reward-choice.png")
	var pants_before := bag_count(URCHINS_PANTS)
	await click_control(quest_control("QuestFrameUI", "QuestFrameCompleteQuestButton"))
	if not await wait_frames(func(): return error_shown(MUST_CHOOSE), "'" + MUST_CHOOSE + "'"):
		return false
	if not in_log(client.quest_state(), CHOICE_QUEST) or client.quest_state().get("page") != "Reward":
		fail("Complete Quest turned in without a choice: " + str(client.quest_state()))
		return false
	await capture("16-must-choose.png")
	await click_control(quest_control("QuestFrameUI", "QuestInfoRewardsFrameQuestInfoItem2"))
	if not await wait_quest(func(s): return s.get("choice") == 1, "second reward chosen"):
		return false
	await capture("17-reward-chosen.png")
	await click_control(quest_control("QuestFrameUI", "QuestFrameCompleteQuestButton"))
	if not await wait_quest(func(s): return not in_log(s, CHOICE_QUEST), "choice quest turned in"):
		return false
	if not await wait_frames(func(): return bag_count(URCHINS_PANTS) == pants_before + 1, "Urchin's Pants in the bags"):
		return false
	if not await wait_quest(func(s): return Array(s.system_lines).has("You receive item: [Urchin's Pants]."), "received-item line"):
		return false
	await open_bags("18-choice-in-bags.png")
	print("FIXTURE REWARD_CHOICE pants ", pants_before, " -> ", bag_count(URCHINS_PANTS))
	return true

func check_fixed_reward() -> bool:
	if not await take_quest(MILLY_AT, MILLY, FIXED_QUEST, FIXED_TITLE):
		return false
	if not admin(["quest-complete", character_name(), str(FIXED_QUEST)]):
		return false
	if not await wait_quest(func(s): return log_entry(s, FIXED_QUEST).get("completed", false), FIXED_TITLE + " objectives complete"):
		return false
	if not await open_quest(MILLY_AT, MILLY, FIXED_TITLE):
		return false
	if not await reach_reward(FIXED_QUEST, ""):
		return false
	if client.quest_state().choices != 0:
		fail("Fixed reward page offers choices: " + str(client.quest_state()))
		return false
	await capture("19-fixed-reward.png")
	var before := {}
	for item in FIXED_REWARDS:
		before[item] = bag_count(item)
	await click_control(quest_control("QuestFrameUI", "QuestFrameCompleteQuestButton"))
	if not await wait_quest(func(s): return not in_log(s, FIXED_QUEST), FIXED_TITLE + " turned in"):
		return false
	for item in FIXED_REWARDS:
		if not await wait_frames(func(): return bag_count(item) == before[item] + FIXED_REWARDS[item], "reward item %d in the bags" % item):
			return false
	await open_bags("20-fixed-in-bags.png")
	print("FIXTURE FIXED_REWARD ", before, " lines ", Array(client.quest_state().system_lines).slice(-4))
	return true

## The reward page of `quest_id`, through its progress page (captured as `shot` when
## set) with Continue when the server shows one first.
func reach_reward(quest_id: int, shot: String) -> bool:
	if not await wait_quest(func(s): return s.get("page") in ["Progress", "Reward"] and s.quest_id == quest_id, "turn-in page of %d" % quest_id):
		return false
	if client.quest_state().page == "Progress":
		if shot != "":
			await capture(shot)
		await click_control(quest_control("QuestFrameUI", "QuestFrameCompleteButton"))
	return await wait_quest(func(s): return s.get("page") == "Reward" and s.quest_id == quest_id, "reward page of %d" % quest_id)

## Teleport to `at`, open `npc` and turn in the choiceless quest `quest_id`.
func turn_in(at: Array, npc: String, quest_id: int, title: String) -> bool:
	if not await open_quest(at, npc, title) or not await reach_reward(quest_id, ""):
		return false
	await click_control(quest_control("QuestFrameUI", "QuestFrameCompleteQuestButton"))
	return await wait_quest(func(s): return not in_log(s, quest_id) and Array(s.system_lines).has(title + " completed."), title + " turned in")

## Teleport to `at`, open `npc` and accept `quest_id` from its greeting.
func take_quest(at: Array, npc: String, quest_id: int, title: String) -> bool:
	if not await open_quest(at, npc, title):
		return false
	if not await wait_quest(func(s): return s.get("page") == "Detail" and s.quest_id == quest_id, title + " details"):
		return false
	await click_control(quest_control("QuestFrameUI", "QuestFrameAcceptButton"))
	return await wait_quest(func(s): return in_log(s, quest_id), title + " accepted")

## Teleport to `at`, open `npc`'s greeting and click the quest titled `title`.
func open_quest(at: Array, npc: String, title: String) -> bool:
	if not await teleport(at):
		return false
	var unit := await locate(npc)
	if unit.is_empty() or not await approach(unit.id, INTERACT_YARDS):
		fail("Could not reach " + npc)
		return false
	if not await open_giver(await find_unit(npc)):
		return false
	var index: int = Array(client.quest_state().greeting_quests).find(title)
	if index < 0:
		fail("%s offers %s, not %s" % [npc, client.quest_state().greeting_quests, title])
		return false
	await click_control(quest_control("QuestFrameUI", "QuestTitleButton%dText" % (index + 1)))
	return true

func open_bags(file: String) -> void:
	await tap(KEY_B)
	await frames(10)
	await capture(file)
	await tap(KEY_ESCAPE)
	await frames(5)

## A `UIErrorsFrame` line showing `text`.
func error_shown(text: String) -> bool:
	var ui = client.get_node_or_null("UIErrors")
	if ui == null:
		return false
	for label in ui.find_children("*", "Label", true, false):
		if label.text == text and label.is_visible_in_tree():
			return true
	return false

func character_name() -> String:
	return client.account_state().selected_character_name

## `game-server-admin` (QF_ADMIN) with `args`.
func admin(args: Array) -> bool:
	var output := []
	var code := OS.execute(OS.get_environment("QF_ADMIN"), args, output, true)
	print("FIXTURE ADMIN ", args, " -> ", code, " ", output)
	if code != 0:
		fail("Admin %s failed (%d): %s" % [args, code, output])
	return code == 0

func bag_count(item_id: int) -> int:
	var count := 0
	for item in client.merchant_state().bags:
		if item.item_id == item_id:
			count += item.count
	return count

## Server-side travel (`game-server-admin teleport`) to WoW world `at` on map 0.
func teleport(at: Array) -> bool:
	if not admin(["teleport", character_name(), "0", str(at[0]), str(at[1]), str(at[2])]):
		return false
	var want := Vector3(at[0], at[2], -at[1])
	return await wait_frames(func(): return player_position().distance_to(want) < 4.0, "teleport to " + str(at), 30000)

# --- quest helpers -------------------------------------------------------------

func in_log(state: Dictionary, quest_id: int) -> bool:
	return not log_entry(state, quest_id).is_empty()

func log_entry(state: Dictionary, quest_id: int) -> Dictionary:
	for entry in state.log:
		if entry.quest_id == quest_id:
			return entry
	return {}

func tracker_has(state: Dictionary, quest_id: int) -> bool:
	for quest in state.get("quests", []):
		if quest.quest_id == quest_id:
			return true
	return false

func quest_control(ui_name: String, name: String) -> Control:
	var ui = client.get_node_or_null(ui_name)
	var control = ui.find_child(name, true, false) as Control if ui != null else null
	if control == null:
		var names := []
		if ui != null:
			for node in ui.find_children("Quest*", "Control", true, false):
				names.append(str(node.name))
		print("FIXTURE MISSING ", ui_name, "/", name, " has ", names.slice(0, 40))
	return control

## Right-click `giver` until its greeting opens; a click can land while the camera or
## the NPC still moves, so it is re-aimed up to three times.
func open_giver(giver: Dictionary) -> bool:
	for attempt in range(3):
		if giver.is_empty():
			return false
		await click(giver.point, MOUSE_BUTTON_RIGHT)
		if await wait_quest(func(s): return s.frame_open and s.get("page") == "Greeting" and not s.greeting_quests.is_empty(), giver.name + " greeting", 5000, false):
			await frames(3)
			return true
		print("FIXTURE GREETING_RETRY ", giver.name, " at ", giver.point)
		giver = await find_unit(giver.name)
	fail("%s greeting never opened: %s" % [giver.get("name"), client.quest_state()])
	return false

func wait_quest(predicate: Callable, what: String, timeout_ms := WAIT_MS, required := true) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call(client.quest_state()):
			await frames(2)
			return true
	if required:
		fail("Timed out waiting for %s: %s" % [what, client.quest_state()])
	return false

func wait_frames(predicate: Callable, what: String, timeout_ms := WAIT_MS) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call():
			await frames(2)
			return true
	fail("Timed out waiting for " + what)
	return false

# --- world helpers -------------------------------------------------------------

func enter_world() -> bool:
	var deadline := Time.get_ticks_msec() + 60000
	var replied := false
	while Time.get_ticks_msec() < deadline and not replied:
		await process_frame
		var state: Dictionary = client.account_state()
		replied = state.reply_received
		if replied and (state.screen != "CharacterSelect" or state.character_count < 1):
			fail("Fixture needs an authenticated character: " + str(state))
			return false
	if not replied:
		fail("Timed out waiting for the login reply: " + str(client.account_state()))
		return false
	var enter = null
	deadline = Time.get_ticks_msec() + 30000
	while Time.get_ticks_msec() < deadline and not enter is Button:
		await process_frame
		var ui = client.get_node_or_null("CharacterSelectUI")
		enter = ui.find_child("EnterWorld", true, false) if ui != null else null
	await frames(10)
	if not enter is Button:
		fail("Enter World button missing")
		return false
	await click(enter.get_global_rect().get_center(), MOUSE_BUTTON_LEFT)
	deadline = Time.get_ticks_msec() + WORLD_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "InWorld" and state.local_player_position != null and not state.terrain.parsed_tiles.is_empty():
			print("FIXTURE IN_WORLD ", state.selected_character_name, " at ", state.local_player_position)
			await frames(60)
			return true
	fail("Timed out entering the world: " + str(client.account_state()))
	return false

func camera() -> Camera3D:
	return root.get_viewport().get_camera_3d()

func player_position() -> Vector3:
	return client.account_state().local_player_position

func units_named(name: String) -> Array:
	var units := client.get_node_or_null("WorldUnits")
	if units == null:
		return []
	# Sibling names are unique in Godot; the unit's own name is its meta.
	return units.get_children().filter(func(unit): return unit.get_meta("unit_name", "") == name)

func pick_point(unit: Node) -> Variant:
	var area := unit.find_child("UnitPick", true, false) as Area3D
	if area == null:
		return null
	return (area.get_child(0) as Node3D).global_position

## The named unit's server id once it is mirrored.
func locate(name: String) -> Dictionary:
	var deadline := Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		for unit in units_named(name):
			var area: Node = unit.find_child("UnitPick", true, false)
			if area != null:
				return {"id": area.get_meta("unit_server_id"), "name": name}
	fail("%s is not mirrored" % name)
	return {}

## The named unit's pick shape centre on screen, selected by the native ray.
func find_unit(name: String) -> Dictionary:
	var deadline := Time.get_ticks_msec() + 60000
	var attempt := 0
	while Time.get_ticks_msec() < deadline:
		await process_frame
		for unit in units_named(name):
			if pick_point(unit) == null or camera() == null:
				continue
			var id = unit.find_child("UnitPick", true, false).get_meta("unit_server_id")
			await face_unit(id)
			# Something in front (a lamp post, the player model) can cover it: look from
			# a little to either side, then step closer.
			var to: Vector3 = (unit as Node3D).global_position - player_position()
			var offsets := [0.0, 0.5, -0.5, 0.9, -0.9]
			await face_direction(atan2(to.x, to.z) + offsets[attempt % offsets.size()])
			var point := find_unit_point(unit, id)
			if point != Vector2.INF:
				return {"id": id, "name": name, "point": point}
			attempt += 1
			if attempt % offsets.size() == 0:
				await approach(id, 2.0)
	var names := []
	for unit in client.get_node("WorldUnits").get_children():
		names.append(str(unit.name))
	await capture("fail-find-unit.png")
	fail("%s is not visible and unoccluded; units %s" % [name, names])
	return {}

## A screen pixel the native ray picks as unit `id`, or `Vector2.INF`.
func find_unit_point(unit: Node, id: int) -> Vector2:
	var world_point = pick_point(unit)
	if world_point == null or not camera().is_position_in_frustum(world_point):
		return Vector2.INF
	var centre := camera().unproject_position(world_point)
	for dy in range(-72, 25, 8):
		for dx in range(-32, 33, 8):
			var point := centre + Vector2(dx, dy)
			if UnitPicker.pick(camera(), point) == id:
				return point
	return Vector2.INF

func unit_by_id(id: int) -> Node:
	var units := client.get_node_or_null("WorldUnits")
	if units == null:
		return null
	for unit in units.get_children():
		var area: Node = unit.find_child("UnitPick", true, false)
		if area != null and area.get_meta("unit_server_id") == id:
			return unit
	return null

func nearest_living(name: String) -> Dictionary:
	var best := {}
	var here := player_position()
	for unit in units_named(name):
		var area: Node = unit.find_child("UnitPick", true, false)
		if area == null:
			continue
		var id: int = area.get_meta("unit_server_id")
		var at: Vector3 = (unit as Node3D).global_position
		var distance := Vector2(at.x - here.x, at.z - here.z).length()
		if not client.unit_alive(id) or distance > HUNT_YARDS:
			continue
		if best.is_empty() or distance < best.distance:
			best = {"id": id, "distance": distance}
	return best

func yaw() -> float:
	return client.minimap_state().facing_yaw

## Turn with the turn keys until facing unit `id` (forward is (sin yaw, cos yaw)).
func face_unit(id: int) -> void:
	for attempt in range(120):
		var unit := unit_by_id(id)
		if unit == null:
			return
		var to: Vector3 = (unit as Node3D).global_position - player_position()
		var want := atan2(to.x, to.z)
		var diff := wrapf(want - yaw(), -PI, PI)
		if abs(diff) < 0.15:
			return
		var key := KEY_LEFT if diff > 0.0 else KEY_RIGHT
		push_key(key, true)
		await frames(1 if abs(diff) < 0.5 else 3)
		push_key(key, false)
		await frames(1)

## Walk to within `yards` of the engine point `target`.
func walk_to(target: Vector3, yards: float) -> bool:
	var deadline := Time.get_ticks_msec() + 45000
	while Time.get_ticks_msec() < deadline:
		var to: Vector3 = target - player_position()
		if Vector2(to.x, to.z).length() <= yards:
			push_key(KEY_W, false)
			await frames(5)
			return true
		await face_direction(atan2(to.x, to.z))
		push_key(KEY_W, true)
		await frames(6)
	push_key(KEY_W, false)
	return false

func face_direction(want: float) -> void:
	for attempt in range(120):
		var diff := wrapf(want - yaw(), -PI, PI)
		if abs(diff) < 0.15:
			return
		var key := KEY_LEFT if diff > 0.0 else KEY_RIGHT
		push_key(key, true)
		await frames(1 if abs(diff) < 0.5 else 3)
		push_key(key, false)
		await frames(1)

## Walk to within `yards` of unit `id`.
func approach(id: int, yards: float) -> bool:
	var deadline := Time.get_ticks_msec() + 45000
	while Time.get_ticks_msec() < deadline:
		var unit := unit_by_id(id)
		if unit == null:
			return false
		var to: Vector3 = (unit as Node3D).global_position - player_position()
		if Vector2(to.x, to.z).length() <= yards:
			push_key(KEY_W, false)
			await frames(5)
			return true
		await face_unit(id)
		push_key(KEY_W, true)
		await frames(6)
	push_key(KEY_W, false)
	return false

## Right-click the prey to start auto-attack and fight until its objective count rises
## or it dies.
func attack(id: int, objective_before: String) -> void:
	var deadline := Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		var unit := unit_by_id(id)
		if unit == null or not client.unit_alive(id):
			await frames(20)
			return
		var to: Vector3 = (unit as Node3D).global_position - player_position()
		if Vector2(to.x, to.z).length() > MELEE_YARDS + 1.5:
			if not await approach(id, MELEE_YARDS):
				return
		await face_unit(id)
		var world_point = pick_point(unit)
		if client.target_state().get("auto_attack") != id and world_point != null and camera().is_position_in_frustum(world_point):
			var target := await find_unit_point(unit, id)
			print("FIXTURE ATTACK ", id, " point ", target, " auto ", client.target_state().get("auto_attack"))
			if target != Vector2.INF:
				await click(target, MOUSE_BUTTON_RIGHT)
		await frames(20)
		var entry := log_entry(client.quest_state(), QUEST)
		if entry.objectives[0] != objective_before or entry.completed:
			print("FIXTURE KILL ", entry.objectives[0])
			await frames(20)
			return

func frames(count: int) -> void:
	for frame in range(count):
		await process_frame

func click(point: Vector2, button: MouseButton) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await process_frame
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = button
		event.button_mask = (1 << (button - 1)) if pressed else 0
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await frames(3)

func click_control(control: Control) -> void:
	if control == null:
		fail("Missing control to click")
		return
	await click(control.get_global_rect().get_center(), MOUSE_BUTTON_LEFT)

func capture(file: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var error := image.save_png(shots + file)
	if error != OK:
		fail("Could not save " + file + ": " + str(error))

func tap(code: Key) -> void:
	push_key(code, true)
	await process_frame
	push_key(code, false)
	await frames(2)

func push_key(code: Key, pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.pressed = pressed
	root.push_input(event, true)

func fail(message: String) -> void:
	push_error(message)
	quit(1)
