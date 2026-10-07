extends "res://tests/dof_ssao_options.gd"
## Pinned Godot 4.7.2, owned Weston ds30, Forward+ Dozen Vulkan, Dummy audio.
## VK_DRIVER_FILES=/opt/game-engine/mesa-dzn/share/vulkan/icd.d/dzn_icd.x86_64.json
## LD_LIBRARY_PATH=/usr/lib/wsl/lib; --display-driver wayland --rendering-driver vulkan
## --script res://tests/dof_ssao_options_pixels.gd -- --screen gamemenu
## Isolate XDG_CONFIG_HOME/XDG_DATA_HOME and GODOT_TEST_CAPTURE_DIR in data/diagnostics.

const UI_REGION := Rect2i(1120, 30, 48, 24)
const CREASE_REGION := Rect2i(180, 360, 420, 250)
const FAR_REGION := Rect2i(750, 200, 280, 280)
var geometry: Node3D

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	options_path = OS.get_environment("XDG_CONFIG_HOME").path_join("world-of-osso/options_settings.ron")
	var directory := OS.get_environment("GODOT_TEST_CAPTURE_DIR")
	if not options_path.contains("/data/diagnostics/") or not directory.contains("/data/diagnostics/"):
		fail("DOF/SSAO pixels require owned config/capture directories")
		return
	if DisplayServer.get_name() == "headless" or RenderingServer.get_rendering_device() == null:
		fail("DOF/SSAO pixels require rendered Vulkan")
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
	var start_enabled := saved_option_value(options_path, "depthOfField") == "true"
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_for_startup_menu(client):
		return
	if not expect_resources(start_enabled, start_enabled, "saved pixel startup"):
		return
	var startup := await capture(client, directory, "startup-on.png" if start_enabled else "startup-off.png")
	await click_menu_action(client, "MenuBtnOptions")
	if not await wait_for_graphics(client):
		return
	if not await commit_effects(client, false, false):
		return
	var off := await capture(client, directory, "off.png")
	if not expect_resources(false, false, "pixel Off") or not expect_ui(off):
		return
	if not start_enabled and startup.get_data() != off.get_data():
		fail("Saved Off differs from live Off baseline")
		return
	if not await commit_effects(client, false, true):
		return
	var ssao := await capture(client, directory, "ssao-on.png")
	if not expect_resources(false, true, "pixel SSAO") or not expect_ui(ssao):
		return
	var crease_delta := image_difference(off, ssao, CREASE_REGION)
	var darker := count_darkened(off, ssao, CREASE_REGION)
	print("SSAO_PIXELS mean_darkening=", crease_delta, " darkened_crease_pixels=", darker)
	if crease_delta < 0.002 or darker < 100:
		fail("SSAO produced no contact shading in owned floor/box crease")
		return
	if not await commit_effects(client, true, false):
		return
	var dof := await capture(client, directory, "dof-on.png")
	if not expect_resources(true, false, "pixel DOF") or not expect_ui(dof):
		return
	var sharp_edges := edge_energy(off, FAR_REGION)
	var blurred_edges := edge_energy(dof, FAR_REGION)
	print("DOF_PIXELS far_off_edges=", sharp_edges, " far_on_edges=", blurred_edges)
	if sharp_edges < 0.03 or blurred_edges > sharp_edges * 0.7:
		fail("DOF did not blur owned far checker plane")
		return
	if not await commit_effects(client, false, false):
		return
	var restored := await capture(client, directory, "off-restored.png")
	if not expect_resources(false, false, "restored Off") or not expect_ui(restored):
		return
	if off.get_data() != restored.get_data():
		fail("Off did not restore exact baseline pixels")
		return
	if start_enabled:
		if not expect_ui(startup) or edge_energy(startup, FAR_REGION) > sharp_edges * 0.7 or count_darkened(off, startup, CREASE_REGION) < 100:
			fail("Saved On startup lacks world effect pixels")
			return
	print("PASS: SSAO darkens crease, DOF blurs far plane, Off exact baseline, UI unchanged; saved startup=", start_enabled)
	quit(0)

func add_geometry() -> void:
	geometry = Node3D.new()
	root.add_child(geometry)
	var material := StandardMaterial3D.new()
	material.albedo_color = Color(0.8, 0.8, 0.8)
	material.roughness = 1.0
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
	var quad := QuadMesh.new()
	quad.size = Vector2(18, 18)
	var far_plane := MeshInstance3D.new()
	far_plane.mesh = quad
	far_plane.position = Vector3(15, 1, -40)
	var checker := ShaderMaterial.new()
	checker.shader = load("res://tests/dof_ssao_pattern.gdshader")
	far_plane.material_override = checker
	geometry.add_child(far_plane)

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

func edge_energy(image: Image, region: Rect2i) -> float:
	var total := 0.0
	for y in range(region.position.y, region.end.y):
		for x in range(region.position.x, region.end.x - 1):
			total += absf(image.get_pixel(x + 1, y).r - image.get_pixel(x, y).r)
	return total / ((region.size.x - 1) * region.size.y)
