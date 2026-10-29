extends "res://tests/guild_member_notes_live.gd"

## Two private clients. The leader begins with ten Linen Cloth in bags and an
## empty first vault tab; the peer watches every server-authoritative update.
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
	if not completion.require(text("GuildBankFrameItem1Count") == "" and text("GuildBankFrameItem2Count") == "", "First two vault slots must start empty"):
		return false
	if role == "a":
		if not completion.require(bag_count(LINEN) == 10, "Leader must start with ten Linen Cloth") or not await deposit_from_bags():
			return false
	if not await barrier("deposited") or not await wait_until(func(): return text("GuildBankFrameItem1Count") == "10", "deposit broadcast"):
		return false
	if role == "a":
		await click_at(control("GuildBankFrameItem1").get_global_rect().get_center(), MOUSE_BUTTON_LEFT)
		await click_at(control("GuildBankFrameItem2").get_global_rect().get_center(), MOUSE_BUTTON_LEFT)
	if not await barrier("moved") or not await wait_until(func(): return text("GuildBankFrameItem1Count") == "" and text("GuildBankFrameItem2Count") == "10", "same-tab move broadcast"):
		return false
	if not await capture_checked("moved"):
		return false
	if role == "a":
		var key := InputEventKey.new()
		key.keycode = KEY_SHIFT
		key.physical_keycode = KEY_SHIFT
		key.pressed = true
		key.shift_pressed = true
		root.push_input(key, true)
		await process_frame
		var point := control("GuildBankFrameItem2").get_global_rect().get_center()
		for down in [true, false]:
			var mouse := InputEventMouseButton.new()
			mouse.position = point
			mouse.global_position = point
			mouse.button_index = MOUSE_BUTTON_LEFT
			mouse.pressed = down
			mouse.shift_pressed = true
			root.push_input(mouse, true)
			await process_frame
		key.pressed = false
		key.shift_pressed = false
		root.push_input(key, true)
		if not await wait_until(func(): return shown("StackSplitFrame"), "guild bank split picker"):
			return false
		await tap(KEY_2)
		await tap(KEY_ENTER)
		await click_at(control("GuildBankFrameItem1").get_global_rect().get_center(), MOUSE_BUTTON_LEFT)
	if not await barrier("split") or not await wait_until(func(): return text("GuildBankFrameItem2Count") == "8" and text("GuildBankFrameItem1Count") == "2", "split broadcast"):
		return false
	if not await capture_checked("split"):
		return false
	if role == "a":
		await click_at(control("GuildBankFrameItem1").get_global_rect().get_center(), MOUSE_BUTTON_LEFT)
		await click_at(control("GuildBankFrameItem2").get_global_rect().get_center(), MOUSE_BUTTON_LEFT)
	if not await barrier("merged") or not await wait_until(func(): return text("GuildBankFrameItem1Count") == "" and text("GuildBankFrameItem2Count") == "10", "merge broadcast"):
		return false
	if role == "a":
		await click_at(control("GuildBankFrameItem2").get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
		if not await wait_until(func(): return bag_count(LINEN) == 10, "restore leader inventory"):
			return false
	return await barrier("done")
