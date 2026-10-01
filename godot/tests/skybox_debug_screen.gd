extends SceneTree

# Production observer, not a replacement scene or renderer. MAIN supplies original
# client arguments and an existing GODOT_SKYBOX_SCREENSHOTS directory.
# Example: godot --path godot -s res://tests/skybox_debug_screen.gd --
#   --screen skyboxdebug --skybox-fdid 525142 --skybox-verify --skybox-time-ms 0
# Root contract: GameClient/SkyboxDebug. Camera and authored M2 are discovered by
# class and assets::M2_SOURCE_META, whose value must be the actual loader input.
const SOURCE_META := "m2_source_path"
const WAIT_MS := 30000
const FOCUS := Vector3(0, 1, 0)
const START_DISTANCE := 7.5
const BASE_PITCH := 0.15
# src/camera_control_data.rs and src/client_options_data.rs, not native readback.
const DEFAULT_FOV := 90.0
const DEFAULT_SENSITIVITY := 0.003
# src/rendering/camera/orbit_camera.rs, independent of native controller state.
const ZOOM_STEP := 0.4
const ZOOM_LERP := 0.25
const MIN_DISTANCE := 0.5
const MAX_DISTANCE := 20.0
const POSE_EPSILON := 0.0002
const PIXEL_DELTA := 2.0 / 255.0

var client: Node
var scene: Node3D
var camera: Camera3D
var sky: Node3D
var shots := ""
var expected_file := ""
var verify_only := false
var fixed_time := false
var sensitivity := DEFAULT_SENSITIVITY
var expected_fov := DEFAULT_FOV
var failed := false
var ui_rects: Array[Rect2] = []

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	if not choose_case():
		return
	if not await mount_production():
		return
	if not observe_assets():
		return
	if not read_camera_options():
		return
	if not check_offline():
		return
	if not check_composition():
		return
	if not check_pose(0.0, START_DISTANCE):
		return
	if not await check_orbit_input():
		return
	if not await check_sky_pixels():
		return
	if not check_offline():
		return
	print("LIMIT: no original-expected pixel oracle; contribution is not full pixel parity")
	print("LIMIT: cached sequence durations alone do not prove varying material/bone tracks; no live-animation assertion")
	print("PASS: bounded production skybox startup/source/camera/input/composition/contribution observer")
	quit(0)

func choose_case() -> bool:
	shots = OS.get_environment("GODOT_SKYBOX_SCREENSHOTS")
	if shots.is_empty() or not DirAccess.dir_exists_absolute(shots):
		return reject("Requires existing GODOT_SKYBOX_SCREENSHOTS directory")
	var args := OS.get_cmdline_user_args()
	print("FIXTURE SKYBOX_ARGV ", args)
	if first_value(args, "--screen") != "skyboxdebug":
		return reject("Requires original --screen skyboxdebug")
	verify_only = args.has("--skybox-verify")
	fixed_time = args.has("--skybox-time-ms")
	var fdid := first_value(args, "--skybox-fdid")
	var light_id := first_value(args, "--light-skybox-id")
	if not fdid.is_empty() and not light_id.is_empty():
		return reject("--skybox-fdid and --light-skybox-id are mutually exclusive")
	if light_id == "653" or fdid == "5412968":
		expected_file = "11xp_cloudsky01.m2"
	elif fdid == "525142":
		expected_file = "costalislandskybox.m2"
	else:
		return reject("Bounded cached cases: --skybox-fdid 525142/5412968 or --light-skybox-id 653")
	return true

func first_value(args: PackedStringArray, flag: String) -> String:
	var index := args.find(flag)
	if index < 0:
		return ""
	if index + 1 >= args.size():
		return ""
	return args[index + 1]

func mount_production() -> bool:
	if not ClassDB.class_exists("GameClient"):
		return reject("GameClient native class unavailable; extension/setup failure, not missing-scene RED")
	var packed := load("res://scenes/client.tscn") as PackedScene
	if packed == null:
		return reject("Production client scene failed to load")
	client = packed.instantiate()
	if client == null:
		return reject("Production client scene failed to instantiate")
	if not client.is_class("GameClient"):
		client.free()
		return reject("Production client root is not GameClient")
	root.add_child(client)
	var deadline := Time.get_ticks_msec() + WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if not is_instance_valid(client):
			return reject("Production client freed during asset/startup initialization")
		if client.is_queued_for_deletion():
			return reject("Production client queued for deletion during asset/startup initialization")
		scene = client.get_node_or_null("SkyboxDebug") as Node3D
		if scene != null:
			if scene.is_node_ready():
				await RenderingServer.frame_post_draw
				return true
	# Current native production exits 1 itself after API/Vulkan/CASC startup:
	# Cannot initialize client: --screen skyboxdebug is not yet implemented in Godot.
	return reject("Missing production SkyboxDebug after bounded asset/startup wait; inspect Cannot initialize client error")

func observe_assets() -> bool:
	var cameras := scene.find_children("*", "Camera3D", true, false)
	if cameras.size() != 1:
		return reject("SkyboxDebug requires one real Camera3D, found %d" % cameras.size())
	camera = cameras[0] as Camera3D
	if root.get_camera_3d() != camera:
		return reject("SkyboxDebug Camera3D is not rendering the root viewport")
	var roots: Array[Node3D] = []
	for node in scene.find_children("*", "Node3D", true, false):
		if node.has_meta(SOURCE_META):
			roots.append(node as Node3D)
	if roots.size() != 1:
		return reject("Expected one actual authored M2 loader root with %s, found %d" % [SOURCE_META, roots.size()])
	sky = roots[0]
	var source := str(sky.get_meta(SOURCE_META))
	if not source.replace("\\", "/").ends_with("/models/skyboxes/" + expected_file):
		return reject("Wrong authored loader source: expected %s, got %s" % [expected_file, source])
	if not FileAccess.file_exists(source):
		return reject("Actual authored loader source file unavailable: " + source)
	var meshes := sky.find_children("*", "MeshInstance3D", true, false)
	var surfaces := 0
	for node in meshes:
		var mesh_node := node as MeshInstance3D
		if mesh_node.mesh == null:
			return reject("Authored M2 batch has no mesh: " + str(node.get_path()))
		for index in mesh_node.mesh.get_surface_count():
			var arrays := mesh_node.mesh.surface_get_arrays(index)
			if arrays.size() <= Mesh.ARRAY_VERTEX:
				return reject("Authored M2 surface has no vertex array")
			var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
			if vertices.is_empty():
				return reject("Authored M2 surface has no vertices")
			var material := mesh_node.get_active_material(index) as ShaderMaterial
			if material == null:
				return reject("Authored sky batch lacks specialized ShaderMaterial")
			if material.get_shader_parameter("base_texture") == null:
				return reject("Authored sky batch lacks actual base texture")
			surfaces += 1
	if surfaces == 0:
		return reject("Source metadata without actual M2 geometry")
	print("FIXTURE SKYBOX_SOURCE ", source, " surfaces=", surfaces, " camera=", camera.get_path(), " sky=", sky.get_path())
	return true

func read_camera_options() -> bool:
	var config := OS.get_environment("XDG_CONFIG_HOME")
	if config.is_empty():
		config = OS.get_environment("HOME").path_join(".config")
	var path := config.path_join("world-of-osso/options_settings.ron")
	if not FileAccess.file_exists(path):
		var data_root := str(sky.get_meta(SOURCE_META)).get_base_dir().get_base_dir().get_base_dir()
		path = data_root.path_join("ui/options_settings.ron")
	if FileAccess.file_exists(path):
		var raw := FileAccess.get_file_as_string(path)
		if FileAccess.get_open_error() != OK:
			return reject("Cannot read persisted CameraOptions: " + path)
		var block := RegEx.new()
		block.compile("(?s)camera\\s*:\\s*\\((.*?)\\)")
		var matched := block.search(raw)
		if matched != null:
			expected_fov = saved_number(matched.get_string(1), "fovDegrees", DEFAULT_FOV)
			sensitivity = saved_number(matched.get_string(1), "mouseSensitivity", DEFAULT_SENSITIVITY)
		expected_fov = clampf(expected_fov, 90.0, 120.0)
		sensitivity = clampf(sensitivity, 0.001, 0.01)
	print("FIXTURE CAMERA_OPTIONS ", path, " fov=", expected_fov, " sensitivity=", sensitivity)
	return not failed

func saved_number(raw: String, field: String, default_value: float) -> float:
	var regex := RegEx.new()
	regex.compile("\\b" + field + "\\s*:\\s*([^,\\s)]+)")
	var matched := regex.search(raw)
	if matched == null:
		return default_value
	var value := matched.get_string(1)
	if not value.is_valid_float():
		reject("Unsupported persisted camera numeric value: " + field + "=" + value)
		return default_value
	return value.to_float()

func check_offline() -> bool:
	if not is_instance_valid(client):
		return reject("Client freed before offline checks")
	var login := client.get_node_or_null("LoginUI") as CanvasLayer
	if login != null:
		if login.visible:
			return reject("LoginUI visible over offline SkyboxDebug")
	if not client.has_method("account_state"):
		return reject("Production GameClient lacks existing account_state observer")
	var state: Dictionary = client.call("account_state")
	if not state.has("reply_received"):
		return reject("Production account_state lacks reply_received")
	var reply_received: bool = state["reply_received"]
	if reply_received:
		return reject("Offline SkyboxDebug received authentication reply")
	return true

func check_composition() -> bool:
	var planes := 0
	var other_geometry := 0
	for node in scene.find_children("*", "MeshInstance3D", true, false):
		if sky.is_ancestor_of(node):
			continue
		var mesh_node := node as MeshInstance3D
		if not mesh_node.is_visible_in_tree():
			continue
		var bounds := mesh_node.get_aabb()
		var planar: bool = bounds.size.y < 0.001 and bounds.size.x > 1.0
		if planar and bounds.size.z > 1.0:
			planes += 1
		else:
			other_geometry += 1
	var environment := camera.environment
	if environment == null:
		var environments := scene.find_children("*", "WorldEnvironment", true, false)
		if environments.size() == 1:
			environment = (environments[0] as WorldEnvironment).environment
	if environment == null:
		return reject("Cannot observe actual scene Environment/composition")
	var fog: bool = environment.fog_enabled or environment.volumetric_fog_enabled
	if verify_only:
		if planes != 0 or other_geometry != 0:
			return reject("Authored-only mode contains visible reference/procedural geometry")
		if fog:
			return reject("Authored-only mode retains procedural fog")
	else:
		# Forced FDIDs have absent Light flags, not the flags of a guessed DB row.
		# LightSkyboxID 653 has audited flags 0b01111. Both retain baseline + fog.
		if planes != 1 or other_geometry == 0:
			return reject("Default forced-source mode lacks reference plane/procedural baseline")
		if not fog:
			return reject("Default forced-source mode lacks original procedural fog")
	print("FIXTURE COMPOSITION verify=", verify_only, " planes=", planes, " other_geometry=", other_geometry, " fog=", fog)
	return true

func check_pose(yaw: float, distance: float) -> bool:
	if not is_instance_valid(camera) or not is_instance_valid(sky):
		return reject("Camera/authored sky freed during observation")
	var offset := Vector3(sin(yaw) * cos(BASE_PITCH), sin(BASE_PITCH), cos(yaw) * cos(BASE_PITCH))
	var expected_eye := FOCUS + offset * distance
	if camera.global_position.distance_to(expected_eye) > POSE_EPSILON:
		return reject("Camera eye mismatch: expected %s got %s" % [expected_eye, camera.global_position])
	var expected_basis := Basis.looking_at(FOCUS - expected_eye, Vector3.UP)
	for axis in 3:
		if camera.global_basis[axis].distance_to(expected_basis[axis]) > POSE_EPSILON:
			return reject("Camera orbit basis mismatch at axis %d" % axis)
	if absf(camera.fov - expected_fov) > 0.001:
		return reject("Camera FOV mismatch: persisted/default %.4f actual %.4f" % [expected_fov, camera.fov])
	if sky.global_position.distance_to(FOCUS) > POSE_EPSILON:
		return reject("Authored sky is not centered on orbit focus")
	return true

func check_orbit_input() -> bool:
	await RenderingServer.frame_post_draw
	await drag(Vector2(4, 0))
	var yaw := -4.0 * sensitivity
	var orbit_ok: bool = check_pose(yaw, START_DISTANCE)
	# Always release/reverse injected input, including a failed pose assertion.
	await drag(Vector2(-4, 0))
	if not orbit_ok:
		return false
	if not check_pose(0.0, START_DISTANCE):
		return false
	var distance := START_DISTANCE
	var target := START_DISTANCE
	# One notch proves .4 and first-frame .25; large runs prove both clamps.
	for count in [1, 80, -100]:
		var button := MOUSE_BUTTON_WHEEL_UP if count > 0 else MOUSE_BUTTON_WHEEL_DOWN
		for notch in absi(count):
			mouse_button(button, true)
			mouse_button(button, false)
			target = clampf(target - signi(count) * ZOOM_STEP, MIN_DISTANCE, MAX_DISTANCE)
		distance = lerpf(distance, target, ZOOM_LERP)
		await RenderingServer.frame_post_draw
		if not check_pose(0.0, distance):
			return false
		for frame in 48:
			distance = lerpf(distance, target, ZOOM_LERP)
			await RenderingServer.frame_post_draw
			if not check_pose(0.0, distance):
				return false
	# Restore through wheel events, not a controller callback or transform setter.
	# The 20 endpoint needs 31 up notches then one fractional notch: 20-12.4-.1.
	for notch in 31:
		mouse_button(MOUSE_BUTTON_WHEEL_UP, true)
		mouse_button(MOUSE_BUTTON_WHEEL_UP, false)
	mouse_button(MOUSE_BUTTON_WHEEL_UP, true, 0.25)
	mouse_button(MOUSE_BUTTON_WHEEL_UP, false, 0.25)
	for frame in 64:
		await RenderingServer.frame_post_draw
	return check_pose(0.0, START_DISTANCE)

func drag(relative: Vector2) -> void:
	mouse_button(MOUSE_BUTTON_LEFT, true)
	var motion := InputEventMouseMotion.new()
	motion.position = Vector2(640, 360)
	motion.relative = relative
	motion.button_mask = MOUSE_BUTTON_MASK_LEFT
	root.push_input(motion, true)
	await RenderingServer.frame_post_draw
	mouse_button(MOUSE_BUTTON_LEFT, false)
	await RenderingServer.frame_post_draw

func mouse_button(button: MouseButton, pressed: bool, factor: float = 1.0) -> void:
	var event := InputEventMouseButton.new()
	event.position = Vector2(640, 360)
	event.button_index = button
	event.pressed = pressed
	event.factor = factor
	root.push_input(event, true)

func check_sky_pixels() -> bool:
	# Visible Control rectangles are excluded from the sky mask; UI is not hidden.
	for node in client.find_children("*", "Control", true, false):
		var control := node as Control
		if visible_ui_surface(control):
			ui_rects.append(control.get_global_rect())
	for frame in 32:
		await RenderingServer.frame_post_draw
	var shown := await capture("sky-shown.png")
	if shown == null:
		return false
	var was_visible := sky.visible
	if not sky.is_visible_in_tree():
		return reject("Actual authored sky root is invisible")
	sky.visible = false
	var hidden := await capture("sky-hidden.png")
	sky.visible = was_visible
	var restored := await capture("sky-restored.png")
	if hidden == null or restored == null:
		return false
	var mask: Array[Vector2i] = []
	for y in range(0, shown.get_height(), 2):
		for x in range(0, shown.get_width(), 2):
			var point := Vector2i(x, y)
			if is_ui_pixel(point):
				continue
			if pixel_delta(shown, hidden, point) > PIXEL_DELTA:
				mask.append(point)
	if mask.size() < 100:
		return reject("Hiding only actual authored M2 changed too few pixels: %d" % mask.size())
	if not coherent(shown, restored, mask):
		return reject("Shown/restored sky captures incoherent; cannot attribute hidden baseline to M2")
	if verify_only:
		if not black_baseline(hidden):
			return false
	if fixed_time:
		for sample in 3:
			for frame in 20:
				await RenderingServer.frame_post_draw
			var later := await capture("sky-fixed-%d.png" % sample)
			if later == null:
				return false
			if not coherent(shown, later, mask):
				return reject("Fixed --skybox-time-ms did not stabilize actual authored sky pixels")
	else:
		print("LIMIT: no fixed time; A/B/A contribution coherence only, no track-specific animation oracle")
	print("FIXTURE SKYBOX_CONTRIBUTION sampled_pixels=", mask.size(), " fixed_time=", fixed_time)
	return not failed

func capture(name: String) -> Image:
	await RenderingServer.frame_post_draw
	if not is_instance_valid(sky):
		reject("Authored sky freed while capturing")
		return null
	var image := root.get_texture().get_image()
	if image == null:
		reject("Viewport capture unavailable")
		return null
	if image.get_size() != root.size:
		reject("Viewport capture extent mismatch")
		return null
	var result := image.save_png(shots.path_join(name))
	if result != OK:
		reject("Cannot save " + name + ": " + error_string(result))
		return null
	return image

func visible_ui_surface(control: Control) -> bool:
	if not control.is_visible_in_tree():
		return false
	var ancestor := control.get_parent()
	while ancestor != null:
		if ancestor is CanvasLayer:
			if not (ancestor as CanvasLayer).visible:
				return false
		ancestor = ancestor.get_parent()
	# Layout-only full-rect Controls must not erase the whole viewport mask.
	if control is Label or control is TextureRect:
		return true
	if control is NinePatchRect or control is BaseButton:
		return true
	if control is Panel or control is PanelContainer:
		return true
	return false

func is_ui_pixel(point: Vector2i) -> bool:
	for rect in ui_rects:
		if rect.grow(2.0).has_point(Vector2(point)):
			return true
	return false

func pixel_delta(a: Image, b: Image, point: Vector2i) -> float:
	var p := a.get_pixelv(point)
	var q := b.get_pixelv(point)
	return maxf(absf(p.r - q.r), maxf(absf(p.g - q.g), absf(p.b - q.b)))

func coherent(a: Image, b: Image, mask: Array[Vector2i]) -> bool:
	for point in mask:
		if pixel_delta(a, b, point) > PIXEL_DELTA:
			return false
	return true

func black_baseline(image: Image) -> bool:
	for y in range(0, image.get_height(), 2):
		for x in range(0, image.get_width(), 2):
			var point := Vector2i(x, y)
			if is_ui_pixel(point):
				continue
			var color := image.get_pixelv(point)
			if maxf(color.r, maxf(color.g, color.b)) > PIXEL_DELTA:
				return reject("Authored-only hidden-sky baseline is not black at %s" % point)
	return true

func reject(message: String) -> bool:
	failed = true
	push_error("SKYBOX FIXTURE: " + message)
	quit(1)
	return false
