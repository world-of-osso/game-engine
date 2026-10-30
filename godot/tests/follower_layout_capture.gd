extends SceneTree

# Same native Group Finder projection as the live window; offline catalog snapshot.
func _initialize() -> void:
	call_deferred("capture")

func capture() -> void:
	var directory := OS.get_environment("GODOT_FOLLOWER_LAYOUT_CAPTURES")
	if directory.is_empty():
		push_error("GODOT_FOLLOWER_LAYOUT_CAPTURES required")
		quit(1)
		return
	if root.size != Vector2i(1920, 1080):
		push_error("Capture requires compositor output 1920x1080")
		quit(1)
		return
	for skin in ["forever"]:
		for menu in [false, true]:
			var ui = ClassDB.instantiate("RegistryUi")
			root.add_child(ui)
			var error = ui.call("show_follower_layout_preview", skin == "forever", menu)
			if error != "":
				push_error(error)
				quit(1)
				return
			for frame in range(120):
				await process_frame
				await RenderingServer.frame_post_draw
			if OS.get_environment("GODOT_FOLLOWER_LAYOUT_ASSERT") == "1":
				if not verify_xml_geometry(ui):
					quit(1)
					return
			var image := root.get_texture().get_image()
			if OS.get_environment("GODOT_FOLLOWER_LAYOUT_ASSERT") == "1":
				if not verify_description_pixels(image):
					quit(1)
					return
			var suffix := "-dropdown" if menu else ""
			var result := image.save_png(directory.path_join(skin + suffix + ".png"))
			if result != OK:
				push_error("Cannot save capture: " + str(result))
				quit(1)
				return
			ui.queue_free()
			await process_frame
	print("PASS both-skin Group Finder capture at 1920x1080")
	quit(0)

# PVEFrame.xml + LFDFrame.xml values, observed after native Control layout.
func verify_xml_geometry(ui: Node) -> bool:
	var rects := {
		"PVEFrame": Rect2(100, 84, 563, 428),
		"PVEFrameLeftInset": Rect2(104, 108, 217, 400),
		"LFDParentFrameInset": Rect2(328, 144, 328, 342),
		"LFDQueueFrameBackground": Rect2(330, 230, 512, 256),
		"GroupFinderFrameGroupButton1": Rect2(110, 185, 203, 60),
		"GroupFinderFrameGroupButton2": Rect2(110, 275, 203, 60),
		"GroupFinderFrameGroupButton3": Rect2(110, 365, 203, 60),
		"GroupFinderFrameGroupButton1Ring": Rect2(98, 168, 95, 96),
		"GroupFinderFrameGroupButton1Icon": Rect2(112.5, 183, 66, 66),
		"LFDQueueFrameTypeDropdown": Rect2(455, 200, 200, 25),
		"LFDQueueFrameRoleButtonLeader": Rect2(598, 130, 48, 48),
		"LFDQueueFrameRoleButtonTankBackground": Rect2(358, 114, 80, 80),
		"LFDQueueFrameRoleButtonHealerBackground": Rect2(431, 114, 80, 80),
		"LFDQueueFrameRoleButtonDPSBackground": Rect2(502, 114, 80, 80),
	}
	for name in rects:
		var node = ui.find_child(name, true, false)
		# Native texture projection snaps half-pixel XML centers to whole pixels.
		var expected := Rect2(rects[name].position.round(), rects[name].size)
		if node == null or not node.get_global_rect().is_equal_approx(expected):
			push_error("XML geometry mismatch " + name + ": " + str(node.get_global_rect() if node else null) + " expected " + str(rects[name]))
			return false
	var leader = ui.find_child("LFDQueueFrameRoleButtonLeader", true, false)
	if leader.disabled or ui.find_child("LFDQueueFrameRoleButtonLeaderCheckBox", true, false) == null:
		push_error("Retail solo leader must remain visible and enabled, with its checkbox")
		return false
	var highlight = ui.find_child("GroupFinderFrameGroupButton1Highlight", true, false)
	if highlight == null or not is_equal_approx(highlight.modulate.a, 0.8):
		push_error("GroupFinderGroupButtonTemplate highlight must retain source alpha .8")
		return false
	var menu = ui.find_child("LFDQueueFrameTypeDropdownMenu", true, false)
	if menu != null:
		# LFDFrame.lua SetupMenu creates a selected radio, with no title/divider.
		if not menu.get_global_rect().is_equal_approx(Rect2(455, 225, 200, 43)):
			push_error("Follower type menu must use a single headerless Retail radio row")
			return false
		if ui.find_child("LFDQueueFrameTypeTitle", true, false) != null or ui.find_child("LFDQueueFrameTypeDivider", true, false) != null:
			push_error("Follower type menu must not invent a header/divider")
			return false
		if ui.find_child("LFDQueueFrameTypeFollowerTick", true, false) == null:
			push_error("Follower type must render its selected radio tick")
			return false
	var inset = ui.find_child("LFDParentFrameInset", true, false)
	if not inset.clip_contents:
		push_error("LFDParentFrameInset must clip native quest paper")
		return false
	var tab1 = ui.find_child("PVEFrameTab1", true, false)
	var tab2 = ui.find_child("PVEFrameTab2", true, false)
	var tab3 = ui.find_child("PVEFrameTab3", true, false)
	if tab1 == null or tab2 == null or tab3 == null:
		push_error("All three PanelTabButtonTemplate tabs required")
		return false
	var first: Rect2 = tab1.get_global_rect()
	var second: Rect2 = tab2.get_global_rect()
	var third: Rect2 = tab3.get_global_rect()
	if first.position != Vector2(119, 510) or first.size.y != 32 or second.position != Vector2(first.end.x - 16, first.position.y) or third.position != Vector2(second.end.x - 16, second.position.y):
		push_error("PanelTabButtonTemplate BOTTOMLEFT19,-30 and LEFT-to-RIGHT -16 anchors differ")
		return false
	return true

# Native parchment must sit behind the white description, not cover sibling text.
func verify_description_pixels(image: Image) -> bool:
	var white_pixels := 0
	for y in range(267, 304):
		for x in range(339, 619):
			var color := image.get_pixel(x, y)
			if color.r > 0.9 and color.g > 0.9 and color.b > 0.9:
				white_pixels += 1
	if white_pixels < 20:
		push_error("Native quest paper covered the follower description")
		return false
	return true
