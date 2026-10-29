extends "res://tests/guild_member_notes_live.gd"

## Two private clients in one guild; role a is the Guild Master. The leader
## provisions its own state: it withdraws every stack from the first vault tab
## through the frame, gets Linen Cloth from the admin wrapper when its bags hold
## too little, and deposits one Linen stack. Both clients then watch the
## server-authoritative results of a whole-stack move, a split and a merge.

const STACK_FILE := "stack-count"
const MIN_STACK := 3
const GRANT := 10
const SPLIT := 2

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
	if not completion.require(client.bank_state().guild_tab == 0, "Vault must open on its first tab"):
		return false
	if role == "a" and not await provision_leader():
		return false
	if not await barrier("provisioned"):
		return false
	var stack := int(FileAccess.get_file_as_string("%s/%s" % [sync_dir, STACK_FILE]))
	if not completion.require(stack >= MIN_STACK, "Leader must record the deposited stack"):
		return false
	if not await wait_slots([stack, 0], "deposited stack"):
		return false
	# Whole stack: pick up slot 1 and drop it on empty slot 2.
	if role == "a":
		await left_click_slot(1)
		await left_click_slot(2)
	if not await barrier("moved") or not await wait_slots([0, stack], "whole-stack move"):
		return false
	if not await capture_checked("moved"):
		return false
	# Split: Shift-click slot 2, type 2, Enter, drop on empty slot 1.
	if role == "a" and not await split_onto_first_slot():
		return false
	if not await barrier("split") or not await wait_slots([SPLIT, stack - SPLIT], "split"):
		return false
	if not await capture_checked("split"):
		return false
	# Merge: the split part goes back onto its source stack.
	if role == "a":
		await left_click_slot(1)
		await left_click_slot(2)
	if not await barrier("merged") or not await wait_slots([0, stack], "merge"):
		return false
	if not await capture_checked("merged"):
		return false
	if role == "a":
		var before := bag_count(LINEN)
		await click_at(slot_center(2), MOUSE_BUTTON_RIGHT)
		if not await wait_until(func(): return bag_count(LINEN) == before + stack and slot_count(1) == 0, "leader restores Linen to bags"):
			return false
	return await barrier("done")

## Empty tab 1 through the frame, then deposit one backpack Linen stack into slot 1.
func provision_leader() -> bool:
	if not await clear_first_tab():
		return false
	if bag_count(LINEN) < MIN_STACK and not admin_grant_linen():
		return false
	if not await wait_until(func(): return largest_backpack_linen().get("count", 0) >= MIN_STACK, "Linen stack in the backpack"):
		return false
	var linen := largest_backpack_linen()
	await click_at(control("ContainerFrame0Slot%d" % linen.slot).get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
	if not await wait_until(func(): return slot_count(0) == linen.count, "deposit of %d Linen" % linen.count):
		return false
	FileAccess.open("%s/%s" % [sync_dir, STACK_FILE], FileAccess.WRITE).store_string(str(linen.count))
	return true

func clear_first_tab() -> bool:
	while true:
		var filled := first_tab_slots()
		if filled.is_empty():
			return true
		var slot: int = filled[0]
		await click_at(slot_center(slot + 1), MOUSE_BUTTON_RIGHT)
		if not await wait_until(func(): return slot_count(slot) == 0, "withdraw leftover from slot %d" % (slot + 1)):
			return false
	return false

func admin_grant_linen() -> bool:
	var output := []
	var code := OS.execute(OS.get_environment("BANK_ADMIN"), ["grant-item", OS.get_environment("BANK_CHARACTER"), str(LINEN), str(GRANT)], output, true)
	print("FIXTURE GRANT ", code, " ", output)
	return code == 0 or fail("grant-item failed")

func split_onto_first_slot() -> bool:
	var point := slot_center(2)
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
	await left_click_slot(1)
	return true

func left_click_slot(id: int) -> void:
	await click_at(slot_center(id), MOUSE_BUTTON_LEFT)

func slot_center(id: int) -> Vector2:
	return control("GuildBankFrameItem%d" % id).get_global_rect().get_center()

## Server counts of slots 1 and 2 of tab 1, and their rendered count labels.
func wait_slots(counts: Array, what: String) -> bool:
	return await wait_until(func(): return slot_count(0) == counts[0] and slot_count(1) == counts[1] and text("GuildBankFrameItem1Count") == count_label(counts[0]) and text("GuildBankFrameItem2Count") == count_label(counts[1]), what)

func count_label(count: int) -> String:
	return str(count) if count > 1 else ""

func slot_count(slot: int) -> int:
	for entry in client.bank_state().guild_slots:
		if entry.tab == 0 and entry.slot == slot:
			return entry.count
	return 0

func first_tab_slots() -> Array:
	var slots := []
	for entry in client.bank_state().guild_slots:
		if entry.tab == 0:
			slots.append(entry.slot)
	return slots

func largest_backpack_linen() -> Dictionary:
	var best := {}
	for entry in client.merchant_state().bags:
		if entry.item_id == LINEN and entry.bag == 0 and entry.count > best.get("count", 0):
			best = entry
	return best
