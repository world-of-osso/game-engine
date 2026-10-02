extends "res://tests/bank_live.gd"

## Guild bank repair and full vendor item tooltips against a private server
## (docs/specs/merchant-frame.md). One Guild Master funds the Stormwind Guild Vault, is
## moved (admin set-position) to Janos Hammerknuckle, the Northshire weaponsmith who
## repairs, hovers a weapon (full item tooltip, Shift comparison with the equipped sword),
## repairs with the guild bank, and reads the repair in the vault's Money Log. Environment:
##   GODOT_TEST_SERVER   private server address
##   BANK_CHARACTER      the Guild Master (account BANK_ACCOUNT, password fbtest)
##   BANK_SHOTS          screenshot directory
##   BANK_ADMIN          game-server-admin binary
## Real input only; every result is the server's.

const VENDOR := "Janos Hammerknuckle"
# bank_live.gd's Stormwind vault spot; Janos stands at -8909.5,-104.2,82.0.
const STORMWIND_VAULT := ["-8928.5", "614.0", "100.6"]
const AT_JANOS := ["-8907.0", "-106.5", "82.6"]
const DEPOSIT_GOLD := 10
const ITEM_REPAIR := 7994

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	role = "g"
	shots = OS.get_environment("BANK_SHOTS")
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(OS.get_environment("GODOT_TEST_SERVER"), OS.get_environment("BANK_ACCOUNT"), PASSWORD, false)
	if error != "":
		fail("connect: " + error)
		return
	if not await enter_world(OS.get_environment("BANK_CHARACTER")):
		return
	for step in [Callable(self, "fund_guild"), Callable(self, "open_vendor"), Callable(self, "weapon_tooltip"), Callable(self, "guild_repair"), Callable(self, "money_log")]:
		if not await step.call():
			return
	print("FIXTURE MERCHANT_GUILD_LIVE_DONE")
	client.free()
	quit(0)

func move_to(spot: Array) -> bool:
	var output := []
	var code := OS.execute(OS.get_environment("BANK_ADMIN"), ["set-position", OS.get_environment("BANK_CHARACTER")] + spot, output, true)
	print("FIXTURE MOVE ", spot, " ", code, " ", output)
	if code != 0:
		return fail("set-position failed")
	var target := Vector2(float(spot[0]), float(spot[1]))
	# The client mirrors WoW x,y as Godot x,-z.
	if not await wait_until(func():
		var state: Dictionary = client.account_state()
		var at = state.local_player_position
		return at != null and Vector2(at.x, -at.z).distance_to(target) < 2.0 and state.terrain.pending_count == 0, "arrival at %s" % [spot]):
		return false
	await wait_frames(120)
	print("FIXTURE AT ", client.account_state().local_player_position)
	return true

func open_vault_at_stormwind() -> bool:
	if not await move_to(STORMWIND_VAULT):
		return false
	if not await click_pickable(func(node): return node.has_meta("game_object_name") and str(node.get_meta("game_object_name")) == VAULT, "game_object_server_id"):
		return false
	return await wait_until(func(): return shown("GuildBankFrame") and embedded_backpack_shown(), "guild bank open")

## The Guild Master deposits 10g: the only guild money the repair can use.
func fund_guild() -> bool:
	if not await open_vault_at_stormwind():
		return false
	if not await money_prompt("GuildBankFrameDepositButton", "GuildBankMoney", "GuildBankMoneyPopupAccept", DEPOSIT_GOLD, money() - DEPOSIT_GOLD * 10000):
		return false
	await capture("1-guild-funded")
	print("FIXTURE GUILD_FUNDED money=", money())
	await tap(KEY_ESCAPE)
	return await wait_until(func(): return not shown("GuildBankFrame"), "vault closed")

func open_vendor() -> bool:
	if not await move_to(AT_JANOS):
		return false
	if not await click_pickable(func(node): return str(node.name) == VENDOR, "unit_server_id"):
		return false
	if not await wait_until(func(): return client.merchant_state().open and client.merchant_state().vendor_name == VENDOR, VENDOR + " frame"):
		return false
	var state: Dictionary = client.merchant_state()
	print("FIXTURE VENDOR_OPEN items=", state.items, " money=", state.money, " repair_cost=", state.repair_cost, " guild_repair_money=", state.guild_repair_money)
	if state.repair_cost <= 0 or state.guild_repair_money != DEPOSIT_GOLD * 10000:
		return fail("Expected damaged gear and the Guild Master's 10g: " + str(state))
	if not shown("MerchantGuildBankRepairButton"):
		return fail("MerchantGuildBankRepairButton not shown")
	await capture("2-janos-guild-repair-button")
	return true

## MerchantItem1 (Shortsword): SetMerchantItem's full tooltip, then Shift compares it with
## the equipped Worn Shortsword.
func weapon_tooltip() -> bool:
	var cell := control("MerchantItem1")
	await hover(cell.get_global_rect().get_center())
	if not await wait_until(func(): return client.tooltip_state().visible and client.tooltip_state().title == client.merchant_state().items[0], "weapon tooltip"):
		return false
	var lines: PackedStringArray = client.tooltip_state().lines
	print("FIXTURE WEAPON_TOOLTIP ", client.tooltip_state().title, " ", lines)
	if not (has_line(lines, " Damage|Speed ") and has_line(lines, "damage per second") and has_line(lines, "Durability ") and has_line(lines, "Sell Price:") and has_line(lines, "Item ID: ")):
		return fail("Vendor weapon tooltip lacks full stats: " + str(lines))
	await capture("3-weapon-tooltip")
	await push_shift(true)
	var compared := await wait_until(func(): return client.tooltip_state().shopping.size() >= 1, "Shift comparison")
	if compared:
		var shopping: Dictionary = client.tooltip_state().shopping[0]
		print("FIXTURE WEAPON_COMPARE ", shopping.header, " ", shopping.title, " ", shopping.lines)
		await capture("4-weapon-compare")
	await push_shift(false)
	return compared

func guild_repair() -> bool:
	var before: Dictionary = client.merchant_state()
	await hover(control("MerchantGuildBankRepairButton").get_global_rect().get_center())
	if not await wait_until(func(): return client.tooltip_state().visible and client.tooltip_state().title == "Repair All Items", "guild repair tooltip"):
		return false
	print("FIXTURE GUILD_REPAIR_TOOLTIP ", client.tooltip_state().lines)
	await capture("5-guild-repair-tooltip")
	var sounds := sound_count(ITEM_REPAIR)
	print("FIXTURE REQUEST RepairItem guild_bank=true cost=", before.repair_cost)
	await press("MerchantGuildBankRepairButton")
	if not await wait_until(func(): return client.merchant_state().repair_cost == 0 and client.merchant_state().guild_repair_money == before.guild_repair_money - before.repair_cost, "guild repair"):
		return false
	var after: Dictionary = client.merchant_state()
	if after.money != before.money:
		return fail("Guild repair changed personal money %d -> %d" % [before.money, after.money])
	if sound_count(ITEM_REPAIR) != sounds + 1:
		return fail("Guild repair played ITEM_REPAIR %d times" % (sound_count(ITEM_REPAIR) - sounds))
	print("FIXTURE GUILD_REPAIRED cost=", before.repair_cost, " guild ", before.guild_repair_money, " -> ", after.guild_repair_money, " personal ", before.money, " -> ", after.money)
	await capture("6-guild-repaired")
	await tap(KEY_ESCAPE)
	return await wait_until(func(): return not client.merchant_state().open, "merchant closed")

func money_log() -> bool:
	if not await open_vault_at_stormwind():
		return false
	await press("GuildBankFrameTab3")
	if not await wait_until(func(): return log_lines().any(func(line): return line.contains(" for repairs")), "repair in the money log"):
		return false
	print("FIXTURE GUILD_MONEY_LOG ", log_lines())
	await capture("7-guild-money-log")
	return true

func has_line(lines: PackedStringArray, part: String) -> bool:
	for line in lines:
		if line.contains(part):
			return true
	return false

func sound_count(kit: int) -> int:
	return client.merchant_state().ui_sounds.count(kit)

func hover(point: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	Input.warp_mouse(point)
	root.push_input(motion, true)
	await wait_frames(4)

## Shift down or up as a key event carrying the modifier state.
func push_shift(pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = KEY_SHIFT
	event.physical_keycode = KEY_SHIFT
	event.pressed = pressed
	event.shift_pressed = pressed
	root.push_input(event, true)
	await wait_frames(3)
