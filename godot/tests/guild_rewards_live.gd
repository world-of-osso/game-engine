extends "res://tests/guild_control_live.gd"

## Two private clients in one fresh guild; role a is the Guild Master, role b a Member.
## The Guild Master turns in a real quest: the gain shows in chat and on the Benefits
## reputation bar, while the Member's own bar stays put (guild reputation is per character).
## Shay Pressler then gates a Friendly heirloom helm: refused at Neutral, sold for its
## guild reward cost once admin sets Friendly, and its Benefits row drops "Requires:
## Friendly". Both clients then read the purchase in guild News.

const GUILD_FACTION := 1168
const FRIENDLY := 3000
## The list's first rows: Friendly heirloom helms at 1500g, unique, so each run buys the
## first one the Guild Master does not own yet.
const HELMS := [
	{"id": 61931, "name": "Polished Helm of Valor"},
	{"id": 61935, "name": "Tarnished Raging Berserker's Helm"},
	{"id": 61936, "name": "Mystical Coif of Elements"},
	{"id": 61937, "name": "Stained Shadowcraft Cap"},
	{"id": 61942, "name": "Preened Tribal War Feathers"},
	{"id": 61958, "name": "Tattered Dreadmist Mask"},
]
const HELM_COST := 15000000
const GOLD := 20000000
const QUEST_LEVEL := 10
const VENDOR := "Shay Pressler"
const VENDOR_AT := ["-8886.96", "608.32", "95.34"]
const REFUSED := "Your guild reputation isn't high enough to do that."
const BAR := "CommunitiesGuildFactionBar"
## Any-race Stormwind deliveries without a prerequisite (ContentTuning 73, levels 1-30),
## tried in order so a repeated run takes a quest the Guild Master has not turned in.
const QUESTS := [
	{"id": 332, "giver": "Renato Gallina", "at": ["-8847.26", "614.98", "95.27"], "ender": "Suzetta Gallina", "to": ["-8883.88", "675.55", "100.26"]},
	{"id": 333, "giver": "Harlan Bagley", "at": ["-8781.9", "640.19", "97.41"], "ender": "Rema Schneider", "to": ["-8776.6", "713.5", "99.62"]},
	{"id": 334, "giver": "Rema Schneider", "at": ["-8776.6", "713.5", "99.62"], "ender": "Thurman Schneider", "to": ["-8960.84", "809.21", "109.63"]},
	{"id": 1097, "giver": "Smith Argus", "at": ["-9455.65", "87.86", "58.43"], "ender": "Grimand Elmore", "to": ["-8386.14", "685.2", "95.36"]},
	{"id": 239, "giver": "Marshal Dughan", "at": ["-9464.52", "74.01", "56.78"], "ender": "Deputy Rainer", "to": ["-9661.75", "694.32", "36.94"]},
	{"id": 2158, "giver": "Falkhaan Isenstrider", "at": ["-9072.71", "-39.39", "87.86"], "ender": "Innkeeper Farley", "to": ["-9461.66", "16.19", "57.05"]},
]

var gained := 0
var helm: Dictionary = HELMS[0]
var bar_shots := 0

func run_completion() -> void:
	var success := await run_rewards()
	if success:
		print("PASS GUILD_REWARDS_LIVE role=", role, " skin=", skin)
		completion.case_done()
		client.free()
	completion.finish(success)

func run_rewards() -> bool:
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
	if role == "a" and not provision_rewards_guild():
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
	if role == "a" and not choose_helm():
		return false
	return await neutral() and await earned() and await vendor_gate() and await purchase_news() and await barrier("done")

func choose_helm() -> bool:
	for candidate in HELMS:
		if bag_count(candidate.id) == 0:
			helm = candidate
			print("FIXTURE HELM ", helm)
			return true
	return fail("The Guild Master owns every Friendly helm: %s" % [HELMS])

## Fresh guild of both characters at Neutral guild standing; the Guild Master at a level
## the quests reward and with gold for the helm.
func provision_rewards_guild() -> bool:
	var leader := OS.get_environment("BANK_CHARACTER")
	var member := OS.get_environment("GUILD_PEER_CHARACTER")
	var id := str(int(Time.get_unix_time_from_system()))
	for step in [
		["create-guild", leader, id, GUILD_NAME],
		["create-guild", member, id, GUILD_NAME],
		["set-guild-rank", member, str(MEMBER_RANK)],
		["set-level", leader, str(QUEST_LEVEL)],
		["set-gold", leader, str(GOLD)],
		["set-reputation", leader, str(GUILD_FACTION), "0"],
		["set-reputation", member, str(GUILD_FACTION), "0"],
	]:
		if not admin(step):
			return false
	return true

# --- cases -----------------------------------------------------------------

## Both bars start Neutral at 0/3,000; the helm needs Friendly.
func neutral() -> bool:
	if not await benefits("Neutral", 0, FRIENDLY):
		return false
	if role == "a" and not await wait_until(func(): return text(helm_row() + "SubTextStanding") == "Friendly", "helm requires Friendly"):
		return false
	return await step("neutral")

## The Guild Master turns in a quest; only its own bar moves.
func earned() -> bool:
	if role == "a" and not await turn_in_quest():
		return false
	if not await step("quest-turned-in"):
		return false
	if not await benefits("Neutral", gained if role == "a" else 0, FRIENDLY):
		return false
	return await step("earned")

## Shay Pressler refuses the helm at Neutral and sells it at Friendly.
func vendor_gate() -> bool:
	if role == "a":
		await close_communities()
		if not admin(["set-position", OS.get_environment("BANK_CHARACTER")] + VENDOR_AT):
			return false
		if not await open_vendor() or not await buy_helm(false):
			return false
	if not await step("vendor-refused"):
		return false
	if role == "a":
		await press("MerchantFrameCloseButton")
		if not await wait_until(func(): return not client.merchant_state().open, "merchant closed"):
			return false
		if not admin(["set-reputation", OS.get_environment("BANK_CHARACTER"), str(GUILD_FACTION), str(FRIENDLY)]):
			return false
		if not await open_vendor() or not await buy_helm(true):
			return false
		await press("MerchantFrameCloseButton")
		if not await wait_until(func(): return not client.merchant_state().open, "merchant closed"):
			return false
		if not await benefits("Friendly", 0, 6000):
			return false
		if not await wait_until(func(): return shown(helm_row()) and not shown(helm_row() + "SubTextStanding"), "helm row without a standing requirement"):
			return false
	return await step("vendor-bought")

# --- quest -----------------------------------------------------------------

## Accept the first candidate quest the server still offers, deliver it, and read the
## chat gain line.
func turn_in_quest() -> bool:
	var leader := OS.get_environment("BANK_CHARACTER")
	for quest in QUESTS:
		if not admin(["set-position", leader] + quest.at):
			return false
		if not await npc_request(func(): return client.accept_quest_from(quest.giver, quest.id), quest.giver):
			return false
		if not await within(func(): return in_log(quest.id), 10000):
			print("FIXTURE QUEST ", quest.id, " not offered; trying the next")
			continue
		if not admin(["set-position", leader] + quest.to):
			return false
		var lines_before: int = client.quest_state().system_lines.size()
		if not await npc_request(func(): return client.turn_in_quest_to(quest.ender, quest.id), quest.ender):
			return false
		if not await wait_until(func(): return not in_log(quest.id) and gain_line(lines_before) > 0, "guild reputation for quest %d" % quest.id):
			return false
		gained = gain_line(lines_before)
		print("FIXTURE QUEST ", quest.id, " gained ", gained)
		return completion.require(gained == 166 or gained == 167, "5.0.4 quest gain is 166 or 167, not %d" % gained)
	return fail("Every candidate quest is already turned in: %s" % [client.quest_state().log])

## Repeat an NPC request until the NPC is mirrored after a teleport.
func npc_request(request: Callable, npc: String) -> bool:
	return await wait_until(func(): return request.call() == "", "mirrored " + npc)

func in_log(quest_id: int) -> bool:
	for entry in client.quest_state().log:
		if entry.quest_id == quest_id:
			return true
	return false

## The guild gain of the newest system line after `from`; 0 without one.
func gain_line(from: int) -> int:
	var lines: Array = client.quest_state().system_lines
	# FACTION_STANDING_INCREASED.
	var pattern := RegEx.create_from_string("^Reputation with Guild increased by (\\d+)\\.$")
	for index in range(lines.size() - 1, from - 1, -1):
		var found := pattern.search(lines[index])
		if found != null:
			return int(found.get_string(1))
	return 0

## True once `predicate` holds within `timeout_ms`; no failure.
func within(predicate: Callable, timeout_ms: int) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call():
			return true
	return false

# --- vendor ----------------------------------------------------------------

func open_vendor() -> bool:
	if not await npc_request(func(): return client.interact_npc_named(VENDOR), VENDOR):
		return false
	if not await wait_until(func(): return client.merchant_state().open and client.merchant_state().items.has(helm.name), VENDOR + "'s list with the helm"):
		return false
	var index: int = Array(client.merchant_state().items).find(helm.name)
	for page in range(index / 10):
		await press("MerchantNextPageButton")
	return await wait_until(func(): return text(helm_cell() + "Name") == helm.name, "helm on its merchant page")

func helm_cell() -> String:
	return "MerchantItem%d" % (Array(client.merchant_state().items).find(helm.name) % 10 + 1)

func helm_row() -> String:
	return "CommunitiesGuildReward%d" % helm.id

## Right-click the helm: refused without spending, or bought for its price.
func buy_helm(sold: bool) -> bool:
	var before: Dictionary = client.merchant_state()
	await click_at(control(helm_cell()).get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
	if sold:
		return await wait_until(func(): return bag_count(helm.id) == 1 and money() == before.money - HELM_COST, "helm bought for its guild reward cost")
	if not await wait_until(func(): return error_shown(REFUSED), "helm refused below Friendly"):
		return false
	await wait_frames(30)
	return completion.require(bag_count(helm.id) == 0 and money() == before.money, "refusal kept the helm and the gold")

func error_shown(value: String) -> bool:
	for index in range(1, 6):
		if text("UIErrorsFrameLine%d" % index) == value:
			return true
	return false

# --- Benefits --------------------------------------------------------------

func close_communities() -> void:
	if shown("CommunitiesGuildBenefitsTab"):
		await open_communities()
		await wait_until(func(): return not shown("CommunitiesGuildBenefitsTab"), "Communities closed")

## The Benefits tab shows `standing` on the bar; hovered, the bar shows `value / maximum`
## (GUILD_EXPERIENCE_LABEL) and its tooltip GUILD_EXPERIENCE_CURRENT.
func benefits(standing: String, value: int, maximum: int) -> bool:
	if not shown("CommunitiesGuildBenefitsTab"):
		await open_communities()
		if not await wait_until(func(): return shown("CommunitiesGuildBenefitsTab"), "Communities"):
			return false
	await press("CommunitiesGuildBenefitsTab")
	await hover_at(Vector2.ZERO)
	if not await wait_until(func(): return shown(helm_row()) and text(BAR + "Label") == standing, "rewards list and %s bar" % standing):
		return false
	var current := "Current: %s/%s (%d%%)" % [grouped(value), grouped(maximum), ceili(value * 100.0 / maximum)]
	var hovered := "%s / %s" % [grouped(value), grouped(maximum)]
	await hover_at(control(BAR).get_global_rect().get_center())
	if not await wait_until(func(): return bar_tooltip_has(current) and text(BAR + "Label") == hovered, "hovered bar %s and tooltip %s" % [hovered, current]):
		return false
	bar_shots += 1
	await capture_checked("bar%d-%s-%d" % [bar_shots, standing.to_lower(), value])
	await hover_at(Vector2.ZERO)
	return await wait_until(func(): return text(BAR + "Label") == standing, "bar label back to " + standing)

func bar_tooltip_has(line: String) -> bool:
	var tooltip: Dictionary = client.tooltip_state()
	return tooltip.get("visible", false) and tooltip.get("title", "") == "Guild Reputation" and str(tooltip.get("lines", [])).contains(line)

func hover_at(point: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	Input.warp_mouse(point)
	await wait_frames(5)

## BreakUpLargeNumbers.
func grouped(value: int) -> String:
	var digits := str(value)
	var out := ""
	for index in range(digits.length()):
		if index > 0 and (digits.length() - index) % 3 == 0:
			out += ","
		out += digits[index]
	return out

# --- News --------------------------------------------------------------------

## Both clients read the Guild Master's helm purchase (TC Player.cpp:23904: above epic
## quality) in the guild News on the Info tab.
func purchase_news() -> bool:
	if not shown("GuildInfoTab"):
		await open_communities()
		if not await wait_until(func(): return shown("GuildInfoTab"), "Communities"):
			return false
	await press("GuildInfoTab")
	var leader := OS.get_environment("BANK_CHARACTER")
	if not await wait_until(func(): return purchase_row(leader) != "", "purchased helm news by " + leader):
		return false
	print("FIXTURE NEWS ", purchase_row(leader), " ", text(purchase_row(leader)))
	return await step("news-purchased")

## The drawn News row of a helm purchase by `member`; "" without one.
func purchase_row(member: String) -> String:
	var ui := client.get_node_or_null("GuildRanksUI")
	if ui == null:
		return ""
	var raw := str(ui.call("guild_news_observation"))
	var envelope = {} if raw.is_empty() else JSON.parse_string(raw)
	if envelope == null or envelope.is_empty() or envelope.get("reply") == null:
		return ""
	for entry in envelope.reply.get("entries", []):
		var purchase = entry.kind.get("ItemPurchased") if entry.kind is Dictionary else null
		if purchase != null and purchase.member == member and HELMS.any(func(candidate): return candidate.id == int(purchase.item_id)):
			var row := "CommunitiesGuildNewsEntry%d" % int(entry.id)
			return row if shown(row) else ""
	return ""
