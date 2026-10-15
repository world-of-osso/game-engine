extends "res://tests/communities_fidelity.gd"
# Synthetic offline invitation through the production screen/input reducer.

func capture() -> void:
	var directory := OS.get_environment("GUILD_INVITE_CAPTURES")
	assert(not directory.is_empty())
	for skin in ["modern", "forever"]:
		for choice in [["JoinButton", "AcceptInvite"], ["DeclineButton", "DeclineInvite"]]:
			var ui: Node = ClassDB.instantiate("RegistryUi")
			root.add_child(ui)
			assert(ui.call("show_guild_invitation_preview", skin == "forever") == "")
			await settle()
			assert_rect(ui, "GuildInviteFrame", Rect2(805, 446, 310, 188))
			assert_rect(ui, "GuildInviteFrameBg", Rect2(815, 456, 290, 168))
			for corner in [["TopLeftCorner", 805, 446], ["TopRightCorner", 1083, 446], ["BottomLeftCorner", 805, 602], ["BottomRightCorner", 1083, 602]]:
				assert_rect(ui, "GuildInviteFrame" + corner[0], Rect2(corner[1], corner[2], 32, 32))
			assert_rect(ui, "GuildInviteFrameTopBorder", Rect2(837, 447, 246, 16))
			assert_rect(ui, "GuildInviteFrameBottomBorder", Rect2(837, 618, 246, 16))
			assert_rect(ui, "GuildInviteFrameLeftBorder", Rect2(806, 478, 16, 124))
			assert_rect(ui, "GuildInviteFrameRightBorder", Rect2(1099, 478, 16, 124))
			assert(ui.call("frame_text", "GuildInviteFrameInviterName") == "Alice")
			assert(ui.call("frame_text", "GuildInviteFrameGuildName") == "Raid Team")
			if choice[0] == "JoinButton":
				capture_png(directory, skin + "-invite-after.png")
			var button: String = "GuildInviteFrame" + choice[0]
			await click(ui, button)
			assert(String(ui.call("guild_communities_preview_receipt")).count(choice[1]) == 1)
			assert((control(ui, button) as Button).disabled)
			await click(ui, button)
			assert(String(ui.call("guild_communities_preview_receipt")).count(choice[1]) == 1, "Pending input must not duplicate a request")
			ui.queue_free()
			await settle()
	print("CHROME PASS: both-skin native source inset/eight border pieces, physical Join/Decline and pending pacing; fixture authority only")
	quit(0)
