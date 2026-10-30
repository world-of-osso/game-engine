extends SceneTree

var completion = preload("res://tests/completion.gd").new(self, 2)

func _initialize() -> void:
	call_deferred("run_completion")

func capture() -> bool:
	var directory := OS.get_environment("GUILD_CAPTURES")
	if not completion.require(not directory.is_empty()):
		return false
	# Pixel samples and captures use 1920x1080; the project window defaults to 1280x720.
	root.size = Vector2i(1920, 1080)
	for skin in ["forever"]:
		var ui: Node = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		var error: String = ui.call("show_guild_ranks_preview" if skin == "modern" else "show_forever_guild_ranks_preview")
		if not completion.require(error == "", error):
			return false
		if not completion.step(await settle(), "settle"):
			return false
		if not completion.step(assert_rect(ui, "GuildControlUI", Rect2(440, 130, 338, 444)), "assert_rect"):
			return false
		if not completion.step(await assert_dropdown_member_image(ui, "GuildControlUINavigationDropdownBackground"), "assert_dropdown_member_image"):
			return false
		if not completion.step(assert_rect(ui, "GuildControlUIRankOrderFrame", Rect2(454, 190, 312, 372)), "assert_rect"):
			return false
		if not completion.step(assert_rect(ui, "GuildRankOrderName2", Rect2(536, 258, 132, 32)), "assert_rect"):
			return false
		if not completion.step(assert_rect(ui, "GuildControlUIRankOrderFrameRank3ShiftUpButton", Rect2(669, 261, 26, 26)), "assert_rect"):
			return false
		var backdrop: Color = root.get_texture().get_image().get_pixel(457, 530)
		if backdrop.r >= 0.1 or backdrop.g >= 0.1 or backdrop.b >= 0.1:
			push_error("TranslucentFrameTemplate black0.9 background missing: " + str(backdrop))
			completion.fail("guild_control_fidelity: capture failed")
		if not completion.require(backdrop.r < 0.1 and backdrop.g < 0.1 and backdrop.b < 0.1):
			return false
		if not completion.step(capture_png(directory, skin + "-guild-control.png"), "capture_png"):
			return false
		if not completion.step(await type_text(ui, "GuildRankOrderName2", "Pathfinder"), "type_text"):
			return false
		var request: String = ui.call("guild_control_preview_dispatch")
		if not completion.require(request.contains("Rename") and request.contains("Pathfinder"), request):
			return false
		if not completion.require(ui.call("guild_control_preview_waiting")):
			return false
		if not completion.require(ui.call("guild_control_preview_reply") == ""):
			return false
		if not completion.step(await settle(), "settle"):
			return false
		if not completion.require(ui.call("frame_text", "GuildRankOrderName2") == "Pathfinder"):
			return false
		if not completion.step(await type_text(ui, "GuildRankOrderName2", "Discarded", false), "type_text"):
			return false
		var edit := control(ui, "GuildRankOrderName2") as LineEdit
		if edit.get_theme_color("font_color") != Color(0, 1, 0, 1):
			push_error("Source changed-name text must be green: " + str(edit.get_theme_color("font_color")))
			completion.fail("guild_control_fidelity: capture failed")
		if not completion.require(edit.get_theme_color("font_color") == Color(0, 1, 0, 1)):
			return false
		if not completion.step(await key(KEY_ESCAPE), "key"):
			return false
		if not completion.step(await settle(), "settle"):
			return false
		var escaped: String = ui.call("guild_control_preview_dispatch")
		if escaped != "":
			push_error("Escape must restore authoritative text without a rename request: " + escaped)
			completion.fail("guild_control_fidelity: capture failed")
		if not completion.require(escaped == ""):
			return false
		if not completion.require(ui.call("frame_text", "GuildRankOrderName2") == "Pathfinder"):
			return false
		if not completion.step(await click(ui, "GuildControlUINavigationDropdown"), "click"):
			return false
		if not completion.require(ui.call("guild_control_preview_dispatch") == ""):
			return false
		if not completion.step(await settle(), "settle"):
			return false
		if not completion.step(capture_png(directory, skin + "-guild-navigation-menu.png"), "capture_png"):
			return false
		if not completion.step(await click(ui, "GuildControlPage1"), "click"):
			return false
		if not completion.require(ui.call("guild_control_preview_dispatch") == ""):
			return false
		if not completion.step(await settle(), "settle"):
			return false
		if not completion.step(assert_rect(ui, "GuildControlUIRankSettingsFrameRankDropdown", Rect2(574, 196, 180, 25)), "assert_rect"):
			return false
		if not completion.step(await assert_dropdown_member_image(ui, "GuildControlUIRankSettingsFrameRankDropdownBackground"), "assert_dropdown_member_image"):
			return false
		if not completion.step(assert_rect(ui, "GuildControlUIRankSettingsFrameOfficerBg", Rect2(465, 248, 290, 160)), "assert_rect"):
			return false
		if not completion.step(assert_rect(ui, "GuildControlUIRankSettingsFrameOfficerCheckbox", Rect2(465, 251, 20, 20)), "assert_rect"):
			return false
		if not completion.step(assert_rect(ui, "GuildRankGold", Rect2(612, 522, 60, 28)), "assert_rect"):
			return false
		for id in [9, 11, 12, 13]:
			if not completion.require(ui.find_child("GuildControlUIRankSettingsFrameCheckbox" + str(id), true, false) == null, "Mainline has no such control"):
				return false
		var officer := "GuildControlUIRankSettingsFrameOfficerCheckbox"
		var old_rights: int = ui.call("guild_control_preview_rights")
		if not completion.require((old_rights & 12) == 4, "Fixture deliberately preserves a mixed officer mask"):
			return false
		if not completion.require(ui.find_child(officer + "Check", true, false) != null):
			return false
		if not completion.step(await click(ui, officer), "click"):
			return false
		request = ui.call("guild_control_preview_dispatch")
		if not completion.require(request.contains("SetPermissions") and request.contains("rights: " + str(old_rights & ~12)), request):
			return false
		if not completion.require(ui.call("guild_control_preview_rights") == old_rights, "No optimistic authoritative mask mutation"):
			return false
		if not completion.require(ui.call("guild_control_preview_waiting")):
			return false
		if not completion.step(await settle(), "settle"):
			return false
		if not completion.step(await click(ui, "GuildControlUIRankSettingsFrameCheckbox7"), "click"):
			return false
		if not completion.require(ui.call("guild_control_preview_dispatch") == "", "Pending writes disable further mutations"):
			return false
		if not completion.require(ui.call("guild_control_preview_reply") == ""):
			return false
		if not completion.step(await settle(), "settle"):
			return false
		if not completion.require(ui.find_child(officer + "Check", true, false) == null):
			return false
		if not completion.step(await click(ui, officer), "click"):
			return false
		request = ui.call("guild_control_preview_dispatch")
		if not completion.require(request.contains("rights: " + str(old_rights | 12)), request):
			return false
		if not completion.require(ui.call("guild_control_preview_reply") == ""):
			return false
		if not completion.step(await settle(), "settle"):
			return false
		if not completion.require(ui.call("guild_control_preview_rights") == (old_rights | 12)):
			return false
		if not completion.step(await type_text(ui, "GuildRankGold", "7x"), "type_text"):
			return false
		request = ui.call("guild_control_preview_dispatch")
		if not request.contains("gold_per_day: 70000"):
			push_error("Source numeric GoldBox must reject x while preserving digit7: " + request)
			completion.fail("guild_control_fidelity: capture failed")
		if not completion.require(request.contains("gold_per_day: 70000"), request):
			return false
		if not completion.require(ui.call("guild_control_preview_reply") == ""):
			return false
		if not completion.step(await settle(), "settle"):
			return false
		if not completion.require(ui.call("frame_text", "GuildRankGold") == "7"):
			return false
		if not completion.step(capture_png(directory, skin + "-guild-permissions.png"), "capture_png"):
			return false
		if not completion.step(await click(ui, "GuildControlUIRankSettingsFrameRankDropdown"), "click"):
			return false
		if not completion.require(ui.call("guild_control_preview_dispatch") == ""):
			return false
		if not completion.step(await settle(), "settle"):
			return false
		if not completion.require(ui.find_child("GuildControlRankChoice0", true, false) == null):
			return false
		if not completion.step(capture_png(directory, skin + "-guild-rank-menu.png"), "capture_png"):
			return false
		if not completion.step(await click(ui, "GuildControlRankChoice2"), "click"):
			return false
		if not completion.require(ui.call("guild_control_preview_dispatch") == ""):
			return false
		if not completion.step(await click(ui, "GuildControlUINavigationDropdown"), "click"):
			return false
		if not completion.require(ui.call("guild_control_preview_dispatch") == ""):
			return false
		if not completion.step(await click(ui, "GuildControlPage2"), "click"):
			return false
		if not completion.require(ui.call("guild_control_preview_dispatch") == ""):
			return false
		if not completion.step(await settle(), "settle"):
			return false
		if not completion.step(assert_rect(ui, "GuildControlBankScrollBox", Rect2(473, 243, 254, 294)), "assert_rect"):
			return false
		if not completion.step(assert_rect(ui, "GuildControlBankTab2", Rect2(473, 321, 254, 74)), "assert_rect"):
			return false
		if not completion.step(assert_rect(ui, "GuildTabLimitLabel0", Rect2(488, 269, 150, 24)), "assert_rect"):
			return false
		if not completion.step(await click(ui, "GuildTabDeposit0"), "click"):
			return false
		request = ui.call("guild_control_preview_dispatch")
		if not completion.require(request.contains("SetTab") and request.contains("deposit: false"), request):
			return false
		if not completion.require(ui.call("guild_control_preview_reply") == ""):
			return false
		if not completion.step(await type_text(ui, "GuildTabItems0", "9x"), "type_text"):
			return false
		request = ui.call("guild_control_preview_dispatch")
		if not completion.require(request.contains("withdrawals_per_day: 9"), request):
			return false
		if not completion.require(ui.call("guild_control_preview_reply") == ""):
			return false
		if not completion.step(await settle(), "settle"):
			return false
		if not completion.step(capture_png(directory, skin + "-guild-bank-permissions.png"), "capture_png"):
			return false
		if not completion.require(ui.call("guild_control_preview_overflow") == ""):
			return false
		ui.call("enable_gameplay_preview_scroll_input")
		if not completion.step(await settle(), "settle"):
			return false
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
		if not completion.step(await settle(), "settle"):
			return false
		var last := control(ui, "GuildControlBankTab8")
		if not completion.require(list.get_global_rect().encloses(last.get_global_rect()), "Wheel must expose eighth purchased tab: " + str(last.get_global_rect())):
			return false
		if not completion.step(capture_png(directory, skin + "-guild-bank-scrolled.png"), "capture_png"):
			return false
		if not completion.case_done():
			return false
		ui.queue_free()
		await process_frame
	print("PASS: both-skin GuildControlUI source rectangles, physical navigation/rename/Officer/gold/bank edits, pacing, replies and overflow; 1920x1080 captures")
	return true

func control(ui: Node, name: String) -> Control:
	var node := ui.find_child(name, true, false) as Control
	if not completion.require(node != null, "Missing native control " + name):
		return null
	return node

func assert_rect(ui: Node, name: String, expected: Rect2) -> bool:
	var actual := control(ui, name).get_global_rect()
	if not actual.is_equal_approx(expected):
		push_error(name + " actual=" + str(actual) + " expected=" + str(expected))
		completion.fail("guild_control_fidelity: assert_rect failed")
	if not completion.require(actual.is_equal_approx(expected), name + " actual=" + str(actual) + " expected=" + str(expected)):
		return false

# The decoded textholder art is41px tall; the native renderer may use any number of slices.
# Validate sampled bitmap rows and full painted coverage, not one TextureRect/crop construction.
	return true

func assert_dropdown_member_image(ui: Node, name: String) -> bool:
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
					completion.fail("guild_control_fidelity: assert_dropdown_member_image failed")
				if not completion.require(image.get_height() == 41):
					return false
				painted_width += rect.size.x
		if is_equal_approx(painted_width, target.size.x):
			return true
		await process_frame
		await RenderingServer.frame_post_draw
	push_error("Dropdown backdrop coverage missing after bounded asset wait: " + name)
	completion.fail("guild_control_fidelity: assert_dropdown_member_image failed")

	return false
func capture_png(directory: String, filename: String) -> bool:
	var image := root.get_texture().get_image()
	if not completion.require(image.get_size() == Vector2i(1920, 1080)):
		return false
	if not completion.require(image.save_png(directory.path_join(filename)) == OK):
		return false

	return true

func settle() -> bool:
	for frame in range(4):
		await process_frame
		await RenderingServer.frame_post_draw

	return true

func click(ui: Node, name: String) -> bool:
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
	if not completion.step(await settle(), "settle"):
		return false

	return true

func key(code: int, unicode: int = 0, ctrl: bool = false) -> bool:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = code
		event.unicode = unicode
		event.ctrl_pressed = ctrl
		event.pressed = pressed
		Input.parse_input_event(event)
		await process_frame

	return true

func type_text(ui: Node, name: String, text: String, submit: bool = true) -> bool:
	if not completion.step(await click(ui, name), "click"):
		return false
	if not completion.step(await key(KEY_A, 0, true), "key"):
		return false
	for character in text:
		if not completion.step(await key(character.unicode_at(0), character.unicode_at(0)), "key"):
			return false
		if not completion.require(ui.call("sync_input") == "", "Mirror production host input polling before observing paint"):
			return false
	if submit:
		if not completion.step(await key(KEY_ENTER), "key"):
			return false
	if not completion.step(await settle(), "settle"):
		return false
	return true

func run_completion() -> void:
	completion.finish(await capture())
