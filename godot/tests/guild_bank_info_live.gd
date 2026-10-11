extends "res://tests/bank_live.gd"

## Private guild master beside a vault with a purchased tab.
## BANK_ACCOUNT/BANK_CHARACTER, GODOT_TEST_SERVER, BANK_SHOTS.
const INFO := "Monday supplies\nFriday raid"

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	shots = OS.get_environment("BANK_SHOTS")
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await wait_frames(30)
	var error = client.connect_account(OS.get_environment("GODOT_TEST_SERVER"), OS.get_environment("BANK_ACCOUNT"), PASSWORD, false)
	if error != "":
		fail(error)
		return
	if not await enter_world(OS.get_environment("BANK_CHARACTER")):
		return
	if not await click_pickable(func(node): return node.has_meta("game_object_name") and str(node.get_meta("game_object_name")) == VAULT, "game_object_server_id"):
		return
	if not await wait_until(func(): return shown("GuildBankFrame") and embedded_backpack_shown(), "vault projection"):
		return
	await press("GuildBankFrameTab4")
	if not await wait_until(func(): return shown("GuildBankTabInfoEditBox"), "tab editor"):
		return
	await press("GuildBankTabInfoEditBox")
	var editor = control("GuildBankTabInfoEditBox")
	editor.select_all()
	await tap(KEY_BACKSPACE)
	await type_text("Monday supplies")
	await tap(KEY_ENTER)
	await type_text("Friday raid")
	await wait_frames(30)
	if editor.text != INFO:
		fail("Typed multiline bank info: " + editor.text)
		return
	await press("GuildBankFrameInfoSaveButton")
	await wait_frames(90)
	# Rehydrate from the authoritative tab contents, not the typed local editor.
	await press("GuildBankFrameTab1")
	await press("GuildBankFrameTab4")
	if not await wait_until(func(): return control("GuildBankTabInfoEditBox").text == INFO, "authoritative multiline bank info"):
		return
	print("FIXTURE GUILD_BANK_INFO_LIVE_DONE skin=", client.account_state().ui_skin)
	client.free()
	quit(0)
