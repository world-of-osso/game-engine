extends SceneTree

# `--screen particledebug` through the real startup path: the original torch, then (Tab)
# the portal and Frostbolt missile. For each, emitters spawn and draw, number keys switch
# emitters and their pools and pixels off, 0 restores them; the orbit camera follows
# drag and wheel.
# Run: godot --path godot -s res://tests/particle_debug_screen.gd -- --screen particledebug
const TORCH := 145304
const PORTAL := 197007
const FROSTBOLT := 1598570
const WAIT_MS := 60000
# Pixels whose largest channel changes by more than this count as particle pixels.
const PIXEL_DELTA := 0.08
const MIN_PARTICLE_PIXELS := 150

var client: Node
var scene: Node
var shots := ""

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	shots = OS.get_environment("GODOT_PARTICLE_SCREENSHOTS")
	if shots.is_empty() or not DirAccess.dir_exists_absolute(shots):
		fail("Particle debug fixture requires an existing GODOT_PARTICLE_SCREENSHOTS directory")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	scene = client.get_node_or_null("ParticleDebug")
	if scene == null:
		fail("--screen particledebug did not open the particle debug scene")
		return
	var login := client.get_node_or_null("LoginUI") as CanvasLayer
	if login != null and login.visible:
		fail("Login UI is still visible over the particle debug scene")
		return
	if client.account_state().reply_received:
		fail("Particle debug screen contacted a server")
		return
	if not await check_torch_overlay():
		return
	if not await check_emitters(TORCH):
		return
	if not await check_orbit():
		return
	for model in [PORTAL, FROSTBOLT]:
		push_key(KEY_TAB)
		if not await check_emitters(model):
			return
	print("PASS: particle debug screen")
	quit(0)

func check_torch_overlay() -> bool:
	if not await wait_drawn(TORCH):
		return false
	var state: Dictionary = scene.debug_state()
	if not state.missing_textures.is_empty():
		fail("Torch textures missing: " + str(state.missing_textures))
		return false
	if not state.overlay.contains("Model: club_1h_torch_a_01 (145304)") or not state.overlay.contains("Emitter #0 [on]\nblend=4 type=1"):
		fail("Overlay lacks the torch emitter info: " + state.overlay)
		return false
	return true

func check_emitters(model: int) -> bool:
	if not await wait_drawn(model):
		return false
	await settle(1500)
	var state: Dictionary = scene.debug_state()
	var enabled: PackedInt32Array = state.enabled
	await save_shot("%d-on.png" % model)
	print("FIXTURE PARTICLE_MODEL ", state_summary())
	scene.get_node("Overlay").visible = false
	var lit_scene := await save_shot("%d-on-no-overlay.png" % model)
	scene.get_node("Overlay").visible = true
	for index in enabled:
		push_key(KEY_1 + index)
	await settle(300)
	state = scene.debug_state()
	var pools := scene.find_children("Particles*", "MultiMeshInstance3D", true, false)
	if state.drawn != 0 or not state.enabled.is_empty() or not pools.is_empty():
		fail("Toggling every emitter off left particles: %s pools=%d" % [state_summary(), pools.size()])
		return false
	if not state.overlay.contains("Emitter #0 [off]"):
		fail("Overlay does not show emitter #0 as off")
		return false
	await save_shot("%d-off.png" % model)
	scene.get_node("Overlay").visible = false
	var dark := await save_shot("%d-off-no-overlay.png" % model)
	scene.get_node("Overlay").visible = true
	var particle_pixels := changed_pixels(lit_scene, dark)
	print("FIXTURE PARTICLE_PIXELS ", model, " ", particle_pixels)
	if particle_pixels < MIN_PARTICLE_PIXELS:
		fail("Model %d particles changed only %d pixels" % [model, particle_pixels])
		return false
	push_key(KEY_0)
	if not await wait_drawn(model):
		return false
	if enabled.size() > 1:
		push_key(KEY_1 + enabled[0])
		await settle(300)
		state = scene.debug_state()
		var first := scene.find_child("Particles%d_%d" % [model, enabled[0]], true, false)
		var second := scene.find_child("Particles%d_%d" % [model, enabled[1]], true, false)
		if enabled[0] in state.enabled or first != null or second == null or state.drawn == 0:
			fail("Emitter key did not switch off only emitter #%d: " % enabled[0] + state_summary())
			return false
		push_key(KEY_0)
		if not await wait_drawn(model):
			return false
	return true

func check_orbit() -> bool:
	var camera := scene.get_node("Camera") as Camera3D
	var start := camera.global_position
	var focus := Vector3(0.0, 0.5, 0.0)
	mouse_button(MOUSE_BUTTON_LEFT, true)
	for step in 10:
		var motion := InputEventMouseMotion.new()
		motion.position = Vector2(640 + step * 20, 360)
		motion.relative = Vector2(20, 0)
		motion.button_mask = MOUSE_BUTTON_MASK_LEFT
		root.push_input(motion, true)
		await process_frame
	mouse_button(MOUSE_BUTTON_LEFT, false)
	await process_frame
	var turned := camera.global_position
	if turned.distance_to(start) < 0.5 or absf(turned.distance_to(focus) - start.distance_to(focus)) > 0.01:
		fail("Drag did not orbit at constant distance: %s -> %s" % [start, turned])
		return false
	for notch in 3:
		mouse_button(MOUSE_BUTTON_WHEEL_UP, true)
		mouse_button(MOUSE_BUTTON_WHEEL_UP, false)
	await settle(500)
	var zoomed := camera.global_position.distance_to(focus)
	if absf(zoomed - (turned.distance_to(focus) - 1.2)) > 0.02:
		fail("Wheel did not zoom 3 x 0.4 yd: %.3f -> %.3f" % [turned.distance_to(focus), zoomed])
		return false
	await save_shot("%d-orbit.png" % TORCH)
	print("FIXTURE PARTICLE_ORBIT ", start, " -> ", camera.global_position)
	return true

func wait_drawn(model: int) -> bool:
	var deadline := Time.get_ticks_msec() + WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = scene.debug_state()
		if state.get("model_fdid", 0) == model and state.drawn > 0:
			return true
	fail("Model %d drew no particles: %s" % [model, state_summary()])
	return false

func changed_pixels(a: Image, b: Image) -> int:
	var count := 0
	for y in range(0, a.get_height(), 2):
		for x in range(0, a.get_width(), 2):
			var p := a.get_pixel(x, y)
			var q := b.get_pixel(x, y)
			if maxf(absf(p.r - q.r), maxf(absf(p.g - q.g), absf(p.b - q.b))) > PIXEL_DELTA:
				count += 1
	return count

func save_shot(name: String) -> Image:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var path := shots.path_join(name)
	var saved := image.save_png(path)
	if saved != OK:
		fail("Save %s: %s" % [path, error_string(saved)])
	return image

func settle(ms: int) -> void:
	var until := Time.get_ticks_msec() + ms
	while Time.get_ticks_msec() < until:
		await process_frame

func state_summary() -> String:
	var state: Dictionary = scene.debug_state()
	return "model=%s emitters=%s enabled=%s drawn=%s pools=%s" % [state.get("model_fdid"), state.get("emitters"), state.get("enabled"), state.drawn, state.pools]

func push_key(code: Key) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = code
		event.physical_keycode = code
		event.pressed = pressed
		root.push_input(event, true)

func mouse_button(button: MouseButton, pressed: bool) -> void:
	var event := InputEventMouseButton.new()
	event.position = Vector2(640, 360)
	event.button_index = button
	event.pressed = pressed
	event.factor = 1.0
	root.push_input(event, true)

func fail(message: String) -> void:
	push_error(message)
	quit(1)
