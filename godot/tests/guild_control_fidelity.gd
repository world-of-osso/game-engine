extends SceneTree

func _initialize() -> void:
	call_deferred("capture")

func capture() -> void:
	var directory := OS.get_environment("GUILD_CAPTURES")
	assert(not directory.is_empty())
	for skin in ["modern", "forever"]:
		var ui: Node = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		var error: String = ui.call("show_guild_ranks_preview" if skin == "modern" else "show_forever_guild_ranks_preview")
		assert(error == "", error)
		await settle()
		assert_rect(ui, "GuildControlUI", Rect2(440, 130, 338, 444))
		await assert_dropdown_member_image(ui, "GuildControlUINavigationDropdownBackground")
		assert_rect(ui, "GuildControlUIRankOrderFrame", Rect2(454, 190, 312, 372))
		assert_rect(ui, "GuildRankOrderName2", Rect2(536, 258, 132, 32))
		assert_rect(ui, "GuildControlUIRankOrderFrameRank3ShiftUpButton", Rect2(669, 261, 26, 26))
		var backdrop: Color = root.get_texture().get_image().get_pixel(457, 530)
		if backdrop.r >= 0.1 or backdrop.g >= 0.1 or backdrop.b >= 0.1:
			push_error("TranslucentFrameTemplate black0.9 background missing: " + str(backdrop))
			quit(1)
		assert(backdrop.r < 0.1 and backdrop.g < 0.1 and backdrop.b < 0.1)
		capture_png(directory, skin + "-guild-control.png")
		await type_text(ui, "GuildRankOrderName2", "Pathfinder")
		var request: String = ui.call("guild_control_preview_dispatch")
		assert(request.contains("Rename") and request.contains("Pathfinder"), request)
		assert(ui.call("guild_control_preview_waiting"))
		assert(ui.call("guild_control_preview_reply") == "")
		await settle()
		assert(ui.call("frame_text", "GuildRankOrderName2") == "Pathfinder")
		await type_text(ui, "GuildRankOrderName2", "Discarded", false)
		var edit := control(ui, "GuildRankOrderName2") as LineEdit
		if edit.get_theme_color("font_color") != Color(0, 1, 0, 1):
			push_error("Source changed-name text must be green: " + str(edit.get_theme_color("font_color")))
			quit(1)
		assert(edit.get_theme_color("font_color") == Color(0, 1, 0, 1))
		await key(KEY_ESCAPE)
		await settle()
		var escaped: String = ui.call("guild_control_preview_dispatch")
		if escaped != "":
			push_error("Escape must restore authoritative text without a rename request: " + escaped)
			quit(1)
		assert(escaped == "")
		assert(ui.call("frame_text", "GuildRankOrderName2") == "Pathfinder")
		await click(ui, "GuildControlUINavigationDropdown")
		assert(ui.call("guild_control_preview_dispatch") == "")
		await settle()
		capture_png(directory, skin + "-guild-navigation-menu.png")
		await click(ui, "GuildControlPage1")
		assert(ui.call("guild_control_preview_dispatch") == "")
		await settle()
		assert_rect(ui, "GuildControlUIRankSettingsFrameRankDropdown", Rect2(574, 196, 180, 25))
		await assert_dropdown_member_image(ui, "GuildControlUIRankSettingsFrameRankDropdownBackground")
		assert_rect(ui, "GuildControlUIRankSettingsFrameOfficerBg", Rect2(465, 248, 290, 160))
		assert_rect(ui, "GuildControlUIRankSettingsFrameOfficerCheckbox", Rect2(465, 251, 20, 20))
		assert_rect(ui, "GuildRankGold", Rect2(612, 522, 60, 28))
		for id in [9, 11, 12, 13]:
			assert(ui.find_child("GuildControlUIRankSettingsFrameCheckbox" + str(id), true, false) == null, "Mainline has no such control")
		var officer := "GuildControlUIRankSettingsFrameOfficerCheckbox"
		var old_rights: int = ui.call("guild_control_preview_rights")
		assert((old_rights & 12) == 4, "Fixture deliberately preserves a mixed officer mask")
		assert(ui.find_child(officer + "Check", true, false) != null)
		await click(ui, officer)
		request = ui.call("guild_control_preview_dispatch")
		assert(request.contains("SetPermissions") and request.contains("rights: " + str(old_rights & ~12)), request)
		assert(ui.call("guild_control_preview_rights") == old_rights, "No optimistic authoritative mask mutation")
		assert(ui.call("guild_control_preview_waiting"))
		await settle()
		await click(ui, "GuildControlUIRankSettingsFrameCheckbox7")
		assert(ui.call("guild_control_preview_dispatch") == "", "Pending writes disable further mutations")
		assert(ui.call("guild_control_preview_reply") == "")
		await settle()
		assert(ui.find_child(officer + "Check", true, false) == null)
		await click(ui, officer)
		request = ui.call("guild_control_preview_dispatch")
		assert(request.contains("rights: " + str(old_rights | 12)), request)
		assert(ui.call("guild_control_preview_reply") == "")
		await settle()
		assert(ui.call("guild_control_preview_rights") == (old_rights | 12))
		await type_text(ui, "GuildRankGold", "7x")
		request = ui.call("guild_control_preview_dispatch")
		if not request.contains("gold_per_day: 70000"):
			push_error("Source numeric GoldBox must reject x while preserving digit7: " + request)
			quit(1)
		assert(request.contains("gold_per_day: 70000"), request)
		assert(ui.call("guild_control_preview_reply") == "")
		await settle()
		assert(ui.call("frame_text", "GuildRankGold") == "7")
		capture_png(directory, skin + "-guild-permissions.png")
		await click(ui, "GuildControlUIRankSettingsFrameRankDropdown")
		assert(ui.call("guild_control_preview_dispatch") == "")
		await settle()
		assert(ui.find_child("GuildControlRankChoice0", true, false) == null)
		capture_png(directory, skin + "-guild-rank-menu.png")
		await click(ui, "GuildControlRankChoice2")
		assert(ui.call("guild_control_preview_dispatch") == "")
		await click(ui, "GuildControlUINavigationDropdown")
		assert(ui.call("guild_control_preview_dispatch") == "")
		await click(ui, "GuildControlPage2")
		assert(ui.call("guild_control_preview_dispatch") == "")
		await settle()
		assert_rect(ui, "GuildControlBankScrollBox", Rect2(473, 243, 254, 294))
		assert_rect(ui, "GuildControlBankTab2", Rect2(473, 321, 254, 74))
		await click(ui, "GuildTabDeposit0")
		request = ui.call("guild_control_preview_dispatch")
		assert(request.contains("SetTab") and request.contains("deposit: false"), request)
		assert(ui.call("guild_control_preview_reply") == "")
		await type_text(ui, "GuildTabItems0", "9x")
		request = ui.call("guild_control_preview_dispatch")
		assert(request.contains("withdrawals_per_day: 9"), request)
		assert(ui.call("guild_control_preview_reply") == "")
		await settle()
		capture_png(directory, skin + "-guild-bank-permissions.png")
		assert(ui.call("guild_control_preview_overflow") == "")
		ui.call("enable_gameplay_preview_scroll_input")
		await settle()
		var list := control(ui, "GuildControlBankScrollBox")
		for step in range(48):
			for pressed in [true, false]:
				var wheel := InputEventMouseButton.new()
				wheel.button_index = MOUSE_BUTTON_WHEEL_DOWN
				wheel.position = list.get_global_rect().get_center()
				wheel.global_position = wheel.position
				wheel.pressed = pressed
				Input.parse_input_event(wheel)
				await process_frame
		await settle()
		var last := control(ui, "GuildControlBankTab8")
		assert(list.get_global_rect().encloses(last.get_global_rect()), "Wheel must expose eighth purchased tab: " + str(last.get_global_rect()))
		capture_png(directory, skin + "-guild-bank-scrolled.png")
		ui.queue_free()
		await process_frame
	print("PASS: both-skin GuildControlUI source rectangles, physical navigation/rename/Officer/gold/bank edits, pacing, replies and overflow; 1920x1080 captures")
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
	assert(actual.is_equal_approx(expected), name + " actual=" + str(actual) + " expected=" + str(expected))

# The decoded textholder art is41px tall; the native renderer may use any number of slices.
# Validate sampled bitmap rows and full painted coverage, not one TextureRect/crop construction.
func assert_dropdown_member_image(ui: Node, name: String) -> void:
	var target := control(ui, name).get_global_rect()
	for frame in range(120):
		var painted_width := 0.0
		for node in ui.find_children("*", "TextureRect", true, false):
			var part := node as TextureRect
			var rect := part.get_global_rect()
			var covers_backdrop_row := is_equal_approx(rect.position.y, target.position.y) and is_equal_approx(rect.size.y, target.size.y) and target.encloses(rect)
			if covers_backdrop_row and part.texture != null:
				var image: Image = part.texture.get_image()
				if image.get_height() != 41:
					push_error("Dropdown samples unrelated button-sheet rows: " + name + " image=" + str(image.get_size()))
					quit(1)
				assert(image.get_height() == 41)
				painted_width += rect.size.x
		if is_equal_approx(painted_width, target.size.x):
			return
		await process_frame
		await RenderingServer.frame_post_draw
	push_error("Dropdown backdrop coverage missing after bounded asset wait: " + name)
	quit(1)

func capture_png(directory: String, filename: String) -> void:
	var image := root.get_texture().get_image()
	assert(image.get_size() == Vector2i(1920, 1080))
	assert(image.save_png(directory.path_join(filename)) == OK)

func settle() -> void:
	for frame in range(4):
		await process_frame
		await RenderingServer.frame_post_draw

func click(ui: Node, name: String) -> void:
	var point := control(ui, name).get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	Input.parse_input_event(motion)
	await process_frame
	for pressed in [true, false]:
		var mouse := InputEventMouseButton.new()
		mouse.button_index = MOUSE_BUTTON_LEFT
		mouse.position = point
		mouse.global_position = point
		mouse.pressed = pressed
		Input.parse_input_event(mouse)
		await process_frame
	await settle()

func key(code: int, unicode: int = 0, ctrl: bool = false) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = code
		event.unicode = unicode
		event.ctrl_pressed = ctrl
		event.pressed = pressed
		Input.parse_input_event(event)
		await process_frame

func type_text(ui: Node, name: String, text: String, submit: bool = true) -> void:
	await click(ui, name)
	await key(KEY_A, 0, true)
	for character in text:
		await key(character.unicode_at(0), character.unicode_at(0))
		assert(ui.call("sync_input") == "", "Mirror production host input polling before observing paint")
	if submit:
		await key(KEY_ENTER)
	await settle()
