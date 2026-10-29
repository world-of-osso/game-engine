extends "res://tests/guild_member_notes_live.gd"

## Two private clients; role a is the Guild Master, role b a Member. Before either
## logs in, the leader puts both characters into a fresh guild through the admin
## wrapper, sets the Member tab rights and its own gold. Through the frame it buys
## two tabs and deposits Linen into tab 1 and Wool into tab 2. Both clients then
## check the server's slots after each move:
##   same tab: whole-stack move, split, merge (leader);
##   cross tab: swap and move (leader, unlimited rights);
##   rights: missing deposit right, a move and a swap that use allowances, the
##   daily limit, and a same-tab move that needs no allowance (member).

const WOOL := 2592
const GUILD_NAME := "Vault Keepers"
const MEMBER_RANK := 3
const GRANT := 10
const SPLIT := 2
## Member allowances: tab 1 three stacks a day, tab 2 one.
const TAB1_PER_DAY := 3
const TAB2_PER_DAY := 1
const PERMISSIONS := "You don't have permission to do that."
const WITHDRAW_LIMIT := "You cannot withdraw that much from the guild bank."

var guild_id := ""
var linen_stack := 0
var wool_stack := 0

func run_completion() -> void:
	var success := await run_moves()
	if success:
		print("PASS GUILD_BANK_MOVE role=", role, " skin=", client.account_state().ui_skin)
		completion.case_done()
		client.free()
	completion.finish(success)

func run_moves() -> bool:
	root.size = Vector2i(1920, 1080)
	Engine.max_fps = 30
	role = OS.get_environment("BANK_ROLE")
	sync_dir = OS.get_environment("BANK_SYNC")
	shots = OS.get_environment("BANK_SHOTS")
	DirAccess.make_dir_recursive_absolute(shots)
	if role == "a" and not provision_guild():
		return false
	if not await barrier("guild-ready"):
		return false
	guild_id = FileAccess.get_file_as_string(sync_path("guild-id"))
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await wait_frames(30)
	var error: String = client.connect_account(OS.get_environment("GODOT_TEST_SERVER"), OS.get_environment("BANK_ACCOUNT"), PASSWORD, false)
	if error != "":
		return fail(error)
	if not await enter_world(OS.get_environment("BANK_CHARACTER")) or not await select_skin(OS.get_environment("GUILD_BANK_SKIN")):
		return false
	if not await open_vault():
		return false
	if role == "a" and not await stock_vault():
		return false
	if not await barrier("stocked"):
		return false
	linen_stack = int(FileAccess.get_file_as_string(sync_path("linen")))
	wool_stack = int(FileAccess.get_file_as_string(sync_path("wool")))
	if not await wait_layout([[0, 0, LINEN, linen_stack], [1, 0, WOOL, wool_stack]], "stocked vault"):
		return false
	return await same_tab_cases() and await cross_tab_cases() and await rights_cases() and await barrier("done")

# --- provisioning ----------------------------------------------------------

## A fresh guild per run: no leftover slots and no allowance used today.
func provision_guild() -> bool:
	var id := str(int(Time.get_unix_time_from_system()))
	var leader := OS.get_environment("BANK_CHARACTER")
	var member := OS.get_environment("GUILD_PEER_CHARACTER")
	var steps := [
		["create-guild", leader, id, GUILD_NAME],
		["create-guild", member, id, GUILD_NAME],
		["set-guild-rank", member, str(MEMBER_RANK)],
		["set-guild-bank-rights", id, str(MEMBER_RANK), "tab", "0", "1", "1", str(TAB1_PER_DAY)],
		# Tab 2 starts without the deposit right; the first rights case needs it missing.
		["set-guild-bank-rights", id, str(MEMBER_RANK), "tab", "1", "1", "0", str(TAB2_PER_DAY)],
		["set-gold", leader, "10000000"],
	]
	for step in steps:
		if not admin(step):
			return false
	FileAccess.open(sync_path("guild-id"), FileAccess.WRITE).store_string(id)
	return true

## Buy two tabs, then deposit granted Linen into tab 1 and Wool into tab 2.
func stock_vault() -> bool:
	for tab in [1, 2]:
		if not await buy_tab(tab):
			return false
	for item in [LINEN, WOOL]:
		if not admin(["grant-item", OS.get_environment("BANK_CHARACTER"), str(item), str(GRANT)]):
			return false
	if not await wait_until(func(): return backpack_stack(LINEN) > 0 and backpack_stack(WOOL) > 0, "granted Linen and Wool"):
		return false
	var linen := await deposit_stack(LINEN, 0)
	var wool := await deposit_stack(WOOL, 1)
	if linen == 0 or wool == 0:
		return false
	FileAccess.open(sync_path("linen"), FileAccess.WRITE).store_string(str(linen))
	FileAccess.open(sync_path("wool"), FileAccess.WRITE).store_string(str(wool))
	return true

func buy_tab(tab: int) -> bool:
	await press("GuildBankTab%dButton" % tab)
	if not await wait_until(func(): return shown("GuildBankFrameBuyInfoPurchaseButton"), "purchase offer for tab %d" % tab):
		return false
	await press("GuildBankFrameBuyInfoPurchaseButton")
	if not await wait_until(func(): return popup_text() != "", "tab %d confirmation" % tab):
		return false
	await press("StaticPopup1Button1")
	return await wait_until(func(): return client.bank_state().guild_tabs.size() == tab, "purchased tab %d" % tab)

## Right-click the largest backpack stack of `item` with tab `tab` selected; its count.
func deposit_stack(item: int, tab: int) -> int:
	if not await select_tab(tab):
		return 0
	var entry := largest_backpack(item)
	await click_at(control("ContainerFrame0Slot%d" % entry.slot).get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
	if not await wait_until(func(): return slot_item(tab, 0) == [item, entry.count], "deposit of %d x %d into tab %d" % [entry.count, item, tab + 1]):
		return 0
	return entry.count

# --- cases -----------------------------------------------------------------

func same_tab_cases() -> bool:
	if role == "a":
		await move_slot(0, 0, 0, 1)
	if not await step("same-tab-move", [[0, 1, LINEN, linen_stack], [1, 0, WOOL, wool_stack]]):
		return false
	if role == "a" and not await split_slot(0, 1, 0, 0):
		return false
	if not await step("same-tab-split", [[0, 0, LINEN, SPLIT], [0, 1, LINEN, linen_stack - SPLIT], [1, 0, WOOL, wool_stack]]):
		return false
	if role == "a":
		await move_slot(0, 0, 0, 1)
	return await step("same-tab-merge", [[0, 1, LINEN, linen_stack], [1, 0, WOOL, wool_stack]])

func cross_tab_cases() -> bool:
	if role == "a":
		await move_slot(0, 1, 1, 0)
	if not await step("cross-tab-swap", [[0, 1, WOOL, wool_stack], [1, 0, LINEN, linen_stack]]):
		return false
	if role == "a":
		await move_slot(1, 0, 0, 0)
	return await step("cross-tab-move", [[0, 0, LINEN, linen_stack], [0, 1, WOOL, wool_stack]])

## Server rights of the Member (TrinityCore Guild.cpp:945-1029, 3151-3245).
func rights_cases() -> bool:
	var start := [[0, 0, LINEN, linen_stack], [0, 1, WOOL, wool_stack]]
	if role == "b":
		if not await wait_allowances([TAB1_PER_DAY, TAB2_PER_DAY], "Member allowances"):
			return false
		await move_slot(0, 0, 1, 0)
		if not await refused(PERMISSIONS, start, [TAB1_PER_DAY, TAB2_PER_DAY], "move into a tab without the deposit right"):
			return false
	if not await step("rights-no-deposit", start):
		return false
	if role == "a" and not admin(["set-guild-bank-rights", guild_id, str(MEMBER_RANK), "tab", "1", "1", "1", str(TAB2_PER_DAY)]):
		return false
	if not await barrier("deposit-granted"):
		return false
	if role == "b":
		await move_slot(0, 1, 1, 0)
	if not await step("rights-move", [[0, 0, LINEN, linen_stack], [1, 0, WOOL, wool_stack]]):
		return false
	if role == "b" and not await wait_allowances([TAB1_PER_DAY - 1, TAB2_PER_DAY], "source allowance used by a move"):
		return false
	if role == "b":
		await move_slot(0, 0, 1, 0)
	var swapped := [[0, 0, WOOL, wool_stack], [1, 0, LINEN, linen_stack]]
	if not await step("rights-swap", swapped):
		return false
	if role == "b":
		if not await wait_allowances([TAB1_PER_DAY - 2, 0], "both allowances used by a swap"):
			return false
		await move_slot(1, 0, 0, 1)
		if not await refused(WITHDRAW_LIMIT, swapped, [TAB1_PER_DAY - 2, 0], "move from a tab with no allowance left"):
			return false
	if not await step("rights-limit", swapped):
		return false
	# Same-tab moves need no allowance (Guild.cpp:3120-3127).
	if role == "b":
		await move_slot(1, 0, 1, 1)
	if not await step("rights-same-tab", [[0, 0, WOOL, wool_stack], [1, 1, LINEN, linen_stack]]):
		return false
	return role == "a" or await wait_allowances([TAB1_PER_DAY - 2, 0], "same-tab move used no allowance")

## Both clients reach `name`, then see exactly `layout` and capture it.
func step(name: String, layout: Array) -> bool:
	return await barrier(name) and await wait_layout(layout, name) and await capture_checked(name)

func refused(message: String, layout: Array, allowances: Array, what: String) -> bool:
	if not await wait_until(func(): return client.bank_state().guild_error == message, "refusal of " + what):
		return false
	# A refusal sends no contents; the slots and allowances stay as they were.
	await wait_frames(30)
	return completion.require(layout_matches(layout) and allowances_match(allowances), "Refused %s changed the vault" % what)

# --- input -----------------------------------------------------------------

func select_tab(tab: int) -> bool:
	if client.bank_state().guild_tab != tab:
		await press("GuildBankTab%dButton" % (tab + 1))
	return await wait_until(func(): return client.bank_state().guild_tab == tab, "tab %d selected" % (tab + 1))

## Pick up the whole stack at (tab, slot), switch tabs if needed and drop it.
func move_slot(from_tab: int, from_slot: int, to_tab: int, to_slot: int) -> void:
	await select_tab(from_tab)
	await click_at(slot_center(from_slot), MOUSE_BUTTON_LEFT)
	await select_tab(to_tab)
	await click_at(slot_center(to_slot), MOUSE_BUTTON_LEFT)

## Shift-click opens StackSplitFrame; type SPLIT, Enter, drop the split part.
func split_slot(from_tab: int, from_slot: int, to_tab: int, to_slot: int) -> bool:
	await select_tab(from_tab)
	var point := slot_center(from_slot)
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	Input.warp_mouse(point)
	await wait_frames(2)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.shift_pressed = true
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	if not await wait_until(func(): return shown("StackSplitFrame"), "guild bank split picker"):
		return false
	await type_text(str(SPLIT))
	if not completion.require(text("StackSplitText") == str(SPLIT), "Split picker must show the typed amount"):
		return false
	await tap(KEY_ENTER)
	if not await wait_until(func(): return not shown("StackSplitFrame"), "split picker closes"):
		return false
	await select_tab(to_tab)
	await click_at(slot_center(to_slot), MOUSE_BUTTON_LEFT)
	return true

func slot_center(slot: int) -> Vector2:
	return control("GuildBankFrameItem%d" % (slot + 1)).get_global_rect().get_center()

# --- observation -----------------------------------------------------------

## Every occupied slot of every tab, as [tab, slot, item_id, count], equals `layout`;
## the selected tab's count labels match too.
func wait_layout(layout: Array, what: String) -> bool:
	return await wait_until(func(): return layout_matches(layout) and labels_match(layout), what)

func layout_matches(layout: Array) -> bool:
	var seen := []
	for entry in client.bank_state().guild_slots:
		seen.append([entry.tab, entry.slot, entry.item_id, entry.count])
	if seen.size() != layout.size():
		return false
	for wanted in layout:
		if not seen.has(wanted):
			return false
	return true

func labels_match(layout: Array) -> bool:
	var tab: int = client.bank_state().guild_tab
	for slot in range(2):
		var count := 0
		for wanted in layout:
			if wanted[0] == tab and wanted[1] == slot:
				count = wanted[3]
		if text("GuildBankFrameItem%dCount" % (slot + 1)) != (str(count) if count > 1 else ""):
			return false
	return true

func wait_allowances(allowances: Array, what: String) -> bool:
	return await wait_until(func(): return allowances_match(allowances), what)

func allowances_match(allowances: Array) -> bool:
	var tabs: Array = client.bank_state().guild_tabs
	if tabs.size() != allowances.size():
		return false
	for index in tabs.size():
		if tabs[index].remaining != allowances[index]:
			return false
	return true

func slot_item(tab: int, slot: int) -> Array:
	for entry in client.bank_state().guild_slots:
		if entry.tab == tab and entry.slot == slot:
			return [entry.item_id, entry.count]
	return []

func largest_backpack(item: int) -> Dictionary:
	var best := {}
	for entry in client.merchant_state().bags:
		if entry.item_id == item and entry.bag == 0 and entry.count > best.get("count", 0):
			best = entry
	return best

func backpack_stack(item: int) -> int:
	return largest_backpack(item).get("count", 0)

func admin(arguments: Array) -> bool:
	var output := []
	var code := OS.execute(OS.get_environment("BANK_ADMIN"), arguments, output, true)
	print("FIXTURE ADMIN ", arguments, " ", code, " ", output)
	return code == 0 or fail("admin %s failed: %s" % [arguments, output])

func sync_path(name: String) -> String:
	return "%s/%s" % [sync_dir, name]
