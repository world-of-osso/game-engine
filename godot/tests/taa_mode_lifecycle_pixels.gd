extends "res://tests/taa_options_pixels.gd"
## Saved hidden-AA reloads through real authored commits, then camera replacement.
## Settled edge coverage only: not an exact first-reset history pixel oracle.


func run_follow_up(client: Node, path: String, directory: String) -> bool:
	var camera := root.get_node("AntiAliasFixtureCamera") as Camera3D
	var original := snapshot_camera(camera)
	for transition in [["None", false], ["Msaa4x", true], ["Taa", false]]:
		var mode: String = transition[0]
		var cap_enabled: bool = transition[1]
		if not rewrite_saved_aa(path, mode):
			return false
		aa_mode = mode
		var segment := "RightHit" if cap_enabled else "LeftHit"
		await click_option(client, "ToggleSwitchframe_rate_limit_enabled" + segment)
		var stage := "live-" + mode
		if not expect_saved_aa(path, cap_enabled, stage) or not expect_renderer_aa(stage):
			return false
		if Engine.max_fps != (DEFAULT_FPS if cap_enabled else 0):
			fail(stage + ": authored frame cap did not apply")
			return false
		if not await expect_settled_camera(camera, original, stage):
			return false
		var image := await capture_options_pixels(client, directory, "taa-" + stage + ".png")
		if not expect_aa_pixels(image, stage) or not expect_camera(camera, original, stage):
			return false
	var replacement := Camera3D.new()
	replacement.name = "AntiAliasReplacementCamera"
	root.add_child(replacement)
	for property in original:
		replacement.set(property, original[property])
	replacement.current = true
	camera.free()
	var stage := "camera-replacement"
	if not await expect_settled_camera(replacement, original, stage):
		return false
	var image := await capture_options_pixels(client, directory, "taa-" + stage + ".png")
	if (
		not expect_saved_aa(path, false, stage)
		or not expect_renderer_aa(stage)
		or not expect_aa_pixels(image, stage)
		or not expect_camera(replacement, original, stage)
	):
		return false
	print("PASS: authored Taa/None/MSAA4x/Taa commits and settled replacement-camera pixels")
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
	if root.get_camera_3d() != camera:
		fail(stage + ": fixture camera is not active")
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


func expect_settled_camera(camera: Camera3D, original: Dictionary, stage: String) -> bool:
	if not expect_camera(camera, original, stage):
		return false
	for frame in range(32):
		await process_frame
		if not expect_camera(camera, original, stage):
			return false
	return true


func rewrite_saved_aa(path: String, mode: String) -> bool:
	var config := OS.get_environment("XDG_CONFIG_HOME")
	if (
		not config.contains("/data/diagnostics/")
		or path != config.path_join("world-of-osso/options_settings.ron")
	):
		fail("Live AA rewrite requires the known owned options path")
		return false
	var source := FileAccess.get_file_as_string(path)
	if FileAccess.get_open_error() != OK:
		fail("Read owned AA options: " + error_string(FileAccess.get_open_error()))
		return false
	var pattern := RegEx.new()
	var error := pattern.compile("(?m)^[\\t ]*antiAlias:[\\t ]*(Taa|None|Msaa4x)[\\t ]*,[\\t ]*$")
	if error != OK:
		fail("Compile AA line pattern: " + error_string(error))
		return false
	var matches := pattern.search_all(source)
	if matches.size() != 1:
		fail("Owned options must contain exactly one antiAlias line, found %d" % matches.size())
		return false
	var found := matches[0]
	if found.get_string(1) != aa_mode or mode not in ["None", "Msaa4x", "Taa"]:
		fail("Owned AA transition input mismatch: " + found.get_string(1) + " -> " + mode)
		return false
	# Replace only the captured enum token; preserve every other byte/field.
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
	if (
		FileAccess.get_file_as_string(path) != rewritten
		or saved_option_value(path, "antiAlias") != mode
	):
		fail("Owned AA rewrite did not persist the exact intended options")
		return false
	return true
