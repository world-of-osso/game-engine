extends SceneTree

# Native Godot keeps processing while the Rust parent invokes the unmodified public CLI.
# File markers synchronize observations only; no IPC callback is called by this script.
const SIZE := Vector2i(1280, 720)
const TORCH_PATH := "res://../data/models/145304.m2"
const EXPORT_EPSILON := 0.0001
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

func expect_export_condition(condition: bool, message: String) -> bool:
	if not condition:
		fail("ExportScene: " + message)
	return condition

func expect_snapshot_node(value: Variant, is_root: bool) -> bool:
	if not expect_export_condition(value is Dictionary, "node must be a JSON object"):
		return false
	var node: Dictionary = value
	var has_fields: bool = node.has_all(["label", "props", "transform", "children"])
	if not expect_export_condition(has_fields, "SceneSnapshotNode fields missing"):
		return false
	if not expect_export_condition(node.label is String, "label must be a string"):
		return false
	if not expect_export_condition(not node.label.is_empty(), "label must name a live node"):
		return false
	if not expect_export_condition(node.children is Array, "children must be an array"):
		return false
	if is_root:
		if not expect_export_condition(node.props == "Scene", "root must have Scene props"):
			return false
	else:
		if not expect_export_condition(node.props is Dictionary, "semantic props must be externally tagged"):
			return false
		var props: Dictionary = node.props
		if not expect_export_condition(props.size() == 1, "semantic props need exactly one tag"):
			return false
		var tag: String = props.keys()[0]
		if not expect_export_condition(tag in ["Camera", "Light", "Object"], "nonsemantic node exported: " + node.label):
			return false
		if not expect_export_condition(props[tag] is Dictionary, "semantic tag payload must be an object"):
			return false
		if not expect_export_condition(node.transform is Dictionary, "semantic node transform missing"):
			return false
	for child: Variant in node.children:
		if not expect_snapshot_node(child, false):
			return false
	return true

func find_exported_nodes(node: Dictionary, label: String) -> Array[Dictionary]:
	var found: Array[Dictionary] = []
	if node.label == label:
		found.append(node)
	for child: Dictionary in node.children:
		found.append_array(find_exported_nodes(child, label))
	return found

func expect_numeric_array(value: Variant, length: int, field: String) -> bool:
	if not expect_export_condition(value is Array, field + " must be an array"):
		return false
	if not expect_export_condition(value.size() == length, field + " has wrong length"):
		return false
	for component: Variant in value:
		var numeric: bool = component is float or component is int
		if not expect_export_condition(numeric, field + " has nonnumeric component"):
			return false
		if not expect_export_condition(is_finite(float(component)), field + " has nonfinite component"):
			return false
	return true

func expect_export_transform(node: Dictionary, translation: Vector3, rotation: Quaternion, scale: Vector3) -> bool:
	var transform: Dictionary = node.transform
	var has_fields: bool = transform.has_all(["translation", "rotation", "scale"])
	if not expect_export_condition(has_fields, node.label + " TRS fields missing"):
		return false
	for field: String in ["translation", "rotation", "scale"]:
		var length: int = 4 if field == "rotation" else 3
		if not expect_numeric_array(transform[field], length, node.label + "." + field):
			return false
	var position := Vector3(transform.translation[0], transform.translation[1], transform.translation[2])
	var dimensions := Vector3(transform.scale[0], transform.scale[1], transform.scale[2])
	var orientation := Quaternion(transform.rotation[0], transform.rotation[1], transform.rotation[2], transform.rotation[3])
	if not expect_export_condition(position.distance_to(translation) < EXPORT_EPSILON, node.label + " translation must compensate skipped groups: " + str(position)):
		return false
	if not expect_export_condition(dimensions.distance_to(scale) < EXPORT_EPSILON, node.label + " scale mismatch"):
		return false
	if not expect_export_condition(absf(orientation.length() - 1.0) < EXPORT_EPSILON, node.label + " quaternion must be normalized"):
		return false
	# q and -q encode the same rotation; do not normalize bad output into acceptance.
	return expect_export_condition(absf(absf(orientation.dot(rotation)) - 1.0) < EXPORT_EPSILON, node.label + " quaternion mismatch")

func expect_scene_export() -> bool:
	var path := artifacts.path_join("scene.json")
	if not expect_export_condition(FileAccess.file_exists(path), "public CLI did not write scene.json"):
		return false
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	if not expect_export_condition(parsed is Dictionary, "written file must parse as SceneSnapshot"):
		return false
	var snapshot: Dictionary = parsed
	if not expect_export_condition(snapshot.has("root"), "SceneSnapshot root missing"):
		return false
	if not expect_snapshot_node(snapshot.root, true):
		return false
	var scene: Dictionary = snapshot.root
	for omitted: String in ["IpcFixtureGroup", "IpcFixtureMesh", "IpcFixtureMarker", "IpcFixtureClient"]:
		var absent: bool = find_exported_nodes(scene, omitted).is_empty()
		if not expect_export_condition(absent, "nonsemantic fixture node exported: " + omitted):
			return false
	var selected: Dictionary = {}
	for label: String in ["IpcFixtureCamera", "IpcFixtureLight", "IpcFixtureTorch"]:
		var matches: Array[Dictionary] = find_exported_nodes(scene, label)
		if not expect_export_condition(matches.size() == 1, "expected exactly one live " + label):
			return false
		selected[label] = matches[0]
	var camera: Dictionary = selected.IpcFixtureCamera
	var camera_at_root := false
	for child: Dictionary in scene.children:
		if child.label == "IpcFixtureCamera":
			camera_at_root = true
	if not expect_export_condition(camera_at_root, "Camera must attach to Scene root across skipped UI/client containers"):
		return false
	var light: Dictionary = selected.IpcFixtureLight
	var torch: Dictionary = selected.IpcFixtureTorch
	for label: String in ["IpcFixtureLight", "IpcFixtureTorch"]:
		var direct := false
		for child: Dictionary in camera.children:
			if child.label == label:
				direct = true
		if not expect_export_condition(direct, label + " must retain nearest semantic Camera ancestor"):
			return false
	if not expect_export_condition(camera.props.has("Camera"), "live Camera3D must use Camera props"):
		return false
	var camera_props: Dictionary = camera.props.Camera
	if not expect_export_condition(camera_props.has("fov"), "Camera fov missing"):
		return false
	var fov_matches: bool = camera_props.fov == 75.0
	if not expect_export_condition(fov_matches, "Camera fov must equal native default 75 degrees"):
		return false
	if not expect_export_condition(light.props.has("Light"), "live OmniLight3D must use Light props"):
		return false
	var light_props: Dictionary = light.props.Light
	var light_fields: bool = light_props.has_all(["kind", "intensity"])
	if not expect_export_condition(light_fields, "Light kind/intensity missing"):
		return false
	if not expect_export_condition(light_props.kind is String, "Light kind must be a string"):
		return false
	if not expect_export_condition(not light_props.kind.is_empty(), "Light kind must identify the real light"):
		return false
	var energy_matches: bool = light_props.intensity == 2.5
	if not expect_export_condition(energy_matches, "Light intensity must equal native energy 2.5"):
		return false
	if not expect_export_condition(torch.props.has("Object"), "loaded M2 must use Object props"):
		return false
	var model_props: Dictionary = torch.props.Object
	var model_fields: bool = model_props.has_all(["kind", "model"])
	if not expect_export_condition(model_fields, "M2 Object kind/model missing"):
		return false
	if not expect_export_condition(model_props.kind is String, "Object kind must be a string"):
		return false
	if not expect_export_condition(not model_props.kind.is_empty(), "Object kind must identify the real M2"):
		return false
	var source_matches: bool = model_props.model == TORCH_PATH
	if not expect_export_condition(source_matches, "model must identify exact real loader input path"):
		return false
	if not expect_export_transform(camera, Vector3(0, 0, 3), Quaternion.IDENTITY, Vector3.ONE):
		return false
	# Authored group: (4,2,-6), Y +90 degrees. Light: (1,3,2), X +60 degrees.
	# Parent rotates (x,y,z) to (z,y,-x); qY90*qX60 is written analytically below.
	var light_rotation := Quaternion(sqrt(2.0) / 4.0, sqrt(6.0) / 4.0, -sqrt(2.0) / 4.0, sqrt(6.0) / 4.0)
	if not expect_export_transform(light, Vector3(6, 5, -7), light_rotation, Vector3.ONE):
		return false
	# Torch: (-2,1,3), Y -90 degrees, scale 0.5. Relative to Camera, not world.
	return expect_export_transform(torch, Vector3(7, 3, -4), Quaternion.IDENTITY, Vector3.ONE * 0.5)

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
	# A skipped spatial group under a selected Camera exercises nearest-exported-ancestor TRS.
	var group := Node3D.new()
	group.name = "IpcFixtureGroup"
	group.position = Vector3(4, 2, -6)
	group.rotation = Vector3(0, PI / 2.0, 0)
	camera.add_child(group)
	var light := OmniLight3D.new()
	light.name = "IpcFixtureLight"
	light.light_energy = 2.5
	light.position = Vector3(1, 3, 2)
	light.rotation = Vector3(PI / 3.0, 0, 0)
	group.add_child(light)
	if not FileAccess.file_exists(TORCH_PATH):
		fail("SETUP: cached torch M2 missing; no extraction authorized")
		return
	if not FileAccess.file_exists("res://../data/models/14530400.skin"):
		fail("SETUP: cached torch skin missing; no extraction authorized")
		return
	var loader: Object = ClassDB.instantiate("WowAssetLoader")
	var loaded: Dictionary = loader.load_m2(TORCH_PATH)
	if loaded.has("error"):
		fail("SETUP: cached torch load failed; no extraction authorized: " + str(loaded.error))
		return
	var has_node: bool = loaded.has("node")
	if not has_node:
		fail("SETUP: torch loader returned no node")
		return
	if not loaded.node is Node3D:
		fail("SETUP: torch loader returned no Node3D")
		return
	var torch: Node3D = loaded.node
	torch.name = "IpcFixtureTorch"
	torch.position = Vector3(-2, 1, 3)
	torch.rotation = Vector3(0, -PI / 2.0, 0)
	torch.scale = Vector3.ONE * 0.5
	group.add_child(torch)
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
	if not await wait_parent("verify-export"):
		return
	if not expect_scene_export():
		return
	signal_parent("verified-export")
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
