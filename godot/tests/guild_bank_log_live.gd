extends "res://tests/bank_live.gd"

## Private guild master beside a vault with 25 real money-log transactions.
## BANK_ACCOUNT/BANK_CHARACTER, GODOT_TEST_SERVER, BANK_SHOTS.
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
	if not await wait_until(func(): return shown("GuildBankFrame"), "vault"):
		return
	await press("GuildBankFrameTab3")
	if not await wait_until(func(): return shown("GuildBankFrameLogLine1"), "money log"):
		return
	if shown("GuildBankFrameLogLine25"):
		fail("Fixture requires more transactions than fit in the initial viewport")
		return
	for i in range(5):
		await press("GuildBankMessageFrameScrollForward")
	if not await wait_until(func(): return shown("GuildBankFrameLogLine25"), "oldest retained transaction"):
		return
	print("FIXTURE GUILD_BANK_LOG_LIVE_DONE skin=", client.account_state().ui_skin)
	client.free()
	quit(0)
