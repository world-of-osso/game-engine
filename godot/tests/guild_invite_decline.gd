extends "res://tests/communities_fidelity.gd"
# Uses the debug-only invitation fixture; no server authority claim.

func capture() -> void:
	var directory := OS.get_environment("GUILD_INVITE_CAPTURES")
	assert(not directory.is_empty())
	for skin in ["modern", "forever"]:
		var ui: Node = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		assert(ui.call("show_guild_invitation_preview", skin == "forever") == "")
		await settle()
		assert_rect(ui, "GuildInviteFrame", Rect2(805, 446, 310, 188))
		assert_rect(ui, "GuildInviteFrameJoinButton", Rect2(829, 588, 110, 22))
		assert_rect(ui, "GuildInviteFrameDeclineButton", Rect2(951, 588, 140, 22))
		capture_png(directory, skin + "-decline-after.png")
		await click(ui, "GuildInviteFrameDeclineButton")
		assert(String(ui.call("guild_communities_preview_receipt")).count("DeclineInvite") == 1)
		assert((control(ui, "GuildInviteFrameDeclineButton") as Button).disabled)
		await click(ui, "GuildInviteFrameDeclineButton")
		assert(String(ui.call("guild_communities_preview_receipt")).count("DeclineInvite") == 1)
		ui.queue_free()
		await settle()
	print("DECLINE PASS: both-skin source140x22 right/bottom anchors, unchanged Join rectangle and physical no-duplicate Decline")
	quit(0)
