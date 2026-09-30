extends "res://tests/bloom_options_pixels.gd"
## Reuse authored bloom assertions with persisted Taa and a perspective camera.
## Functional combination proof, not exact HDR temporal or full-scene parity.


func add_bloom_scene() -> void:
	super.add_bloom_scene()
	var camera := root.get_node("BloomFixtureCamera") as Camera3D
	# Preserve the original two-world-unit vertical span at the emitter plane.
	camera.set_perspective(rad_to_deg(2.0 * atan(1.0 / 3.0)), 0.1, 100.0)


func expect_saved_bloom(path: String, enabled: bool, intensity: float, stage: String) -> bool:
	if not super.expect_saved_bloom(path, enabled, intensity, stage):
		return false
	if saved_option_value(path, "antiAlias") != "Taa":
		fail(stage + ": authored bloom commit changed saved Taa")
		return false
	return true


func expect_capture(image: Image, stage: String) -> bool:
	if not super.expect_capture(image, stage):
		return false
	var camera := root.get_node("BloomFixtureCamera") as Camera3D
	if root.msaa_3d != Viewport.MSAA_DISABLED or root.use_taa:
		fail(stage + ": persisted Taa must use the custom temporal path, not MSAA/stock TAA")
		return false
	if camera.projection != Camera3D.PROJECTION_PERSPECTIVE:
		fail(stage + ": temporal projection was not restored after drawing")
		return false
	print("TAA_BLOOM ", stage, " stock_taa=false perspective_restored=true")
	return true
