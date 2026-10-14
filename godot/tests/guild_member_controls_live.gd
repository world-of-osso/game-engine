extends "res://tests/bank_live.gd"

## Private guild master with a second member at default Member rank.
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
	await press("MinimapLauncherButton")
	await press("LauncherEntryGuildMicroButton")
	if not await wait_until(func(): return shown("GuildMember1"), "authoritative roster"):
		return
	var before := member_text()
	await click_at(control("GuildMember1").get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
	if not await wait_until(func(): return shown("GuildMemberPromote"), "right-click member menu"):
		return
	var menu := control("GuildMemberContextMenu").get_global_rect()
	if not Rect2(Vector2.ZERO, Vector2(root.size)).encloses(menu):
		fail("Member menu outside viewport: " + str(menu))
		return
	await press("GuildMemberPromote")
	if not await wait_until(func(): return member_text() != before, "authoritative promotion"):
		return
	await click_at(control("GuildMember1").get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
	if not await wait_until(func(): return shown("GuildMemberDemote"), "demote menu"):
		return
	await press("GuildMemberDemote")
	if not await wait_until(func(): return member_text() == before, "authoritative demotion"):
		return
	print("FIXTURE GUILD_MEMBER_CONTROLS_LIVE_DONE skin=", client.account_state().ui_skin)
	client.free()
	quit(0)

func member_text() -> String:
	var rank := control("GuildMember1Rank") as Label
	if rank == null:
		return ""
	return rank.text
