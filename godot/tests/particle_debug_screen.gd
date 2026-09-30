extends SceneTree

# `--screen particledebug` through the real startup path: the original torch, then (Tab)
# the portal and Frostbolt missile. For each, emitters spawn and draw, number keys switch
# emitters and their pools and pixels off, 0 restores them; the orbit camera follows
# drag and wheel. The torch's flame particles follow its authored colour and flipbook
# ramps, its point light lights the ground, and its billboarded halo faces the camera.
# Run: godot --path godot -s res://tests/particle_debug_screen.gd -- --screen particledebug
const TORCH := 145304
const PORTAL := 197007
const FROSTBOLT := 1598570
const WAIT_MS := 60000
# Pixels whose largest channel changes by more than this count as particle pixels.
const PIXEL_DELTA := 0.08
const MIN_PARTICLE_PIXELS := 150
const MIN_LIT_PIXELS := 3000
const MIN_HANDLE_PIXELS := 200
const MIN_HALO_PIXELS := 400
# club_1h_torch_a_01 light 0: bone 9 pivot in Godot axes above the model origin
# (0, 0.5, 0), diffuse (119, 74, 34) / 255 x 1.1, retail attenuation end 5.2667 yd.
const TORCH_LIGHT_POSITION := Vector3(0.57652, 0.50347, -0.00012)
const TORCH_LIGHT_COLOR := Color(0.51333, 0.31922, 0.14667)
const TORCH_LIGHT_RANGE := 5.26666
# Flame emitter keys: green 72 -> 138 -> 234 over life, cells 0 -> 7 | 8 -> 16 of 4x4.
const FLAME_GREEN := Vector2(72.0 / 255.0, 234.0 / 255.0)
const FLOATS_PER_PARTICLE := 20

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
	if not await check_flame_ramps():
		return
	if not await check_torch_light():
		return
	if not await check_halo_billboard():
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
	if not state.overlay.contains("Model: club_1h_torch_a_01 (145304)") or not state.overlay.contains("Emitter #0 [on]\nblend=4 type=1") or not state.overlay.contains("Light #0 point bone=9 color=(0.513, 0.319, 0.147) attenuation=1.667-5.267"):
		fail("Overlay lacks the torch emitter info: " + state.overlay)
		return false
	return true

# Every live flame particle's colour and atlas cell come from one point on the authored
# ramps, so a later cell never has a less yellow colour.
func check_flame_ramps() -> bool:
	await settle(1500)
	var pool := scene.find_child("Particles%d_0" % TORCH, true, false) as MultiMeshInstance3D
	var multimesh := pool.multimesh
	var buffer := multimesh.buffer
	var samples: Array[Vector2] = []
	for index in multimesh.visible_instance_count:
		var at := index * FLOATS_PER_PARTICLE
		var green := buffer[at + 13]
		if buffer[at + 15] < 0.01:
			continue
		var cell := roundi(buffer[at + 16] * 4.0) + 4 * roundi(buffer[at + 17] * 4.0)
		if green < FLAME_GREEN.x - 0.001 or green > FLAME_GREEN.y + 0.001 or absf(buffer[at + 12] - 1.0) > 0.13:
			fail("Flame particle colour off its ramp: %s" % [buffer.slice(at + 12, at + 16)])
			return false
		samples.append(Vector2(cell, green))
	var cells := {}
	for sample in samples:
		cells[sample.x] = true
		for other in samples:
			if sample.x < other.x and sample.y > other.y + 0.0001:
				fail("Flame cell %d is yellower than older cell %d" % [sample.x, other.x])
				return false
	if cells.size() < 4:
		fail("Flame flipbook shows only cells %s" % [cells.keys()])
		return false
	print("FIXTURE FLAME_RAMPS particles=%d cells=%s" % [samples.size(), cells.keys()])
	return true

func torch_node() -> Node3D:
	return scene.get_node("ParticleDebugModel%d" % TORCH) as Node3D

func check_torch_light() -> bool:
	var light := torch_node().get_node_or_null("M2Lights/M2Light0") as OmniLight3D
	if light == null:
		fail("Torch has no M2 point light")
		return false
	var color := light.light_color
	if light.global_position.distance_to(TORCH_LIGHT_POSITION) > 0.001 or absf(light.omni_range - TORCH_LIGHT_RANGE) > 0.001 or Vector3(color.r, color.g, color.b).distance_to(Vector3(TORCH_LIGHT_COLOR.r, TORCH_LIGHT_COLOR.g, TORCH_LIGHT_COLOR.b)) > 0.001:
		fail("Torch light at %s range %.4f colour %s" % [light.global_position, light.omni_range, color])
		return false
	# Without flame particles and the pulsing halo, only the light differs between the
	# shots; the screen's near-black ground is lightened so the light's reach shows.
	push_key(KEY_1)
	await settle(300)
	scene.get_node("Overlay").visible = false
	var halo := torch_node().get_node("Batch1") as MeshInstance3D
	halo.visible = false
	var ground := (scene.get_node("Ground") as MeshInstance3D).mesh.material as StandardMaterial3D
	var albedo := ground.albedo_color
	ground.albedo_color = Color(0.3, 0.3, 0.3)
	var warmed := await warmed_pixels(light, "%d-light" % TORCH)
	ground.albedo_color = albedo
	print("FIXTURE TORCH_LIGHT_PIXELS ", warmed)
	if warmed < MIN_LIT_PIXELS:
		fail("Torch light warmed only %d ground pixels" % warmed)
		return false
	# M2 surfaces take point lights too: a white light over the torch handle.
	var probe := OmniLight3D.new()
	# 0.23 yd over the handle, out of reach of the ground 0.75 yd below.
	probe.position = Vector3(0.3, 0.75, 0.0)
	probe.omni_range = 0.45
	scene.add_child(probe)
	var handle := await warmed_pixels(probe, "%d-handle-light" % TORCH)
	probe.free()
	halo.visible = true
	scene.get_node("Overlay").visible = true
	print("FIXTURE TORCH_HANDLE_LIT_PIXELS ", handle)
	if handle < MIN_HANDLE_PIXELS:
		fail("A point light lit only %d torch handle pixels" % handle)
		return false
	push_key(KEY_0)
	return await wait_drawn(TORCH)

# Pixels `light` brightens, red at least as much as blue, between shots with it on and
# off (energy 0: the M2 light node rewrites its visibility every frame).
func warmed_pixels(light: Light3D, name: String) -> int:
	var lit := await save_shot(name + "-on.png")
	light.light_energy = 0.0
	var unlit := await save_shot(name + "-off.png")
	light.light_energy = 1.0
	var warmed := 0
	for y in lit.get_height():
		for x in lit.get_width():
			var p := lit.get_pixel(x, y)
			var q := unlit.get_pixel(x, y)
			if p.r - q.r > 0.02 and p.r - q.r >= p.b - q.b:
				warmed += 1
	return warmed

# The halo quad (batch 1, bone 1) and the flame's parent bone 2 face the camera from
# any orbit yaw; without billboarding the quad is edge-on to the starting view.
func check_halo_billboard() -> bool:
	var camera := scene.get_node("Camera") as Camera3D
	var skeleton := torch_node().get_node("Skeleton3D") as Skeleton3D
	var halo := torch_node().get_node("Batch1") as MeshInstance3D
	push_key(KEY_1)
	for turn in 2:
		await settle(300)
		for bone in [1, 2]:
			var facing := (skeleton.global_transform.basis * skeleton.get_bone_global_pose(bone).basis).x.normalized()
			if facing.dot(camera.global_basis.z) < 0.999:
				fail("Torch bone %d faces %s, not the camera's %s" % [bone, facing, camera.global_basis.z])
				return false
		scene.get_node("Overlay").visible = false
		var shown := await save_shot("%d-halo-%d.png" % [TORCH, turn])
		halo.visible = false
		var hidden := await save_shot("%d-halo-%d-hidden.png" % [TORCH, turn])
		halo.visible = true
		scene.get_node("Overlay").visible = true
		var halo_pixels := changed_pixels(shown, hidden)
		print("FIXTURE TORCH_HALO_PIXELS ", turn, " ", halo_pixels)
		if halo_pixels < MIN_HALO_PIXELS:
			fail("Torch halo changed only %d pixels at turn %d" % [halo_pixels, turn])
			return false
		drag_orbit(Vector2(200, 40) * (1 - 2 * turn))
	await process_frame
	push_key(KEY_0)
	return await wait_drawn(TORCH)

func drag_orbit(total: Vector2) -> void:
	mouse_button(MOUSE_BUTTON_LEFT, true)
	var motion := InputEventMouseMotion.new()
	motion.position = Vector2(640, 360)
	motion.relative = total
	motion.button_mask = MOUSE_BUTTON_MASK_LEFT
	root.push_input(motion, true)
	mouse_button(MOUSE_BUTTON_LEFT, false)

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
