extends SceneTree

# Offline image oracle. MAIN produces both real renderer captures and manifests.
# No client, shader port, hide-attribution mask, image alignment or fitted threshold.
# Environment: GODOT_SKYBOX_REFERENCE_MANIFEST, GODOT_SKYBOX_NATIVE_MANIFEST,
# GODOT_SKYBOX_COMPARE_REPORT. Self-test only: GODOT_SKYBOX_COMPARE_SELFTEST=1.
const CHANNEL_TOLERANCE := 2.0 / 255.0
const EXTENT := Vector2i(1280, 720)
var failed := false

func _initialize() -> void:
	if OS.get_environment("GODOT_SKYBOX_COMPARE_SELFTEST") == "1":
		quit(0 if self_test() else 1)
		return
	var reference := read_manifest("GODOT_SKYBOX_REFERENCE_MANIFEST", "bevy")
	var native := read_manifest("GODOT_SKYBOX_NATIVE_MANIFEST", "godot")
	if failed:
		quit(1)
		return
	if reference["inputs"] != native["inputs"]:
		reject("Renderer manifests have unequal exact inputs; no comparable-pixels claim")
		quit(1)
		return
	var expected := Image.load_from_file(str(reference["image"]))
	var actual := Image.load_from_file(str(native["image"]))
	if expected == null or actual == null:
		reject("Cannot decode supplied renderer capture")
		quit(1)
		return
	if expected.get_size() != EXTENT or actual.get_size() != EXTENT:
		reject("Comparison requires exact 1280x720 captures, no resize/alignment")
		quit(1)
		return
	var result := compare_pixels(expected, actual)
	result["reference_revision"] = reference["revision"]
	result["native_revision"] = native["revision"]
	result["inputs"] = reference["inputs"]
	result["limit"] = "Supplied original-renderer images only; manifest provenance must be independently audited. No full-scene or all-track parity."
	var report_path := OS.get_environment("GODOT_SKYBOX_COMPARE_REPORT")
	var report := FileAccess.open(report_path, FileAccess.WRITE)
	if report == null:
		reject("Cannot write required comparison report: " + report_path)
		quit(1)
		return
	report.store_string(JSON.stringify(result, "\t"))
	report.close()
	print("FIXTURE ORIGINAL_PIXEL_COMPARISON ", JSON.stringify(result))
	var passes := int(result["failed_pixels"]) == 0
	quit(0 if passes else 1)

func read_manifest(variable: String, renderer: String) -> Dictionary:
	var path := OS.get_environment(variable)
	var file := FileAccess.open(path, FileAccess.READ)
	if file == null:
		reject("Cannot read required manifest " + variable + ": " + path)
		return {}
	var json := JSON.new()
	var error := json.parse(file.get_as_text())
	file.close()
	if error != OK:
		reject("Invalid manifest JSON: " + path)
		return {}
	if not json.data is Dictionary:
		reject("Manifest must be a JSON object: " + path)
		return {}
	var manifest: Dictionary = json.data
	if not validate_manifest(manifest, renderer):
		reject("Incomplete or mismatched manifest: " + path)
		return {}
	manifest["image"] = path.get_base_dir().path_join(str(manifest["image"]))
	if not FileAccess.file_exists(str(manifest["image"])):
		reject("Manifest image unavailable: " + str(manifest["image"]))
		return {}
	return manifest

func validate_manifest(manifest: Dictionary, renderer: String) -> bool:
	for field in ["renderer", "revision", "image", "inputs"]:
		if not manifest.has(field):
			return false
	if str(manifest["renderer"]) != renderer:
		return false
	var revision := str(manifest["revision"])
	if revision.length() != 40 or not revision.is_valid_hex_number():
		return false
	if str(manifest["image"]).is_empty():
		return false
	if not manifest["inputs"] is Dictionary:
		return false
	var inputs: Dictionary = manifest["inputs"]
	for field in ["client_args", "viewport", "camera_eye", "camera_target", "fov_degrees", "time_ms", "composition", "asset_sha256", "render_options", "light_sample", "pixel_encoding"]:
		if not inputs.has(field):
			return false
	if not inputs["viewport"] is Array:
		return false
	var viewport: Array = inputs["viewport"]
	if viewport.size() != 2:
		return false
	# JSON numbers decode as floats; compare values, not Array variant types.
	if viewport[0] != 1280 or viewport[1] != 720:
		return false
	if str(inputs["pixel_encoding"]) != "srgb-rgb8":
		return false
	if not inputs["asset_sha256"] is Dictionary:
		return false
	var assets: Dictionary = inputs["asset_sha256"]
	if assets.size() < 3:
		return false # M2, SKIN and actual texture artifacts, not just source names.
	for value in assets.values():
		var digest := str(value)
		if digest.length() != 64 or not digest.is_valid_hex_number():
			return false
	return true

func compare_pixels(expected: Image, actual: Image) -> Dictionary:
	if expected.get_size() != actual.get_size():
		return {"failed_pixels": -1, "error": "Unequal image extents"}
	var failed_pixels := 0
	var maximum_error := 0.0
	var total_error := 0.0
	var first_failure := Vector2i(-1, -1)
	for y in expected.get_height():
		for x in expected.get_width():
			var point := Vector2i(x, y)
			var a := expected.get_pixelv(point)
			var b := actual.get_pixelv(point)
			var error := maxf(absf(a.r - b.r), maxf(absf(a.g - b.g), absf(a.b - b.b)))
			maximum_error = maxf(maximum_error, error)
			total_error += error
			if error > CHANNEL_TOLERANCE + 0.0000001:
				failed_pixels += 1
				if first_failure.x < 0:
					first_failure = point
	var pixels := expected.get_width() * expected.get_height()
	return {"pixels": pixels, "failed_pixels": failed_pixels, "maximum_rgb_error": maximum_error, "mean_max_rgb_error": total_error / pixels, "first_failure": [first_failure.x, first_failure.y], "channel_tolerance": CHANNEL_TOLERANCE, "comparison": "all RGB pixels, no mask/no fit"}

func self_test() -> bool:
	var expected := Image.create(4, 2, false, Image.FORMAT_RGBA8)
	expected.fill(Color(32.0 / 255.0, 64.0 / 255.0, 128.0 / 255.0))
	var actual := expected.duplicate() as Image
	var identical := compare_pixels(expected, actual)
	if int(identical["failed_pixels"]) != 0:
		return reject("Identical concrete images failed comparison")
	actual.set_pixel(1, 1, Color(34.0 / 255.0, 64.0 / 255.0, 128.0 / 255.0))
	var boundary := compare_pixels(expected, actual)
	if int(boundary["failed_pixels"]) != 0:
		return reject("Declared two-code boundary failed")
	actual.set_pixel(2, 0, Color(32.0 / 255.0, 67.0 / 255.0, 128.0 / 255.0))
	var mismatch := compare_pixels(expected, actual)
	if int(mismatch["failed_pixels"]) != 1:
		return reject("Concrete three-code discrepancy was not rejected")
	if mismatch["first_failure"] != [2, 0]:
		return reject("Discrepancy location differs")
	var wrong_size := Image.create(2, 2, false, Image.FORMAT_RGBA8)
	var unequal := compare_pixels(expected, wrong_size)
	if int(unequal["failed_pixels"]) != -1:
		return reject("Unequal extents were silently aligned")
	if validate_manifest({}, "bevy"):
		return reject("Missing original provenance was accepted")
	var valid := {
		"renderer": "bevy", "revision": "f23343bb4d43d83ce18f6324064f9f939e67b5bd",
		"image": "original.png", "inputs": {
			"client_args": ["--screen", "skyboxdebug"], "viewport": [1280, 720],
			"camera_eye": [0, 2, 7], "camera_target": [0, 1, 0], "fov_degrees": 90,
			"time_ms": 100000, "composition": "authored-only",
			"asset_sha256": {
				"model": "b15bf9b587b81c55e7c5fcba843adfe603dcd348a1f9041353a708f4ee37e517",
				"skin": "9dd550de7fb3f13c2301e7bdea45921b9d0a6f2a25c3f185d4c0d06150742804",
				"texture": "d337757ddb5848f7e2b7e9feeaa302f2c73db4e619d16aa88636feb996c30b47",
			},
			"render_options": {}, "light_sample": "unshaded", "pixel_encoding": "srgb-rgb8",
		},
	}
	# These are synthetic metadata-format fixtures, not measured renderer provenance.
	if not validate_manifest(valid, "bevy"):
		return reject("Valid raw-hex manifest metadata was rejected")
	var decoded: Dictionary = JSON.parse_string(JSON.stringify(valid))
	if not validate_manifest(decoded, "bevy"):
		return reject("Valid manifest metadata failed actual JSON decode boundary")
	decoded["inputs"]["viewport"][0] = 1280.5
	if validate_manifest(decoded, "bevy"):
		return reject("Fractional viewport extent was silently truncated")
	print("PASS: bounded exact image comparator self-test, no renderer-parity claim")
	return true

func reject(message: String) -> bool:
	failed = true
	push_error("SKYBOX REFERENCE FIXTURE: " + message)
	return false
