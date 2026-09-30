extends "res://tests/taa_options_pixels.gd"
## Actual native Taa extent changes and independent viewport isolation.
## Settled coverage only, not exact temporal history or HDR/Bloom parity.

const RESIZED := Vector2i(1440, 720)
const RESIZE_OFFSET := Vector2i(80, 0)
const ISOLATED_SIZE := Vector2i(256, 144)

var isolated: SubViewport
var extent_failed := false


func run_follow_up(client: Node, path: String, directory: String) -> bool:
	var camera := root.get_node("AntiAliasFixtureCamera") as Camera3D
	var original := snapshot_camera(camera)
	var layer := find_fixture_ui()
	if layer == null:
		return false
	add_isolated_scene(camera)
	var baseline := await capture_isolated(directory, "taa-extent-isolated-baseline.png")
	if baseline == null:
		return false
	for extent in [RESIZED, SIZE]:
		var stage := "resize" if extent == RESIZED else "return-size"
		root.size = extent
		# KEEP_HEIGHT preserves the perspective quad's vertical pixel span.
		# Center translates +80px at 1440x720; move UI by the same integer offset.
		layer.offset = Vector2(RESIZE_OFFSET) if extent == RESIZED else Vector2.ZERO
		if not await wait_for_extent(extent, stage):
			return false
		if not await settle_camera(camera, original, stage):
			return false
		var image := await capture_options_pixels(client, directory, "taa-extent-" + stage + ".png")
		if (
			not expect_extent_pixels(image, extent, stage)
			or not expect_camera(camera, original, stage)
			or not expect_saved_aa(path, true, stage)
			or not expect_renderer_aa(stage)
		):
			return false
		if not await expect_isolated_unchanged(baseline, directory, stage):
			return false
	# Hidden saved AA reloads through real authored Graphics commits, not an
	# assignment to a controller/effect or the stock viewport TAA flag.
	for mode in ["None", "Taa"]:
		if not rewrite_saved_aa(path, mode):
			return false
		aa_mode = mode
		var cap_enabled := mode == "Taa"
		var segment := "RightHit" if cap_enabled else "LeftHit"
		await click_option(client, "ToggleSwitchframe_rate_limit_enabled" + segment)
		var stage := "live-" + mode
		if extent_failed:
			return false
		if Engine.max_fps != (DEFAULT_FPS if cap_enabled else 0):
			fail(stage + ": authored frame cap did not apply")
			return false
		if not await settle_camera(camera, original, stage):
			return false
		var image := await capture_options_pixels(client, directory, "taa-extent-" + stage + ".png")
		if (
			not expect_extent_pixels(image, SIZE, stage)
			or not expect_camera(camera, original, stage)
			or not expect_saved_aa(path, cap_enabled, stage)
			or not expect_renderer_aa(stage)
		):
			return false
		if not await expect_isolated_unchanged(baseline, directory, stage):
			return false
	isolated.queue_free()
	for frame in range(3):
		await process_frame
	if is_instance_valid(isolated):
		fail("Independent viewport survived fixture teardown")
		return false
	isolated = null
	print(
		"PASS: active native Taa resize/return, perspective restoration and exact SubViewport isolation"
	)
	return true


func find_fixture_ui() -> CanvasLayer:
	for node in root.get_children():
		if node is CanvasLayer and node.layer == 100:
			return node as CanvasLayer
	fail("Higher-layer AA fixture UI missing")
	return null


func wait_for_extent(extent: Vector2i, stage: String) -> bool:
	var deadline := Time.get_ticks_msec() + MENU_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if root.size == extent and Vector2i(root.get_visible_rect().size) == extent:
			return true
	fail(stage + ": production viewport did not reach " + str(extent))
	return false


func expect_extent_pixels(image: Image, extent: Vector2i, stage: String) -> bool:
	if image == null or image.get_size() != extent:
		fail(stage + ": missing capture or wrong dimensions, expected " + str(extent))
		return false
	# Validate all pixels, including the extra width outside the inherited crop.
	if not expect_finite_image(image, stage):
		return false
	var offset := RESIZE_OFFSET if extent == RESIZED else Vector2i.ZERO
	# Integer translation only: inherited exact UI and geometric coverage
	# thresholds remain unchanged, with no resizing or image filtering.
	return expect_aa_pixels(image.get_region(Rect2i(offset, SIZE)), stage)


func expect_finite_image(image: Image, stage: String) -> bool:
	for y in range(image.get_height()):
		for x in range(image.get_width()):
			var color := image.get_pixel(x, y)
			for channel in range(4):
				if not is_finite(color[channel]):
					fail("%s: nonfinite pixel (%d,%d) channel %d" % [stage, x, y, channel])
					return false
	return true


func snapshot_camera(camera: Camera3D) -> Dictionary:
	var snapshot := {}
	for property in [
		"projection",
		"fov",
		"near",
		"far",
		"size",
		"frustum_offset",
		"keep_aspect",
		"h_offset",
		"v_offset",
		"global_transform",
	]:
		snapshot[property] = camera.get(property)
	return snapshot


func expect_camera(camera: Camera3D, original: Dictionary, stage: String) -> bool:
	if root.get_camera_3d() != camera or camera.projection != Camera3D.PROJECTION_PERSPECTIVE:
		fail(stage + ": fixture perspective camera is not active/restored")
		return false
	for property in original:
		if camera.get(property) != original[property]:
			fail(
				(
					"%s: camera %s changed outside drawing: %s expected %s"
					% [
						stage,
						property,
						camera.get(property),
						original[property],
					]
				)
			)
			return false
	return true


func settle_camera(camera: Camera3D, original: Dictionary, stage: String) -> bool:
	for frame in range(32):
		await process_frame
		if not expect_camera(camera, original, stage):
			return false
	await RenderingServer.frame_post_draw
	return expect_camera(camera, original, stage)


func add_isolated_scene(owner_camera: Camera3D) -> void:
	isolated = SubViewport.new()
	isolated.name = "TaaExtentIndependentViewport"
	isolated.size = ISOLATED_SIZE
	isolated.own_world_3d = true
	isolated.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	root.add_child(isolated)
	var camera := Camera3D.new()
	camera.keep_aspect = Camera3D.KEEP_HEIGHT
	camera.set_perspective(owner_camera.fov, owner_camera.near, owner_camera.far)
	camera.transform = owner_camera.transform
	camera.environment = owner_camera.get_world_3d().environment
	isolated.add_child(camera)
	camera.make_current()
	var quad := QuadMesh.new()
	quad.size = Vector2(0.9, 0.6)
	var mesh := MeshInstance3D.new()
	mesh.mesh = quad
	mesh.rotation.z = ANGLE
	var material := ShaderMaterial.new()
	material.shader = load("res://tests/anti_alias_options_pattern.gdshader")
	mesh.material_override = material
	isolated.add_child(mesh)


func capture_isolated(directory: String, filename: String) -> Image:
	for frame in range(3):
		await process_frame
	await RenderingServer.frame_post_draw
	var image := isolated.get_texture().get_image()
	if image == null or image.get_size() != ISOLATED_SIZE:
		fail("Independent viewport capture missing or wrong dimensions: " + filename)
		return null
	if not expect_finite_image(image, filename):
		return null
	# Nonuniform fixture: both foreground and background must actually render.
	if (
		image.get_pixel(128, 72) != Color.WHITE
		or image.get_pixel(0, 0) != Color.BLACK
		or image.get_pixel(255, 143) != Color.BLACK
	):
		fail("Independent viewport lost white quad or black background: " + filename)
		return null
	var error := DirAccess.make_dir_recursive_absolute(directory)
	if error == OK:
		error = image.save_png(directory.path_join(filename))
	if error != OK:
		fail("Save independent viewport capture: " + error_string(error))
		return null
	return image


func expect_isolated_unchanged(baseline: Image, directory: String, stage: String) -> bool:
	var image := await capture_isolated(directory, "taa-extent-isolated-" + stage + ".png")
	if image == null:
		return false
	if image.get_format() != baseline.get_format() or image.get_data() != baseline.get_data():
		fail(stage + ": global TAA operation changed independent viewport pixels")
		return false
	print("TAA_EXTENT isolated pixel-exact: ", stage)
	return true


func rewrite_saved_aa(path: String, mode: String) -> bool:
	var config := OS.get_environment("XDG_CONFIG_HOME")
	if (
		not config.contains("/data/diagnostics/")
		or path != config.path_join("world-of-osso/options_settings.ron")
	):
		fail("Extent AA rewrite requires the known owned options path")
		return false
	var source := FileAccess.get_file_as_string(path)
	if FileAccess.get_open_error() != OK:
		fail("Read owned AA options: " + error_string(FileAccess.get_open_error()))
		return false
	var pattern := RegEx.new()
	var error := pattern.compile("(?m)^[\\t ]*antiAlias:[\\t ]*(Taa|None)[\\t ]*,[\\t ]*$")
	if error != OK:
		fail("Compile AA line pattern: " + error_string(error))
		return false
	var matches := pattern.search_all(source)
	if matches.size() != 1:
		fail("Owned options must contain exactly one antiAlias line, found %d" % matches.size())
		return false
	var found := matches[0]
	if found.get_string(1) != aa_mode or mode not in ["None", "Taa"]:
		fail("Owned AA transition input mismatch: " + found.get_string(1) + " -> " + mode)
		return false
	var rewritten := source.substr(0, found.get_start(1)) + mode + source.substr(found.get_end(1))
	var file := FileAccess.open(path, FileAccess.WRITE)
	if file == null:
		fail("Open owned AA options for write: " + error_string(FileAccess.get_open_error()))
		return false
	file.store_string(rewritten)
	file.flush()
	error = file.get_error()
	file.close()
	if error != OK:
		fail("Write owned AA options: " + error_string(error))
		return false
	if FileAccess.get_file_as_string(path) != rewritten:
		fail("Owned AA rewrite did not persist exact intended options")
		return false
	return true


func fail(message: String) -> void:
	extent_failed = true
	# Inherited failure reports context and requests a nonzero SceneTree exit.
	super.fail(message)
