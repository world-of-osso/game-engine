extends "res://tests/guild_control_live.gd"

## Two private clients in one fresh guild; role a is the Guild Master, role b a Member.
## The Guild Master turns in a real quest: the gain shows in chat and on the Benefits
## reputation bar, while the Member's own bar stays put (guild reputation is per character).
## Shay Pressler then gates the Friendly helm, the list's first row: refused at Neutral,
## sold once admin sets Friendly, and the Benefits row drops its "Requires: Friendly" line.

const GUILD_FACTION := 1168
const FRIENDLY := 3000
const HELM := 61931
const HELM_NAME := "Polished Helm of Valor"
const HELM_COST := 15000000
const GOLD := 20000000
const QUEST_LEVEL := 10
const VENDOR := "Shay Pressler"
const VENDOR_AT := ["-8886.96", "608.32", "95.34"]
const REFUSED := "You don't have the required reputation."
const HELM_ROW := "CommunitiesGuildReward61931"
const BAR := "CommunitiesGuildFactionBar"
## Any-race Stormwind deliveries without a prerequisite (ContentTuning 73, levels 1-30),
## tried in order so a repeated run takes a quest the Guild Master has not turned in.
const QUESTS := [
	{"id": 332, "giver": "Renato Gallina", "at": ["-8847.26", "614.98", "95.27"], "ender": "Suzetta Gallina", "to": ["-8883.88", "675.55", "100.26"]},
	{"id": 333, "giver": "Harlan Bagley", "at": ["-8781.9", "640.19", "97.41"], "ender": "Rema Schneider", "to": ["-8776.6", "713.5", "99.62"]},
	{"id": 334, "giver": "Rema Schneider", "at": ["-8776.6", "713.5", "99.62"], "ender": "Thurman Schneider", "to": ["-8960.84", "809.21", "109.63"]},
]

var gained := 0

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
	return await neutral() and await earned() and await vendor_gate() and await barrier("done")

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
	if not await benefits("Neutral", "Current: 0/3,000 (0%)"):
		return false
	if role == "a" and not await wait_until(func(): return text(HELM_ROW + "SubTextStanding") == "Friendly", "helm requires Friendly"):
		return false
	return await step("neutral")

## The Guild Master turns in a quest; only its own bar moves.
func earned() -> bool:
	if role == "a" and not await turn_in_quest():
		return false
	if not await step("quest-turned-in"):
		return false
	var expected := "Current: %d/3,000 (%d%%)" % [gained, ceili(gained * 100.0 / FRIENDLY)] if role == "a" else "Current: 0/3,000 (0%)"
	if not await benefits("Neutral", expected):
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
		if not await benefits("Friendly", "Current: 0/6,000 (0%)"):
			return false
		if not await wait_until(func(): return shown(HELM_ROW) and not shown(HELM_ROW + "SubTextStanding"), "helm row without a standing requirement"):
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
	var pattern := RegEx.create_from_string("^gained (\\d+) reputation with Guild$")
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
	if not await wait_until(func(): return client.merchant_state().open and client.merchant_state().items.has(HELM_NAME), VENDOR + "'s list with the helm"):
		return false
	var index: int = Array(client.merchant_state().items).find(HELM_NAME)
	for page in range(index / 10):
		await press("MerchantNextPageButton")
	return await wait_until(func(): return text(helm_cell() + "Name") == HELM_NAME, "helm on its merchant page")

func helm_cell() -> String:
	return "MerchantItem%d" % (Array(client.merchant_state().items).find(HELM_NAME) % 10 + 1)

## Right-click the helm: refused without spending, or bought for its price.
func buy_helm(sold: bool) -> bool:
	var before: Dictionary = client.merchant_state()
	await click_at(control(helm_cell()).get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
	if sold:
		return await wait_until(func(): return bag_count(HELM) == 1 and money() == before.money - HELM_COST, "helm bought for its price")
	if not await wait_until(func(): return error_shown(REFUSED), "helm refused below Friendly"):
		return false
	await wait_frames(30)
	return completion.require(bag_count(HELM) == 0 and money() == before.money, "refusal kept the helm and the gold")

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

## The Benefits tab shows `standing` on the bar, and its tooltip `current`.
func benefits(standing: String, current: String) -> bool:
	if not shown("CommunitiesGuildBenefitsTab"):
		await open_communities()
		if not await wait_until(func(): return shown("CommunitiesGuildBenefitsTab"), "Communities"):
			return false
	await press("CommunitiesGuildBenefitsTab")
	if not await wait_until(func(): return shown(HELM_ROW) and text(BAR + "Label") == standing, "rewards list and %s bar" % standing):
		return false
	return await wait_until(func(): return bar_tooltip_has(current), "reputation tooltip " + current)

func bar_tooltip_has(line: String) -> bool:
	var point := control(BAR).get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	Input.warp_mouse(point)
	var tooltip: Dictionary = client.tooltip_state()
	return tooltip.get("visible", false) and tooltip.get("title", "") == "Guild Reputation" and str(tooltip.get("lines", [])).contains(line)
