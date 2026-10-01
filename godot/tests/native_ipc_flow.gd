extends SceneTree

# Native Godot keeps processing while the Rust parent invokes the unmodified public CLI.
# File markers synchronize observations only; no IPC callback is called by this script.
const SIZE := Vector2i(1280, 720)
var artifacts: String
var marker: ColorRect

func _initialize() -> void:
	call_deferred("run")

func fail(message: String) -> void:
	push_error("Native IPC fixture: " + message)
	quit(1)

func signal_parent(name: String) -> void:
	var file := FileAccess.open(artifacts.path_join(name), FileAccess.WRITE)
	if file == null:
		fail("Cannot write handshake " + name)
		return
	file.store_string("ready\n")
	file.close()

func wait_parent(name: String) -> bool:
	var deadline := Time.get_ticks_msec() + 30000
	while Time.get_ticks_msec() < deadline:
		if FileAccess.file_exists(artifacts.path_join(name)):
			return true
		await process_frame
	fail("Parent did not send " + name)
	return false

func response(name: String) -> Variant:
	return JSON.parse_string(FileAccess.get_file_as_string(artifacts.path_join(name + ".stdout")))

func tree_response(name: String) -> String:
	var value: Variant = response(name)
	if not value is Dictionary or not value.has("Tree") or not value.Tree is String:
		fail("Existing JSON Tree response missing: " + name)
		return ""
	return value.Tree

func expect_diagnostics() -> bool:
	if response("ping-json") != "Pong":
		fail("Existing JSON Pong response changed")
		return false
	var hierarchy := tree_response("tree")
	for name in ["IpcFixtureClient", "IpcFixtureCamera", "IpcFixtureMesh", "IpcFixtureMarker"]:
		if not hierarchy.contains(name):
			fail("Node hierarchy missing live " + name)
			return false
	var filtered := tree_response("tree-filter")
	if not filtered.contains("IpcFixtureMesh") or filtered.contains("IpcFixtureCamera"):
		fail("dump-tree case-insensitive filter changed")
		return false
	if tree_response("tree-empty") != "":
		fail("Unmatched hierarchy filter must return empty Tree")
		return false
	var ui := tree_response("ui-filter")
	if not ui.contains("ConnectButton") or not ui.contains("[Button]") or not ui.contains("alpha=") or not ui.contains("visible"):
		fail("UI dump must expose actual registry frame type/visibility/alpha")
		return false
	if not tree_response("ui").contains("UsernameInput"):
		fail("Full UI dump missing actual login input")
		return false
	var semantic := tree_response("scene")
	if semantic.is_empty() or semantic == "(no scene tree)" or not semantic.to_lower().contains("camera"):
		fail("Semantic scene dump must describe live scene, including camera")
		return false
	var performance: Variant = response("performance")
	if not performance is Dictionary or not performance.has("Performance"):
		fail("Existing JSON Performance response changed")
		return false
	var stats: Dictionary = performance.Performance
	if not stats.has_all(["fps", "frame_time_ms", "focused"]) or not stats.focused is bool:
		fail("Performance snapshot fields/types changed")
		return false
	if not (stats.fps is float or stats.fps is int) or not (stats.frame_time_ms is float or stats.frame_time_ms is int):
		fail("Settled runtime FPS/frame time must be measured numbers")
		return false
	if not is_finite(float(stats.fps)) or not is_finite(float(stats.frame_time_ms)) or stats.fps <= 0 or stats.frame_time_ms <= 0:
		fail("Performance snapshot is not live finite positive runtime data")
		return false
	return true

func expect_capture(phase: String, color: Color) -> bool:
	var path := artifacts.path_join(phase + ".webp")
	var bytes := FileAccess.get_file_as_bytes(path)
	if bytes.size() < 12 or bytes.slice(0, 4).get_string_from_ascii() != "RIFF" or bytes.slice(8, 12).get_string_from_ascii() != "WEBP":
		fail("Screenshot is not actual WebP data")
		return false
	var image := Image.new()
	if image.load(path) != OK or image.get_size() != SIZE:
		fail("Screenshot must decode at actual viewport resolution")
		return false
	var pixel := image.get_pixel(24, 24)
	if absf(pixel.r - color.r) > 0.08 or absf(pixel.g - color.g) > 0.08 or absf(pixel.b - color.b) > 0.08:
		fail("Screenshot captured wrong live frame at marker: " + str(pixel))
		return false
	# A real 3D unshaded white box is centered separately from the 2D marker.
	var center := image.get_pixel(SIZE.x / 2, SIZE.y / 2)
	if center.r < 0.8 or center.g < 0.8 or center.b < 0.8:
		fail("Actual camera/mesh pixels missing from screenshot: " + str(center))
		return false
	return true

func run() -> void:
	artifacts = OS.get_environment("GODOT_IPC_ARTIFACTS")
	if artifacts.is_empty() or DisplayServer.get_name() == "headless":
		fail("Requires owned artifact path and rendered offscreen display")
		return
	root.size = SIZE
	var client: Node3D = ClassDB.instantiate("GameClient")
	client.name = "IpcFixtureClient"
	root.add_child(client)
	# Keep a real rendered 3D viewport above opaque login artwork, without changing UI state.
	var layer := CanvasLayer.new()
	layer.layer = 100
	client.add_child(layer)
	var container := SubViewportContainer.new()
	container.position = Vector2(512, 232)
	container.size = Vector2(256, 256)
	layer.add_child(container)
	var viewport := SubViewport.new()
	viewport.size = Vector2i(256, 256)
	viewport.own_world_3d = true
	viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	container.add_child(viewport)
	var camera := Camera3D.new()
	camera.name = "IpcFixtureCamera"
	camera.position = Vector3(0, 0, 3)
	camera.current = true
	viewport.add_child(camera)
	var mesh := MeshInstance3D.new()
	mesh.name = "IpcFixtureMesh"
	mesh.mesh = BoxMesh.new()
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.albedo_color = Color.WHITE
	mesh.material_override = material
	viewport.add_child(mesh)
	marker = ColorRect.new()
	marker.name = "IpcFixtureMarker"
	marker.position = Vector2(8, 8)
	marker.size = Vector2(48, 48)
	marker.color = Color.RED
	layer.add_child(marker)
	var deadline := Time.get_ticks_msec() + 20000
	while client.find_child("ConnectButton", true, false) == null and Time.get_ticks_msec() < deadline:
		await process_frame
	if client.find_child("ConnectButton", true, false) == null:
		fail("SETUP: actual native Login UI never became ready")
		return
	for frame in range(120):
		await process_frame
	await RenderingServer.frame_post_draw
	signal_parent("ready")
	if not await wait_parent("verify-red"):
		return
	if not expect_diagnostics() or not expect_capture("red", Color.RED):
		return
	marker.color = Color.GREEN
	await process_frame
	await RenderingServer.frame_post_draw
	signal_parent("verified-red")
	if not await wait_parent("verify-green"):
		return
	if not expect_capture("green", Color.GREEN):
		return
	signal_parent("verified-green")
	if not await wait_parent("finish"):
		return
	client.queue_free()
	await process_frame
	print("PASS: actual native diagnostics and two independently changed rendered frames")
	quit(0)
