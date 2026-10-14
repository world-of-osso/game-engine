extends SceneTree

func _initialize() -> void:
	call_deferred("capture")

func capture() -> void:
	var directory := OS.get_environment("COMMUNITIES_CAPTURES")
	assert(not directory.is_empty())
	for skin in ["modern", "forever"]:
		var ui: Node = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		var error: String = ui.call("show_guild_communities_preview", skin == "forever")
		assert(error == "", error)
		# Offline host mirrors GameClient's production ScrollBox event route.
		ui.call("enable_gameplay_preview_scroll_input")
		await settle()
		assert_rect(ui, "CommunitiesFrame", Rect2(553, 327, 814, 426))
		assert_rect(ui, "CommunitiesMemberListScrollBox", Rect2(749, 410, 592, 315))
		assert_rect(ui, "GuildControlButton", Rect2(1232, 728, 130, 20))
		assert_rect(ui, "GuildRosterTab", Rect2(1367, 415, 32, 32))
		assert_rect(ui, "GuildInfoTab", Rect2(1367, 519, 32, 32))
		assert_rect(ui, "CommunitiesRosterColumn0", Rect2(748, 385, 40, 24))
		assert_rect(ui, "CommunitiesRosterColumn2", Rect2(829, 385, 100, 24))
		for name in ["GuildRosterNext", "GuildRosterPrevious", "GuildRosterPage", "GuildRosterInviteButton"]:
			assert(ui.find_child(name, true, false) == null, name + " is not in Retail roster mode")
		assert(ui.call("frame_text", "GuildMember0Level") == "80")
		assert(ui.call("frame_text", "GuildMember1Zone") == "Yesterday")
		assert(ui.call("frame_text", "GuildMember1Note") == "", "Officer notes are not public notes")
		capture_png(directory, skin + "-communities-roster.png")
		await wheel(ui, 1)
		assert(is_equal_approx(control(ui, "GuildMember0").get_global_rect().position.y, 370.0), "Wheel uses two20px pan extents")
		await drag_to_bottom(ui)
		var viewport := control(ui, "CommunitiesMemberListScrollBox").get_global_rect()
		assert(viewport.encloses(control(ui, "GuildMember31").get_global_rect()), "Last of32 members reachable without paging")
		var thumb := control(ui, "CommunitiesMemberListScrollBoxScrollThumb").get_global_rect()
		assert(not viewport.intersects(thumb), "Sibling scrollbar is not clipped inside the list")
		capture_png(directory, skin + "-communities-scrolled.png")
		await click(ui, "GuildMember31", MOUSE_BUTTON_RIGHT)
		assert(String(ui.call("guild_communities_preview_receipt")).contains("Member31"))
		assert(ui.find_child("GuildMemberPromote", true, false) != null)
		await click(ui, "CommunitiesRosterColumn2")
		assert(ui.call("frame_text", "GuildMember0Name") == "Member00")
		await click(ui, "CommunitiesRosterColumn2")
		assert(ui.call("frame_text", "GuildMember0Name") == "Member31")
		await click(ui, "GuildInfoTab")
		assert(ui.call("frame_text", "GuildMOTDText") == "Meet at the guild hall tonight.")
		assert(ui.call("frame_text", "GuildInfoText") == "Tuesday raid\nThursday dungeon")
		capture_png(directory, skin + "-communities-info.png")
		await click(ui, "CommunitiesGuildBenefitsTab")
		assert(ui.find_child("GuildRosterInviteButton", true, false) == null)
		capture_png(directory, skin + "-communities-benefits.png")
		await click(ui, "CommunitiesChatTab")
		assert_rect(ui, "GuildRosterInviteButton", Rect2(1232, 728, 130, 20))
		assert(ui.find_child("GuildControlButton", true, false) == null)
		capture_png(directory, skin + "-communities-chat.png")
		await click(ui, "GuildRosterInviteButton")
		await click(ui, "GuildInviteMemberName")
		for character in "Elowen":
			await key(character.unicode_at(0), character.unicode_at(0))
			assert(ui.call("sync_input") == "")
		await click(ui, "GuildInviteMemberAccept")
		assert(String(ui.call("guild_communities_preview_receipt")).contains("Invite(\"Elowen\")"))
		await click(ui, "GuildRosterTab")
		await click(ui, "GuildControlButton")
		assert_rect(ui, "GuildControlUI", Rect2(440, 130, 338, 444))
		ui.queue_free()
		await settle()
	print("PASS: both-skin Communities source rectangles, physical wheel/thumb overflow,32 members, column sorting, right-click hierarchy, tabs and invitation reducer;1920x1080 captures. Chat/benefits services remain unbacked.")
	quit(0)

func control(ui: Node, name: String) -> Control:
	var node := ui.find_child(name, true, false) as Control
	assert(node != null, "Missing native control " + name)
	return node

func assert_rect(ui: Node, name: String, expected: Rect2) -> void:
	var actual := control(ui, name).get_global_rect()
	if not actual.is_equal_approx(expected):
		push_error(name + " actual=" + str(actual) + " expected=" + str(expected))
		quit(1)
	assert(actual.is_equal_approx(expected))

func settle() -> void:
	for frame in range(8):
		await process_frame
		await RenderingServer.frame_post_draw

func capture_png(directory: String, filename: String) -> void:
	var image := root.get_texture().get_image()
	assert(image.get_size() == Vector2i(1920, 1080))
	assert(image.save_png(directory.path_join(filename)) == OK)

func click(ui: Node, name: String, button: int = MOUSE_BUTTON_LEFT) -> void:
	var point := control(ui, name).get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	Input.parse_input_event(motion)
	await process_frame
	for down in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = button
		event.position = point
		event.global_position = point
		event.pressed = down
		Input.parse_input_event(event)
		await process_frame
	await settle()
	assert(ui.call("guild_control_preview_dispatch") == "")
	await settle()

func wheel(ui: Node, count: int) -> void:
	var point := control(ui, "CommunitiesMemberListScrollBox").get_global_rect().get_center()
	for notch in range(count):
		var event := InputEventMouseButton.new()
		event.button_index = MOUSE_BUTTON_WHEEL_DOWN
		event.pressed = true
		event.position = point
		event.global_position = point
		Input.parse_input_event(event)
		await settle()

func drag_to_bottom(ui: Node) -> void:
	var thumb := control(ui, "CommunitiesMemberListScrollBoxScrollThumb").get_global_rect()
	var track := control(ui, "CommunitiesMemberListScrollBoxScrollTrack").get_global_rect()
	var start := thumb.get_center()
	var finish := Vector2(start.x, track.end.y)
	var button := InputEventMouseButton.new()
	button.button_index = MOUSE_BUTTON_LEFT
	button.pressed = true
	button.position = start
	button.global_position = start
	Input.parse_input_event(button)
	await process_frame
	var motion := InputEventMouseMotion.new()
	motion.position = finish
	motion.global_position = finish
	motion.relative = finish - start
	Input.parse_input_event(motion)
	await process_frame
	button = InputEventMouseButton.new()
	button.button_index = MOUSE_BUTTON_LEFT
	button.pressed = false
	button.position = finish
	button.global_position = finish
	Input.parse_input_event(button)
	await settle()

func key(code: int, unicode: int) -> void:
	for down in [true, false]:
		var event := InputEventKey.new()
		event.keycode = code
		event.unicode = unicode
		event.pressed = down
		Input.parse_input_event(event)
		await process_frame
