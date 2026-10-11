extends "res://tests/bank_live.gd"

## Private guild master already positioned beside a real Guild Vault.
## BANK_ACCOUNT/BANK_CHARACTER, GODOT_TEST_SERVER, BANK_SHOTS.
## Verifies the production combined vault/backpack canvas, not an offline mock.
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
	if not await wait_until(func(): return shown("GuildBankFrame") and embedded_backpack_shown(), "vault and backpack projection"):
		return
	print("FIXTURE GUILD_VAULT_LIVE_DONE skin=", client.account_state().ui_skin)
	client.free()
	quit(0)
