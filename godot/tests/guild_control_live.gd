extends "res://tests/guild_chat_live.gd"

## Two private clients; role a is the Guild Master, role b a Member. The Guild
## Master edits the Member rank only through the Guild Control UI; the Member
## proves each edit through what the server then allows:
##   bank tab view/deposit/stacks: a withdrawal, the daily limit and a deposit
##   refused after the deposit right is removed;
##   gold per day: a 1g withdrawal and the exhausted allowance;
##   Officer: officer chat delivered, then refused once the right is removed.
## Before either logs in, the Guild Master puts both characters into a fresh guild
## through the admin wrapper (Member rank: no tab rights, no gold, no officer chat).

const GUILD_NAME := "Vault Keepers"
const MEMBER_RANK := 3
const GRANT := 10
const PERMISSIONS := "You don't have permission to do that."
const WITHDRAW_LIMIT := "You cannot withdraw that much from the guild bank."
const OFFICER := "GuildControlUIRankSettingsFrameOfficerCheckbox"
const WITHDRAW_GOLD := "GuildControlUIRankSettingsFrameCheckbox16"

func run_completion() -> void:
	var success := await run_control()
	if success:
		print("PASS GUILD_CONTROL_LIVE role=", role, " skin=", skin)
		completion.case_done()
		client.free()
	completion.finish(success)

func run_control() -> bool:
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
	if role == "a" and not provision_guild():
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
	if not await open_vault():
		return false
	if role == "a" and not await stock_vault():
		return false
	if not await barrier("stocked"):
		return false
	return await bank_rights() and await gold_rights() and await officer_rights() and await barrier("done")

# --- provisioning ----------------------------------------------------------

func provision_guild() -> bool:
	var id := str(int(Time.get_unix_time_from_system()))
	var leader := OS.get_environment("BANK_CHARACTER")
	var member := OS.get_environment("GUILD_PEER_CHARACTER")
	for step in [
		["create-guild", leader, id, GUILD_NAME],
		["create-guild", member, id, GUILD_NAME],
		["set-guild-rank", member, str(MEMBER_RANK)],
		["set-gold", leader, "10000000"],
	]:
		if not admin(step):
			return false
	return true

## Buy tab 1, fund the guild with 5g, deposit a Linen stack and give the Member
## Linen of its own; the Guild Master then closes the vault.
func stock_vault() -> bool:
	await press("GuildBankTab1Button")
	if not await wait_until(func(): return shown("GuildBankFrameBuyInfoPurchaseButton"), "purchase offer"):
		return false
	await press("GuildBankFrameBuyInfoPurchaseButton")
	if not await wait_until(func(): return popup_text() != "", "tab confirmation"):
		return false
	await press("StaticPopup1Button1")
	if not await wait_until(func(): return client.bank_state().guild_tabs.size() == 1, "purchased tab"):
		return false
	if not await money_prompt("GuildBankFrameDepositButton", "GuildBankMoney", "GuildBankMoneyPopupAccept", 5, money() - 50000):
		return false
	var leader := OS.get_environment("BANK_CHARACTER")
	if not admin(["grant-item", leader, str(LINEN), str(GRANT)]) or not admin(["grant-item", OS.get_environment("GUILD_PEER_CHARACTER"), str(LINEN), str(GRANT)]):
		return false
	if not await wait_until(func(): return backpack_stack().get("count", 0) > 0, "granted Linen"):
		return false
	var linen := backpack_stack()
	await click_at(control("ContainerFrame0Slot%d" % linen.slot).get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
	if not await wait_until(func(): return slot_count() == linen.count, "Guild Master Linen deposit"):
		return false
	FileAccess.open(sync_path("linen"), FileAccess.WRITE).store_string(str(linen.count))
	return await close_vault()

# --- cases -----------------------------------------------------------------

## Guild Control bank pane: View, Deposit and 1 stack a day on tab 1.
func bank_rights() -> bool:
	var stack := int(FileAccess.get_file_as_string(sync_path("linen")))
	if role == "b" and not completion.require(not tab_rights().viewable, "Member starts without tab 1 view"):
		return false
	if role == "a":
		if not await open_guild_control(2):
			return false
		if not await toggle("GuildTabView0", true) or not await toggle("GuildTabDeposit0", true):
			return false
		if not await enter_number("GuildTabItems0", 1, "tab0", 1):
			return false
	if not await step("bank-rights-granted"):
		return false
	if role == "b":
		if not await wait_until(func(): return rights_are(true, true, 1) and slot_count() == stack, "open vault refreshed with granted tab rights"):
			return false
		var before := bag_count(LINEN)
		await click_at(control("GuildBankFrameItem1").get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
		if not await wait_until(func(): return slot_count() == 0 and bag_count(LINEN) == before + stack and rights_are(true, true, 0), "withdrawal uses the only daily stack"):
			return false
		var linen := backpack_stack()
		await click_at(control("ContainerFrame0Slot%d" % linen.slot).get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
		if not await wait_until(func(): return slot_count() == linen.count, "Member deposit"):
			return false
		await click_at(control("GuildBankFrameItem1").get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
		if not await refused(WITHDRAW_LIMIT, linen.count, "withdrawal past the daily limit"):
			return false
	if not await step("bank-limit"):
		return false
	if role == "a":
		if not await toggle("GuildTabDeposit0", false):
			return false
		if not admin(["grant-item", OS.get_environment("GUILD_PEER_CHARACTER"), str(LINEN), "5"]):
			return false
	if not await step("deposit-revoked"):
		return false
	if role == "b":
		var held := slot_count()
		if not await wait_until(func(): return rights_are(true, false, 0) and backpack_stack().get("count", 0) > 0, "deposit right removed"):
			return false
		var linen := backpack_stack()
		await click_at(control("ContainerFrame0Slot%d" % linen.slot).get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
		if not await refused(PERMISSIONS, held, "deposit without the deposit right"):
			return false
		if not completion.require(backpack_stack().get("count", 0) == linen.count, "Refused deposit kept the bag stack"):
			return false
	return await step("deposit-refused")

## Guild Control permissions pane: Withdraw Gold and 1 gold a day.
func gold_rights() -> bool:
	if role == "a":
		if not await select_page(1):
			return false
		if not await toggle(WITHDRAW_GOLD, true) or not await enter_number("GuildRankGold", 1, "gold", 10000):
			return false
	if not await step("gold-granted"):
		return false
	if role == "b":
		var withdraw := "GuildBankFrameWithdrawButton"
		if not await wait_until(func(): return shown(withdraw) and not (control(withdraw) as BaseButton).disabled, "Withdraw enabled by the granted allowance"):
			return false
		if not await money_prompt(withdraw, "GuildBankMoney", "GuildBankMoneyPopupAccept", 1, money() + 10000):
			return false
		if not await wait_until(func(): return (control(withdraw) as BaseButton).disabled, "1g allowance exhausted"):
			return false
	return await step("gold-used")

## Guild Control Officer checkbox: officer chat delivered, then refused.
func officer_rights() -> bool:
	if role == "b":
		if not await close_vault() or not await open_communities_chat():
			return false
		await press("CommunitiesStreamDropdown")
		if not completion.require(not shown("CommunitiesOfficerChatChoice"), "Member starts without officer chat"):
			return false
		await press("CommunitiesStreamDropdown")
	else:
		if not await toggle(OFFICER, true):
			return false
		await press("GuildControlUICloseButton")
		if not await wait_until(func(): return not shown("GuildControlUI"), "Guild Control closed"):
			return false
		await press("CommunitiesChatTab")
		if not await wait_until(func(): return shown("CommunitiesChatEditBox"), "Guild Master chat"):
			return false
		if not await select_officer_stream():
			return false
	if not await step("officer-granted"):
		return false
	if role == "b":
		if not await select_officer_stream() or not await compose(message("officer-member")):
			return false
	elif not await received(message("officer-member")):
		return false
	if not await step("officer-delivered"):
		return false
	if role == "a":
		await press("GuildRosterTab")
		if not await open_guild_control(1) or not await toggle(OFFICER, false):
			return false
	if not await step("officer-revoked"):
		return false
	if role == "b":
		if not await wait_until(func(): return text("CommunitiesStreamDropdownText") == "Guild Chat", "officer stream removed"):
			return false
		if not await main_chat("/o " + message("officer-refused")):
			return false
		if not await wait_until(func(): return text("CommunitiesChatError") == PERMISSIONS, "officer chat refused"):
			return false
	return await step("officer-refused")

# --- Guild Control ---------------------------------------------------------

## Communities → Guild Settings → page, with the Member rank selected.
func open_guild_control(page: int) -> bool:
	if not shown("CommunitiesFrame"):
		await open_communities()
	if not await wait_until(func(): return shown("GuildControlButton"), "Guild Settings button"):
		return false
	await press("GuildControlButton")
	if not await wait_until(func(): return shown("GuildControlUI"), "Guild Control"):
		return false
	return await select_page(page)

func select_page(page: int) -> bool:
	await press("GuildControlUINavigationDropdown")
	if not await wait_until(func(): return shown("GuildControlPage%d" % page), "navigation menu"):
		return false
	await press("GuildControlPage%d" % page)
	var prefix := "GuildControlUIRankBankFrame" if page == 2 else "GuildControlUIRankSettingsFrame"
	if not await wait_until(func(): return shown(prefix + "RankDropdown"), "page %d" % page):
		return false
	await press(prefix + "RankDropdown")
	if not await wait_until(func(): return shown("GuildControlRankChoice%d" % MEMBER_RANK), "rank menu"):
		return false
	await press("GuildControlRankChoice%d" % MEMBER_RANK)
	return await wait_until(func(): return text(prefix + "RankDropdownText") == "Member", "Member rank selected")

## Click a checkbox and wait for the server's state to show it.
func toggle(name: String, checked: bool) -> bool:
	if not completion.require(shown(name + "Check") != checked, name + " already in the requested state"):
		return false
	await press(name)
	return await wait_until(func(): return member_settings_settled() and shown(name + "Check") == checked, "%s %s by the server" % [name, "set" if checked else "cleared"])

## Type a whole number and press Enter, then wait until the server's Member
## settings hold `saved` (copper for gold, stacks for a tab).
func enter_number(name: String, value: int, field: String, saved: int) -> bool:
	# Guild Control draws a read-only Label under the same name until the rank may edit it.
	if not await wait_until(func(): return control(name) is LineEdit and shown(name), name + " editable"):
		return false
	await press(name)
	var edit := control(name) as LineEdit
	edit.select_all()
	await tap(KEY_BACKSPACE)
	await type_text(str(value))
	await tap(KEY_ENTER)
	return await wait_until(func(): return member_settings_settled() and member_setting(field) == saved, "%s saved as %d" % [name, value])

## `gold_per_day`, or `tab0` for tab 1's stacks per day, as the server last sent them.
func member_setting(field: String) -> int:
	var observed: Dictionary = JSON.parse_string(str(client.guild_ranks_observation()))
	if observed.ranks == null or observed.ranks.size() <= MEMBER_RANK:
		return -1
	var rank: Dictionary = observed.ranks[MEMBER_RANK]
	return int(rank.tabs[0].withdrawals_per_day) if field == "tab0" else int(rank.gold_per_day)

## No Guild Control write is pending, so the shown Member settings are the server's.
func member_settings_settled() -> bool:
	var observed: Dictionary = JSON.parse_string(str(client.guild_ranks_observation()))
	return not observed.awaiting

# --- Communities -----------------------------------------------------------

func open_communities() -> void:
	if shown("GuildMicroButton"):
		await press("GuildMicroButton")
	else:
		await press("MinimapLauncherButton")
		await press("LauncherEntryGuildMicroButton")

func open_communities_chat() -> bool:
	await open_communities()
	if not await wait_until(func(): return shown("CommunitiesChatTab"), "Communities"):
		return false
	await press("CommunitiesChatTab")
	return await wait_until(func(): return shown("CommunitiesChatEditBox"), "guild composer")

func select_officer_stream() -> bool:
	await press("CommunitiesStreamDropdown")
	if not await wait_until(func(): return shown("CommunitiesOfficerChatChoice"), "officer stream offered"):
		return false
	await press("CommunitiesOfficerChatChoice")
	return await wait_until(func(): return text("CommunitiesStreamDropdownText") != "Guild Chat", "officer stream selected")

# --- vault -----------------------------------------------------------------

func close_vault() -> bool:
	for attempt in range(3):
		if not shown("GuildBankFrame"):
			return true
		await tap(KEY_ESCAPE)
		await wait_frames(10)
	return await wait_until(func(): return not shown("GuildBankFrame"), "vault closed")

func refused(message: String, count: int, what: String) -> bool:
	if not await wait_until(func(): return client.bank_state().guild_error == message, "refusal of " + what):
		return false
	await wait_frames(30)
	return completion.require(slot_count() == count, "Refused %s changed tab 1" % what)

func tab_rights() -> Dictionary:
	var tabs: Array = client.bank_state().guild_tabs
	return tabs[0] if not tabs.is_empty() else {}

func rights_are(viewable: bool, can_deposit: bool, remaining: int) -> bool:
	var rights := tab_rights()
	return rights.get("viewable") == viewable and rights.get("can_deposit") == can_deposit and rights.get("remaining") == remaining

func slot_count() -> int:
	for entry in client.bank_state().guild_slots:
		if entry.tab == 0 and entry.slot == 0:
			return entry.count
	return 0

func backpack_stack() -> Dictionary:
	var best := {}
	for entry in client.merchant_state().bags:
		if entry.item_id == LINEN and entry.bag == 0 and entry.count > best.get("count", 0):
			best = entry
	return best

# --- shared ----------------------------------------------------------------

## Both clients reach `name`, then capture it.
func step(name: String) -> bool:
	return await barrier(name) and await capture_checked(name)

func admin(arguments: Array) -> bool:
	var output := []
	var code := OS.execute(OS.get_environment("BANK_ADMIN"), arguments, output, true)
	print("FIXTURE ADMIN ", arguments, " ", code, " ", output)
	return code == 0 or fail("admin %s failed: %s" % [arguments, output])

func sync_path(name: String) -> String:
	return "%s/%s" % [sync_dir, name]
