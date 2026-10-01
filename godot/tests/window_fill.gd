extends SceneTree

## The client fills its real window: the root viewport, its rendered texture and the
## login UI canvas (scaled back to physical pixels) all equal the compositor's window
## size, at startup and after every compositor resize. Needs a real display server
## (niri, cage); headless has no window. Environment:
##   WINDOW_FILL_SECS       how long to watch the window (default 5)
##   WINDOW_FILL_MIN_SIZES  distinct window sizes that must be seen, e.g. 3 while
##                          `niri msg action set-window-height/width` resizes it (default 1)
## Never sets root.size: a forced root size on a tiled Wayland window renders
## into a corner of the surface, the failure this guards.

const SETTLE_FRAMES := 2

var client: Node

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	if DisplayServer.get_name() == "headless":
		fail("window_fill needs a real window, not the headless display server")
		return
	var secs := float(OS.get_environment("WINDOW_FILL_SECS")) if OS.get_environment("WINDOW_FILL_SECS") != "" else 5.0
	var min_sizes := int(OS.get_environment("WINDOW_FILL_MIN_SIZES")) if OS.get_environment("WINDOW_FILL_MIN_SIZES") != "" else 1
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var login_deadline := Time.get_ticks_msec() + 20000
	while ui_canvas_physical_size() == Vector2.ZERO:
		if Time.get_ticks_msec() > login_deadline:
			fail("LoginUI RegistryCanvas never appeared")
			return
		await process_frame
	var sizes: Array[Vector2i] = []
	var last := Vector2i.ZERO
	var stable := 0
	var deadline := Time.get_ticks_msec() + int(secs * 1000.0)
	while Time.get_ticks_msec() < deadline:
		await RenderingServer.frame_post_draw
		var window := DisplayServer.window_get_size()
		stable = stable + 1 if window == last else 0
		last = window
		# A configure reaches the viewport, then the UI relayout, on the following frames.
		if stable < SETTLE_FRAMES:
			continue
		var mismatch := fill_mismatch(window)
		if mismatch != "":
			fail("window %s: %s" % [window, mismatch])
			return
		if not sizes.has(window):
			sizes.append(window)
			print("WINDOW_FILL size=%s root=%s ui_canvas=%s" % [window, root.size, ui_canvas_physical_size()])
	if sizes.size() < min_sizes:
		fail("saw %d window sizes %s, need %d" % [sizes.size(), sizes, min_sizes])
		return
	client.free()
	print("PASS: viewport, rendered texture and UI canvas followed window sizes %s" % [sizes])
	quit(0)

func fill_mismatch(window: Vector2i) -> String:
	if root.size != window or Vector2i(root.get_visible_rect().size) != window:
		return "root viewport %s, visible %s" % [root.size, root.get_visible_rect().size]
	var texture := Vector2i(root.get_texture().get_size())
	if texture != window:
		return "rendered texture %s" % texture
	var canvas := ui_canvas_physical_size()
	if canvas.distance_to(Vector2(window)) > 1.0:
		return "login UI canvas covers %s physical pixels" % canvas
	return ""

## The LoginUI RegistryCanvas size times its scale: the physical area the UI lays out in.
func ui_canvas_physical_size() -> Vector2:
	var ui := client.get_node_or_null("LoginUI")
	var canvas := ui.find_child("RegistryCanvas", true, false) as Control if ui != null else null
	if canvas == null:
		return Vector2.ZERO
	return canvas.size * canvas.scale

func fail(message: String) -> void:
	push_error(message)
	print("FAIL: ", message)
	if client != null:
		client.free()
	quit(1)
