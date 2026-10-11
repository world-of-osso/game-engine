extends "res://tests/bank_live.gd"

## Private member beside a vault with multiline tab text (no Info edit permission).
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
	await press("GuildBankFrameTab4")
	if not await wait_until(func(): return text("GuildBankFrameInfoText").contains("\n"), "multiline readonly info"):
		return
	if shown("GuildBankTabInfoEditBox") or shown("GuildBankFrameInfoSaveButton"):
		fail("Member received an editable info control")
		return
	var label := control("GuildBankFrameInfoText") as Label
	var glyph := label.get_character_bounds(0)
	if glyph.size.y <= 0 or glyph.position.y > 24:
		fail("Readonly first glyph should start at the top: " + str(glyph))
		return
	print("FIXTURE GUILD_BANK_READONLY_INFO_LIVE_DONE skin=", client.account_state().ui_skin, " glyph=", glyph)
	client.free()
	quit(0)
