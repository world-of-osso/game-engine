extends "res://tests/world_quest_flow.gd"

# Rendered timing regression: the real button's error expires after three seconds,
# as UIErrorsFrame does. At 2 FPS the old three-frame click settle loses the image.
class CaptureClient extends Node:
	func objective_tracker_state() -> Dictionary:
		return {"visible": false}

func run_test() -> void:
	Engine.max_fps = 2
	root.size = Vector2i(1920, 1080)
	shots = OS.get_environment("QF_SHOTS").trim_suffix("/") + "/"
	DirAccess.make_dir_recursive_absolute(shots)
	client = CaptureClient.new()
	root.add_child(client)
	var line := Label.new()
	line.position = Vector2(704, 122)
	line.size = Vector2(512, 60)
	line.text = MUST_CHOOSE
	line.add_theme_color_override("font_color", Color(1.0, 0.1, 0.1))
	line.hide()
	root.add_child(line)
	var button := Button.new()
	button.position = Vector2(100, 100)
	button.size = Vector2(180, 40)
	button.text = "Complete Quest"
	root.add_child(button)
	button.pressed.connect(func():
		line.show()
		create_timer(3.0).timeout.connect(line.hide)
	)
	await frames(2)
	var settle := int(OS.get_environment("QF_ERROR_SETTLE"))
	await click(button.get_global_rect().get_center(), MOUSE_BUTTON_LEFT, settle)
	if not await wait_frames(func(): return line.is_visible_in_tree(), "transient error"):
		return
	await capture("error-timing.png")
	var image := Image.load_from_file(shots + "error-timing.png")
	var red := 0
	for y in range(122, 182):
		for x in range(704, 1216):
			var pixel := image.get_pixel(x, y)
			if pixel.r > 0.7 and pixel.r > pixel.g * 2.0 and pixel.r > pixel.b * 2.0:
				red += 1
	if red < 10:
		push_error("Transient error disappeared before capture: red pixels=%d settle=%d" % [red, settle])
		quit(1)
		return
	print("FIXTURE QUEST_ERROR_CAPTURE_PASS red_pixels=", red, " settle=", settle)
	quit(0)
