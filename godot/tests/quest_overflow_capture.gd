extends SceneTree

var shots: String
var proof: Array = []

func _initialize() -> void:
	call_deferred("run_test")

func require(condition: bool, message: String = "Quest overflow assertion failed") -> void:
	if not condition:
		push_error(message)
		quit(1)

func settle() -> void:
	for i in range(4):
		await process_frame
	await RenderingServer.frame_post_draw

func mouse(position: Vector2, button: int, pressed: bool) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = position
	motion.global_position = position
	Input.parse_input_event(motion)
	var event := InputEventMouseButton.new()
	event.position = position
	event.global_position = position
	event.button_index = button
	event.pressed = pressed
	Input.parse_input_event(event)

func image(name: String) -> Image:
	await RenderingServer.frame_post_draw
	var capture := root.get_texture().get_image()
	require(capture.save_png(shots.path_join(name)) == OK)
	return capture

func drag_thumb(probe: Node, list: String, fraction: float) -> void:
	var thumb := node(probe, list + "ScrollThumb")
	var track := node(probe, list + "ScrollTrack")
	require(thumb.is_visible_in_tree())
	var grab := thumb.get_global_rect().get_center()
	mouse(grab, MOUSE_BUTTON_LEFT, true)
	await process_frame
	var track_rect := track.get_global_rect()
	var thumb_height := thumb.get_global_rect().size.y
	var motion := InputEventMouseMotion.new()
	motion.position = Vector2(grab.x, track_rect.position.y + fraction * (track_rect.size.y - thumb_height) + thumb_height / 2)
	motion.global_position = motion.position
	Input.parse_input_event(motion)
	await settle()
	mouse(motion.position, MOUSE_BUTTON_LEFT, false)
	await settle()

func node(probe: Node, name: String) -> Control:
	var control: Control = probe.find_child(name, true, false)
	require(control != null, "Missing " + name)
	return control

func assert_clipped_pixels(before: Image, after: Image, area: Rect2) -> void:
	# Content hiding may only change pixels inside its clipped viewport, not chrome/buttons.
	var outside_changes := 0
	var inside_changes := 0
	for y in range(before.get_height()):
		for x in range(before.get_width()):
			if before.get_pixel(x, y) == after.get_pixel(x, y):
				continue
			if area.grow(2).has_point(Vector2(x, y)):
				inside_changes += 1
			else:
				outside_changes += 1
	require(inside_changes > 100, "No rendered quest content in clipping comparison")
	require(outside_changes == 0, "Content escaped viewport: %d pixels" % outside_changes)

func run_case(forever: bool, page: String, long_text: bool) -> void:
	var probe = ClassDB.instantiate("UiAuditProbe")
	root.add_child(probe)
	require(probe.mount_quest_overflow(forever, page, long_text).is_empty())
	await settle()
	var list: String = {
		"log": "QuestLogDetailsScrollFrame", "detail": "QuestDetailScrollFrame",
		"progress": "QuestProgressScrollFrame", "reward": "QuestRewardScrollFrame"
	}[page]
	var child: String = list.replace("ScrollFrame", "ScrollChildFrame")
	var area := node(probe, list)
	var content := node(probe, child)
	var metrics: Vector3 = probe.quest_scroll_metrics(list)
	var scale: float = area.get_global_transform().get_scale().y
	require(area.clip_contents, "Quest viewport must clip native children")
	require(abs(metrics.y - max(0, ceil(metrics.z - area.size.y))) < 1, "Range does not cover native content")
	require(metrics.x == 0, "New page must begin at top")
	var skin := "forever" if forever else "modern"
	var prefix := "%s-%s-%s" % [skin, page, "long" if long_text else "short"]
	var top_image := await image(prefix + "-top.png")
	if long_text:
		var paragraph_name: String = {"log": "QuestLogDetailsDescription", "detail": "QuestInfoDescriptionText", "progress": "QuestProgressText", "reward": "QuestInfoRewardText"}[page]
		var paragraph: Label = node(probe, paragraph_name)
		print("PARAGRAPH ", prefix, " rect=", paragraph.get_global_rect(), " min=", paragraph.get_minimum_size(), " lines=", paragraph.get_line_count())
		require(abs(paragraph.size.y - paragraph.get_minimum_size().y) < 1, "Laid-out paragraph height differs from native shaping")
		paragraph.hide()
		await settle()
		assert_clipped_pixels(top_image, root.get_texture().get_image(), area.get_global_rect())
		paragraph.show()
		await settle()
	content.hide()
	await settle()
	require(not content.is_visible_in_tree())
	var hidden := root.get_texture().get_image()
	assert_clipped_pixels(top_image, hidden, area.get_global_rect())
	content.show()
	await settle()
	var start: float = content.get_global_rect().position.y
	mouse(area.get_global_rect().get_center(), MOUSE_BUTTON_WHEEL_DOWN, true)
	await settle()
	var moved: Vector3 = probe.quest_scroll_metrics(list)
	if long_text:
		require(metrics.y > 1000, "Long fixture did not overflow")
		require(moved.x == 30, "Native wheel did not scroll one pan extent")
		require(abs(start - content.get_global_rect().position.y - 30 * scale) < 1)
		await image(prefix + "-wheel.png")
		# Native thumb input must expose text in the middle, not merely move an empty rect.
		await drag_thumb(probe, list, 0.5)
		var middle: Vector3 = probe.quest_scroll_metrics(list)
		require(middle.x > 30 and middle.x < middle.y)
		var middle_image := await image(prefix + "-middle.png")
		content.hide()
		await settle()
		assert_clipped_pixels(middle_image, root.get_texture().get_image(), area.get_global_rect())
		content.show()
		await settle()
		await drag_thumb(probe, list, 1.0)
		var bottom: Vector3 = probe.quest_scroll_metrics(list)
		require(bottom.x == bottom.y, "Thumb did not reach full scroll range")
		var last_name := "QuestProgressItem2IconTexture" if page == "progress" else "QuestInfoMoneyText"
		var last := node(probe, last_name).get_global_rect()
		require(area.get_global_rect().grow(1).encloses(last), "Last reward is unreachable")
		var bottom_image := await image(prefix + "-bottom.png")
		content.hide()
		await settle()
		require(not content.is_visible_in_tree())
		assert_clipped_pixels(bottom_image, root.get_texture().get_image(), area.get_global_rect())
	else:
		require(metrics.y == 0 and moved.x == 0, "Short quest changed scrolling behavior")
		require(content.get_global_rect().position.y == start)
		require(not node(probe, list + "ScrollThumb").is_visible_in_tree())
		require(node(probe, list + "ScrollTrack").is_visible_in_tree())
		var last_name := "QuestProgressItem2IconTexture" if page == "progress" else "QuestInfoMoneyText"
		require(area.get_global_rect().grow(1).encloses(node(probe, last_name).get_global_rect()))
	proof.append({"skin": skin, "page": page, "long": long_text, "range": metrics.y, "content_height": metrics.z, "viewport_height": area.size.y, "wheel_offset": moved.x, "capture": prefix})
	print("PASS questoverflow ", proof.back())
	probe.free()
	await process_frame

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	shots = OS.get_environment("QUEST_OVERFLOW_SHOTS")
	require(not shots.is_empty())
	DirAccess.make_dir_recursive_absolute(shots)
	for forever in [false, true]:
		for page in ["log", "detail", "progress", "reward"]:
			for long_text in [false, true]:
				await run_case(forever, page, long_text)
	var file := FileAccess.open(shots.path_join("native-proof.json"), FileAccess.WRITE)
	file.store_string(JSON.stringify(proof, "\t"))
	print("QUEST_OVERFLOW_CAPTURE_PASS cases=", proof.size())
	quit(0)
