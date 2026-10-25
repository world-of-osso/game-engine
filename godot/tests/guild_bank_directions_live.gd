extends "res://tests/guild_member_notes_live.gd"

## Both actors perform actual deposit and withdrawal in each skin. Fresh private
## Vault Keepers guild: leader has Linen10, peer has none, peer tab rights2/day and1g.
func run_completion() -> void:
	var success := await run_bank_directions()
	if success:
		print("PASS GUILD_BANK_DIRECTIONS_NATIVE role=", role, " skin=", client.account_state().ui_skin)
		completion.case_done()
		client.free()
	completion.finish(success)

func run_bank_directions() -> bool:
	root.size = Vector2i(1920, 1080)
	Engine.max_fps = 30
	role = OS.get_environment("BANK_ROLE")
	sync_dir = OS.get_environment("BANK_SYNC")
	shots = OS.get_environment("BANK_SHOTS")
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await wait_frames(30)
	var error: String = client.connect_account(OS.get_environment("GODOT_TEST_SERVER"), OS.get_environment("BANK_ACCOUNT"), PASSWORD, false)
	if error != "":
		return fail(error)
	if not await enter_world(OS.get_environment("BANK_CHARACTER")) or not await select_skin(OS.get_environment("GUILD_BANK_SKIN")):
		return false
	if not completion.require(bag_count(LINEN) == (10 if role == "a" else 0), "Staged real Linen inventory must match the scenario"):
		return false
	if not await open_vault() or not await guild_bank_shared():
		return false
	if not await press_checked("GuildBankFrameTab1"):
		return false
	if not await wait_until(func(): return shown("GuildBankFrameItem1"), "bank items after viewing logs"):
		return false
	# The original live fixture proves leader deposit and member withdrawal.
	# Reverse the item move to close the missing action-by-skin cells.
	if role == "b":
		if not await deposit_from_bags():
			return false
	if not await barrier("reverse-deposit"):
		return false
	if not await wait_until(func(): return text("GuildBankFrameItem1Count") == "10", "member deposit broadcast"):
		return false
	if not await capture_checked("member-deposited"):
		return false
	if not await barrier("reverse-deposit-observed"):
		return false
	if role == "a":
		await click_at(control("GuildBankFrameItem1").get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
		if not await wait_until(func(): return bag_count(LINEN) == 10 and text("GuildBankFrameItem1Count") == "", "leader actual item withdrawal"):
			return false
	if not await barrier("reverse-withdraw"):
		return false
	if not await wait_until(func(): return text("GuildBankFrameItem1Count") == "", "leader withdrawal broadcast"):
		return false
	if not await capture_checked("leader-withdrew"):
		return false
	if role == "b":
		var before := money()
		if not await money_prompt("GuildBankFrameWithdrawButton", "GuildBankMoney", "GuildBankMoneyPopupAccept", 1, before + 10000):
			return false
		if not await wait_until(func(): return (control("GuildBankFrameWithdrawButton") as BaseButton).disabled, "daily gold allowance exhausted"):
			return false
		if not await capture_checked("member-daily-limit"):
			return false
		if not await money_prompt("GuildBankFrameDepositButton", "GuildBankMoney", "GuildBankMoneyPopupAccept", 1, before):
			return false
		if not completion.require((control("GuildBankFrameWithdrawButton") as BaseButton).disabled, "Depositing money cannot reset a consumed daily allowance"):
			return false
		if not await capture_checked("member-money-deposited"):
			return false
	return await barrier("all-directions-done")
