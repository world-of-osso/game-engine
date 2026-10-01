extends SceneTree

# Shared driver for `native_debug_screen_fixture <screen>`: mounts the real client with
# its `--screen` startup, runs the screen's live checks, signals `ready`, waits while
# the parent drives the public CLI (`ping`, `dump-scene`, `dump-tree`, `dump-ui-tree`,
# `screenshot`), then checks the CLI's replies and screenshot and exits 0.
# Screen scripts extend this file and override `check_live` and `check_cli`.
const SIZE := Vector2i(1280, 720)
# Pixels whose largest channel changes by more than this count as changed.
const PIXEL_DELTA := 0.08

var artifacts := ""
var client: Node
var failed := false

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	root.size = SIZE
	artifacts = OS.get_environment("GODOT_DEBUG_SCREEN_ARTIFACTS")
	if artifacts.is_empty() or not DirAccess.dir_exists_absolute(artifacts):
		fail("GODOT_DEBUG_SCREEN_ARTIFACTS must name an existing directory")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await check_live():
		return
	signal_parent("ready")
	if not await wait_parent("verify"):
		return
	if read_stdout("ping").strip_edges() != "pong":
		fail("CLI ping did not answer pong: " + read_stdout("ping"))
		return
	if not await check_cli():
		return
	print("PASS: debug screen flow")
	quit(0)

# Override: assertions on the live screen before the CLI runs.
func check_live() -> bool:
	return true

# Override: assertions on the CLI replies (`tree_reply`, `screenshot`).
func check_cli() -> bool:
	return true

func fail(message: String) -> void:
	if failed:
		return
	failed = true
	push_error("Debug screen fixture: " + message)
	quit(1)

func signal_parent(name: String) -> void:
	var file := FileAccess.open(artifacts.path_join(name), FileAccess.WRITE)
	if file == null:
		fail("Cannot write handshake " + name)
		return
	file.store_string("ready\n")
	file.close()

func wait_parent(name: String) -> bool:
	var deadline := Time.get_ticks_msec() + 120000
	while Time.get_ticks_msec() < deadline:
		if FileAccess.file_exists(artifacts.path_join(name)):
			return true
		await process_frame
	fail("Parent did not send " + name)
	return false

func read_stdout(name: String) -> String:
	return FileAccess.get_file_as_string(artifacts.path_join(name + ".stdout"))

# The `Tree` text of a `--json` CLI reply.
func tree_reply(name: String) -> String:
	var value: Variant = JSON.parse_string(read_stdout(name))
	if not value is Dictionary or not value.has("Tree") or not value.Tree is String:
		fail("CLI %s did not reply with a Tree: %s" % [name, read_stdout(name)])
		return ""
	return value.Tree

# The WebP the CLI `screenshot` command wrote.
func screenshot() -> Image:
	var image := Image.load_from_file(artifacts.path_join("screen.webp"))
	if image == null or image.is_empty():
		fail("CLI screenshot wrote no decodable WebP")
		return null
	image.convert(Image.FORMAT_RGBA8)
	return image

func capture() -> Image:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	image.convert(Image.FORMAT_RGBA8)
	return image

func save(image: Image, name: String) -> void:
	var saved := image.save_png(artifacts.path_join(name))
	if saved != OK:
		fail("Save %s: %s" % [name, error_string(saved)])

func changed_pixels(a: Image, b: Image) -> int:
	if a.get_size() != b.get_size():
		fail("Compared images differ in size: %s vs %s" % [a.get_size(), b.get_size()])
		return 0
	var count := 0
	for y in range(0, a.get_height(), 2):
		for x in range(0, a.get_width(), 2):
			var p := a.get_pixel(x, y)
			var q := b.get_pixel(x, y)
			if maxf(absf(p.r - q.r), maxf(absf(p.g - q.g), absf(p.b - q.b))) > PIXEL_DELTA:
				count += 1
	return count

# The client's child `name`, once startup has mounted it.
func wait_node(name: String) -> Node:
	var deadline := Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		var node := client.get_node_or_null(name)
		if node != null:
			return node
		await process_frame
	fail("Startup did not mount " + name)
	return null

# Pixels where both `a` and `b` differ from `base`: what both show over it.
func shared_changed_pixels(a: Image, b: Image, base: Image) -> int:
	var count := 0
	for y in range(0, base.get_height(), 2):
		for x in range(0, base.get_width(), 2):
			if differs(a.get_pixel(x, y), base.get_pixel(x, y)) and differs(b.get_pixel(x, y), base.get_pixel(x, y)):
				count += 1
	return count

func differs(p: Color, q: Color) -> bool:
	return maxf(absf(p.r - q.r), maxf(absf(p.g - q.g), absf(p.b - q.b))) > PIXEL_DELTA

func settle(ms: int) -> void:
	var until := Time.get_ticks_msec() + ms
	while Time.get_ticks_msec() < until:
		await process_frame

func push_key(code: Key) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = code
		event.physical_keycode = code
		event.pressed = pressed
		root.push_input(event, true)

func mouse_button(button: MouseButton, pressed: bool, at := Vector2(640, 360)) -> void:
	var event := InputEventMouseButton.new()
	event.position = at
	event.button_index = button
	event.pressed = pressed
	root.push_input(event, true)

func drag(button: MouseButton, total: Vector2) -> void:
	mouse_button(button, true)
	var motion := InputEventMouseMotion.new()
	motion.position = Vector2(640, 360)
	motion.relative = total
	motion.button_mask = MOUSE_BUTTON_MASK_LEFT if button == MOUSE_BUTTON_LEFT else MOUSE_BUTTON_MASK_RIGHT
	root.push_input(motion, true)
	mouse_button(button, false)
