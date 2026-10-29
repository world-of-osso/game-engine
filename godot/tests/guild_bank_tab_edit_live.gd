extends "res://tests/guild_member_notes_live.gd"

## Two private clients, a purchased tab and Guild Master/member accounts.
## BANK_ROLE, BANK_SYNC, BANK_SHOTS, BANK_ACCOUNT, BANK_CHARACTER, BANK_ADMIN,
## GODOT_TEST_SERVER and GUILD_BANK_SKIN are supplied by the coordinator.
func run_completion() -> void:
	var success := await run_tab_editor()
	if success:
		print("PASS GUILD_BANK_TAB_EDIT role=", role, " skin=", client.account_state().ui_skin)
		completion.case_done()
		client.free()
	completion.finish(success)

func run_tab_editor() -> bool:
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
	if not await wait_until(func(): return shown("GuildBankTab1Button"), "purchased tab"):
		return false
	if role == "a":
		await click_at(control("GuildBankTab1Button").get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
		if not await wait_until(func(): return shown("GuildBankPopupName"), "tab editor"):
			return false
		var editor := control("GuildBankPopupName") as LineEdit
		if not completion.require(editor.has_focus(), "Name editor must receive focus"):
			return false
		editor.select_all()
		await tap(KEY_BACKSPACE)
		await type_text("SixteenLetters!!")
		if not completion.require(editor.text == "SixteenLetters!!", "The full 16-character name must be entered"):
			return false
		if not await press_checked("GuildBankIcon1"):
			return false
		if not await capture_checked("tab-editor"):
			return false
		if not await press_checked("GuildBankPopupOkay"):
			return false
	if not await barrier("tab-edit-submitted"):
		return false
	if not await wait_until(func(): return text("GuildBankFrameTabTitle") == "SixteenLetters!!", "authoritative tab name broadcast"):
		return false
	if not await capture_checked("tab-edited"):
		return false
	return await barrier("tab-edit-observed")
