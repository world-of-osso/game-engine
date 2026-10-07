extends "res://tests/ssao_options.gd"
## Pinned Godot 4.7.2, owned Weston ss30, Forward+ Dozen Vulkan, Dummy audio.
## VK_DRIVER_FILES=/opt/game-engine/mesa-dzn/share/vulkan/icd.d/dzn_icd.x86_64.json
## LD_LIBRARY_PATH=/usr/lib/wsl/lib; --display-driver wayland --rendering-driver vulkan
## --script res://tests/ssao_options_pixels.gd -- --screen gamemenu
## Isolate XDG_CONFIG_HOME/XDG_DATA_HOME and GODOT_TEST_CAPTURE_DIR in data/diagnostics.

const UI_REGION := Rect2i(1120, 30, 48, 24)
const CREASE_REGION := Rect2i(180, 360, 420, 250)
const FLAT_REGION := Rect2i(1100, 550, 100, 100)
var geometry: Node3D

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	original_prepass = ProjectSettings.get_setting("rendering/driver/depth_prepass/enable")
	options_path = OS.get_environment("XDG_CONFIG_HOME").path_join("world-of-osso/options_settings.ron")
	var directory := OS.get_environment("GODOT_TEST_CAPTURE_DIR")
	if not options_path.contains("/data/diagnostics/") or not directory.contains("/data/diagnostics/"):
		fail("SSAO pixels require owned config/capture directories")
		return
	if DisplayServer.get_name() == "headless" or RenderingServer.get_rendering_device() == null:
		fail("SSAO pixels require rendered Vulkan")
		return
	add_world()
	original_environment.background_mode = Environment.BG_COLOR
	original_environment.background_color = Color(0.12, 0.12, 0.12)
	original_environment.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	original_environment.ambient_light_color = Color.WHITE
	original_environment.reflected_light_source = Environment.REFLECTION_SOURCE_DISABLED
	original_environment.tonemap_mode = Environment.TONE_MAPPER_LINEAR
	world_camera.fov = 60.0
	add_geometry()
	add_ui()
	var start_enabled := saved_option_value(options_path, "ssaoEnabled") == "true"
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_for_startup_menu(client):
		return
	if not expect_resources(start_enabled, "saved pixel startup"):
		return
	var startup := await capture(client, directory, "startup-on.png" if start_enabled else "startup-off.png")
	if OS.get_environment("SSAO_CAPTURE_BASELINE") == "true":
		if start_enabled or not expect_ui(startup):
			fail("Master baseline requires saved Off")
			return
		print("PASS: captured master-material Off baseline")
		client.free()
		quit(0)
		return
	if original_prepass != start_enabled or not expect_ui(startup):
		fail("Saved SSAO/prepass mismatch at pixel startup")
		return
	await click_menu_action(client, "MenuBtnOptions")
	if not await wait_for_graphics(client):
		return
	for pending in [not start_enabled, start_enabled, not start_enabled]:
		if not await commit_effects(client, pending) or not expect_resources(start_enabled, "pending restart"):
			return
		var unchanged := await capture(client, directory, "pending-%s.png" % str(pending))
		if startup.get_data() != unchanged.get_data():
			fail("Restart-pending toggle changed live pixels")
			return
	var master_path := OS.get_environment("SSAO_MASTER_IMAGE")
	var off := Image.load_from_file(master_path)
	if off == null:
		fail("Missing prepass-disabled master image: " + master_path)
		return
	off.convert(startup.get_format())
	if off.get_size() != startup.get_size():
		fail("Master image dimensions differ")
		return
	if not start_enabled:
		if off.get_data() != startup.get_data():
			fail("Off differs byte-for-byte from master-material scene")
			return
		print("PASS: prepass-disabled Off byte-identical to master; pending On pixels unchanged")
	else:
		var crease_delta := image_difference(off, startup, CREASE_REGION)
		var darker := count_darkened(off, startup, CREASE_REGION)
		var flat_delta := image_difference(off, startup, FLAT_REGION)
		print("SSAO_PIXELS mean_darkening=", crease_delta, " darkened_crease_pixels=", darker, " flat_darkening=", flat_delta)
		if not equal_region(off, startup, FLAT_REGION):
			fail("SSAO darkened isolated flat terrain; whole-scene dimming is not crease proof")
			return
		if crease_delta < 0.002 or darker < 100:
			fail("SSAO produced no contact shading in owned floor/box crease")
			return
		print("PASS: startup On darkens creases only; pending Off pixels and UI unchanged")
	client.free()
	quit(0)

func add_geometry() -> void:
	geometry = Node3D.new()
	root.add_child(geometry)
	# Production terrain material: ambient_light_disabled previously bypassed SSAO.
	var material := ShaderMaterial.new()
	var reference := OS.get_environment("SSAO_REFERENCE_TERRAIN")
	if reference.is_empty():
		material.shader = load("res://shaders/terrain.gdshader")
	else:
		var shader := Shader.new()
		shader.code = FileAccess.get_file_as_string(reference)
		material.shader = shader
	var image := Image.create(1, 1, false, Image.FORMAT_RGBA8)
	image.fill(Color(0.6, 0.6, 0.6, 1.0))
	material.set_shader_parameter("ground_0", ImageTexture.create_from_image(image))
	material.set_shader_parameter("ambient", Vector3.ONE * 0.6)
	material.set_shader_parameter("horizon_ambient", Vector3.ONE * 0.6)
	material.set_shader_parameter("ground_ambient", Vector3.ONE * 0.6)
	var sun := DirectionalLight3D.new()
	geometry.add_child(sun)
	var floor_mesh := PlaneMesh.new()
	floor_mesh.size = Vector2(60, 60)
	var floor_node := MeshInstance3D.new()
	floor_node.mesh = floor_mesh
	floor_node.material_override = material
	floor_node.position = Vector3(0, -2, -20)
	geometry.add_child(floor_node)
	var box := BoxMesh.new()
	box.size = Vector3(4, 2, 4)
	var block := MeshInstance3D.new()
	block.mesh = box
	block.material_override = material
	block.position = Vector3(-3, -1, -12)
	geometry.add_child(block)

func add_ui() -> void:
	var layer := CanvasLayer.new()
	layer.layer = 100
	for side in range(2):
		var rect := ColorRect.new()
		rect.color = Color.WHITE if side == 0 else Color.BLACK
		rect.position = Vector2(UI_REGION.position + Vector2i(side * 24, 0))
		rect.size = Vector2(24, 24)
		rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
		layer.add_child(rect)
	root.add_child(layer)

func capture(client: Node, directory: String, filename: String) -> Image:
	for frame in range(12):
		await process_frame
	return await capture_options_pixels(client, directory, filename)

func expect_ui(image: Image) -> bool:
	if image == null:
		fail("Missing rendered image")
		return false
	for y in range(UI_REGION.position.y, UI_REGION.end.y):
		for x in range(UI_REGION.position.x, UI_REGION.end.x):
			var expected := Color.WHITE if x < UI_REGION.position.x + 24 else Color.BLACK
			if image.get_pixel(x, y) != expected:
				fail("World effect altered UI pixel at %s" % Vector2i(x, y))
				return false
	return true

func equal_region(before: Image, after: Image, region: Rect2i) -> bool:
	for y in range(region.position.y, region.end.y):
		for x in range(region.position.x, region.end.x):
			if before.get_pixel(x, y) != after.get_pixel(x, y):
				return false
	return true

func image_difference(before: Image, after: Image, region: Rect2i) -> float:
	var total := 0.0
	for y in range(region.position.y, region.end.y):
		for x in range(region.position.x, region.end.x):
			total += before.get_pixel(x, y).r - after.get_pixel(x, y).r
	return total / (region.size.x * region.size.y)

func count_darkened(before: Image, after: Image, region: Rect2i) -> int:
	var count := 0
	for y in range(region.position.y, region.end.y):
		for x in range(region.position.x, region.end.x):
			if before.get_pixel(x, y).r - after.get_pixel(x, y).r > 0.025:
				count += 1
	return count

