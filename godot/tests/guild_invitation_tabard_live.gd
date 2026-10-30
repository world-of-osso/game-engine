extends "res://tests/guild_perks_live.gd"

var invitation_mode := ""
var invitation_officer_dir := ""

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	Engine.max_fps = 30
	shots = OS.get_environment("BANK_SHOTS")
	diagnostics = OS.get_environment("GUILD_PERKS_DIAGNOSTICS")
	invitation_mode = OS.get_environment("GUILD_INVITATION_MODE")
	invitation_officer_dir = OS.get_environment("GUILD_PERKS_OFFICER_DIR")
	assert(invitation_mode in ["no-design", "registered"])
	assert(OS.get_environment("GODOT_TEST_SERVER") == "127.0.0.1:5891")
	assert(OS.get_environment("BANK_ACCOUNT") == "fb_guildcompletion_perkspeer")
	assert(not invitation_officer_dir.is_empty())
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await wait_frames(30)
	role = "invitation-entry"
	if not await connect_private_account() or not await enter_world("Gcperkspeer"):
		return
	if not await wait_until(func(): return not perk_observation().startup_loading, "actual world startup completion"):
		return
	# An interrupted trial leaves a real durable invitation. Consume it through the UI before new requests.
	if invitation_receipt() != null:
		await press("GuildInviteFrameDeclineButton")
		if not await wait_until(func(): return invitation_receipt() == null and not shown("GuildInviteFrameJoinButton"), "decline the prior durable invitation"):
			return
		await wait_frames(30)
	var phase := 0
	for skin in ["Forever"]:
		phase += 1
		if not await select_skin(skin):
			return
		role = skin.to_lower() + "-invitation-" + invitation_mode
		var marker := FileAccess.open(invitation_officer_dir.path_join("invite-%d.go" % phase), FileAccess.WRITE)
		assert(marker != null)
		marker.store_string("actual invitation requested")
		marker.close()
		if not await wait_until(func(): return shown("GuildInviteFrameJoinButton") and invitation_receipt() is Dictionary, "actual officer invitation"):
			return
		var receipt: Dictionary = invitation_receipt()
		if receipt.guild_name != "Guild Perks" or receipt.inviter_name != "Gcperkslead":
			fail("Wrong actual inviting guild/owner: " + str(receipt))
			return
		if not valid_received_design(receipt.tabard):
			fail("Wrong optional registered invitation design: " + str(receipt))
			return
		write_json(role + "-received.json", receipt)
		await capture("pending")
		if not prove_source_rectangles() or not prove_source_text():
			return
		await press("GuildInviteFrameDeclineButton")
		if not await wait_until(func(): return not shown("GuildInviteFrameJoinButton") and invitation_receipt() == null, "physical decline clears pending invitation"):
			return
		await wait_frames(30)
	var done := FileAccess.open(invitation_officer_dir.path_join("done.go"), FileAccess.WRITE)
	assert(done != null)
	done.store_string("done")
	done.close()
	print("PASS GUILD_INVITATION_TABARD_NATIVE mode=", invitation_mode, " both_skins=true real_invites=2 physical_declines=2")
	client.free()
	quit(0)

func prove_source_rectangles() -> bool:
	var emblem_width := 64.0 if invitation_mode == "no-design" else 56.0
	var popup := control("GuildInviteFrame").get_global_rect()
	if not popup.is_equal_approx(Rect2(805, 446, 310, 188)):
		return fail("Wrong source invitation root rectangle: " + str(popup))
	var source := {
		"Background": Rect2(11.5, 12, 287, 116),
		"TabardBackground": Rect2(123, 70, 62, 62),
		"TabardEmblem": Rect2(154 - emblem_width / 2, 69, emblem_width, 64),
		"TabardBorder": Rect2(124, 71, 61, 60),
		"TabardRing": Rect2(96, 64, 118, 76),
	}
	var actual := {}
	for name in source:
		if not shown("GuildInviteFrame" + name):
			return fail("Missing source invitation texture " + name)
		var rectangle := control("GuildInviteFrame" + name).get_global_rect()
		var expected: Rect2 = source[name]
		# Projection snaps controls to whole pixels; the centered 287px art starts at x=11.5.
		expected.position = (expected.position + popup.position).round()
		if not rectangle.is_equal_approx(expected):
			return fail("Wrong source rectangle " + name + ": " + str(rectangle))
		actual[name] = [rectangle.position.x, rectangle.position.y, rectangle.size.x, rectangle.size.y]
	write_json(role + "-rectangles.json", actual)
	return true

func prove_source_text() -> bool:
	var popup := control("GuildInviteFrame").get_global_rect()
	var actual := {}
	for entry in [["InviterName", 28, 10], ["InviteText", 38, 10], ["GuildName", 50, 12]]:
		var name: String = "GuildInviteFrame" + entry[0]
		var label := control(name) as Label
		if label == null:
			return fail("Missing invitation text " + name)
		var rectangle := label.get_global_rect()
		if rectangle.position.y != popup.position.y + float(entry[1]):
			return fail("Wrong source TOP text anchor " + name + ": " + str(rectangle))
		var font_size := label.get_theme_font_size("font_size")
		if font_size != int(entry[2]):
			return fail("Wrong source inherited text size " + name + ": " + str(font_size))
		actual[name] = {"rect": [rectangle.position.x, rectangle.position.y, rectangle.size.x, rectangle.size.y], "font_size": font_size}
	var guild_name := control("GuildInviteFrameGuildName").get_global_rect()
	var emblem := control("GuildInviteFrameTabardEmblem").get_global_rect()
	if guild_name.end.y >= emblem.position.y:
		return fail("Guild name overlaps the actual tabard: " + str(guild_name))
	# A measured intrinsic string may have an odd pixel width.
	if abs(guild_name.get_center().x - popup.get_center().x) > 0.5:
		return fail("Intrinsic guild name lost its source centered TOP anchor")
	write_json(role + "-text-layout.json", actual)
	return true

func invitation_receipt() -> Variant:
	return JSON.parse_string(str(client.call("guild_invitation_observation")))

func valid_received_design(design: Variant) -> bool:
	if invitation_mode == "no-design":
		return design == null
	if not design is Dictionary:
		return false
	var expected := {"icon": 1, "icon_color": 2, "border": 3, "border_color": 4, "background_color": 5}
	for key in expected:
		if not design.has(key) or int(design[key]) != expected[key]:
			return false
	return true
