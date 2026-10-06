extends SceneTree

var shots: String
var proof: Array = []

func _initialize() -> void:
	call_deferred("run_test")

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
	assert(capture.save_png(shots.path_join(name)) == OK)
	return capture

func node(probe: Node, name: String) -> Control:
	var control: Control = probe.find_child(name, true, false)
	assert(control != null, "Missing " + name)
	return control

func assert_clipped_pixels(before: Image, after: Image, area: Rect2) -> void:
	# Content hiding may only change pixels inside its clipped viewport, not chrome/buttons.
	var outside_changes := 0
	for y in range(before.get_height()):
		for x in range(before.get_width()):
			if not area.grow(2).has_point(Vector2(x, y)) and before.get_pixel(x, y) != after.get_pixel(x, y):
				outside_changes += 1
	assert(outside_changes == 0, "Content escaped viewport: %d pixels" % outside_changes)

func run_case(forever: bool, page: String, long_text: bool) -> void:
	var probe = ClassDB.instantiate("UiAuditProbe")
	root.add_child(probe)
	assert(probe.mount_quest_overflow(forever, page, long_text).is_empty())
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
	assert(area.clip_contents, "Quest viewport must clip native children")
	assert(abs(metrics.y - max(0, ceil(metrics.z - area.size.y))) < 1, "Range does not cover native content")
	assert(metrics.x == 0, "New page must begin at top")
	var skin := "forever" if forever else "modern"
	var prefix := "%s-%s-%s" % [skin, page, "long" if long_text else "short"]
	var top_image := await image(prefix + "-top.png")
	content.hide()
	await settle()
	var hidden := root.get_texture().get_image()
	assert_clipped_pixels(top_image, hidden, area.get_global_rect())
	content.show()
	await settle()
	var start: float = content.get_global_rect().position.y
	mouse(area.get_global_rect().get_center(), MOUSE_BUTTON_WHEEL_DOWN, true)
	await settle()
	var moved: Vector3 = probe.quest_scroll_metrics(list)
	if long_text:
		assert(metrics.y > 1000, "Long fixture did not overflow")
		assert(moved.x == 30, "Native wheel did not scroll one pan extent")
		assert(abs(start - content.get_global_rect().position.y - 30 * scale) < 1)
		await image(prefix + "-wheel.png")
		# Drive the actual scrollbar thumb, not a test-only setter.
		var thumb := node(probe, list + "ScrollThumb")
		var track := node(probe, list + "ScrollTrack")
		assert(thumb.is_visible_in_tree())
		var grab := thumb.get_global_rect().get_center()
		mouse(grab, MOUSE_BUTTON_LEFT, true)
		await process_frame
		var motion := InputEventMouseMotion.new()
		motion.position = Vector2(grab.x, track.get_global_rect().end.y + thumb.size.y * scale)
		motion.global_position = motion.position
		Input.parse_input_event(motion)
		await settle()
		mouse(motion.position, MOUSE_BUTTON_LEFT, false)
		await settle()
		var bottom: Vector3 = probe.quest_scroll_metrics(list)
		assert(bottom.x == bottom.y, "Thumb did not reach full scroll range")
		var last_name := "QuestProgressItem2IconTexture" if page == "progress" else "QuestInfoMoneyText"
		var last := node(probe, last_name).get_global_rect()
		assert(area.get_global_rect().grow(1).encloses(last), "Last reward is unreachable")
		var bottom_image := await image(prefix + "-bottom.png")
		content.hide()
		await settle()
		assert_clipped_pixels(bottom_image, root.get_texture().get_image(), area.get_global_rect())
	else:
		assert(metrics.y == 0 and moved.x == 0, "Short quest changed scrolling behavior")
		assert(content.get_global_rect().position.y == start)
		assert(not node(probe, list + "ScrollThumb").is_visible_in_tree())
		assert(node(probe, list + "ScrollTrack").is_visible_in_tree())
		var last_name := "QuestProgressItem2IconTexture" if page == "progress" else "QuestInfoMoneyText"
		assert(area.get_global_rect().grow(1).encloses(node(probe, last_name).get_global_rect()))
	proof.append({"skin": skin, "page": page, "long": long_text, "range": metrics.y, "content_height": metrics.z, "viewport_height": area.size.y, "wheel_offset": moved.x, "capture": prefix})
	print("PASS questoverflow ", proof.back())
	probe.free()
	await process_frame

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	shots = OS.get_environment("QUEST_OVERFLOW_SHOTS")
	assert(not shots.is_empty())
	DirAccess.make_dir_recursive_absolute(shots)
	for forever in [false, true]:
		for page in ["log", "detail", "progress", "reward"]:
			for long_text in [false, true]:
				await run_case(forever, page, long_text)
	var file := FileAccess.open(shots.path_join("native-proof.json"), FileAccess.WRITE)
	file.store_string(JSON.stringify(proof, "\t"))
	print("QUEST_OVERFLOW_CAPTURE_PASS cases=", proof.size())
	quit(0)
