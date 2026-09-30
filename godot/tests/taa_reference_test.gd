extends SceneTree
## CPU-only handcomputed fixtures. Main owns execution:
## Godot --headless --path godot/tests --script taa_reference_test.gd

var reference: Script
var failures := 0
var checks := 0


func _initialize() -> void:
	reference = load(get_script().resource_path.get_base_dir().path_join("taa_reference.gd"))
	if reference == null or not reference.can_instantiate():
		push_error("TAA CPU reference failed to load")
		quit(1)
		return
	test_reset_hdr_alpha()
	test_constant_clipping()
	test_confidence_motion_offscreen()
	test_depth_ties()
	test_catmull()
	print("TAA reference: %d checks, %d failures" % [checks, failures])
	quit(0 if failures == 0 else 1)


func expect(actual: float, expected: float, label: String, tolerance := 0.000003) -> void:
	checks += 1
	if not is_finite(actual) or absf(actual - expected) > tolerance:
		failures += 1
		push_error("%s: got %.10f, expected %.10f" % [label, actual, expected])


func field(extent: Vector2i, value: Vector4) -> Dictionary:
	var pixels: Array[Vector4] = []
	pixels.resize(extent.x * extent.y)
	pixels.fill(value)
	return {"extent": extent, "pixels": pixels}


func gray_neighborhood() -> Dictionary:
	# mean=.5, variance=2/9: .25 history lies inside the luminance box.
	return {
		"extent": Vector2i(3, 3),
		"pixels":
		[
			Vector4(0, 0, 0, .3),
			Vector4(1, 1, 1, .3),
			Vector4(0, 0, 0, .3),
			Vector4(1, 1, 1, .3),
			Vector4(.5, .5, .5, .3),
			Vector4(1, 1, 1, .3),
			Vector4(0, 0, 0, .3),
			Vector4(1, 1, 1, .3),
			Vector4(0, 0, 0, .3),
		]
	}


func resolve(
	current: Dictionary, history: Dictionary, velocity: Vector2, reset := false, hdr := false
) -> Dictionary:
	return reference.resolve(
		current,
		history,
		field(current.extent, Vector4.ZERO),
		field(current.extent, Vector4(velocity.x, velocity.y, 0, 0)),
		current.extent / 2,
		reset,
		hdr
	)


func test_reset_hdr_alpha() -> void:
	var current := field(Vector2i.ONE, Vector4(4, 2, 1, .37))
	var history := field(Vector2i.ONE, Vector4(99, 99, 99, 7))
	var result := resolve(current, history, Vector2.ZERO, true, true)
	expect(result.history.x, .8, "reset HDR history red")
	expect(result.history.y, .4, "reset HDR history green")
	expect(result.history.z, .2, "reset HDR history blue")
	expect(result.history.w, 1.0 / .015, "reset confidence")
	expect(result.resolved.x, 4, "HDR inverse red")
	expect(result.resolved.y, 2, "HDR inverse green")
	expect(result.resolved.z, 1, "HDR inverse blue")
	expect(result.resolved.w, .37, "original alpha")
	result = resolve(current, history, Vector2.ZERO, true)
	expect(result.history.x, 4, "reset without tonemap does not saturate")
	expect(result.resolved.x, 4, "reset raw HDR")


func test_constant_clipping() -> void:
	var current := field(Vector2i(3, 3), Vector4(.25, .5, .75, .73))
	var history := field(Vector2i(3, 3), Vector4(.9, .1, .8, 90))
	var result := resolve(current, history, Vector2.ZERO)
	for channel in range(3):
		expect(result.resolved[channel], [.25, .5, .75][channel], "zero variance clips colour")
	expect(result.history.w, 100, "static confidence increments without upper cap")
	expect(result.resolved.w, .73, "nonreset retains alpha")
	current = field(Vector2i(3, 3), Vector4(2, 2, 2, .73))
	result = resolve(current, history, Vector2.ZERO)
	expect(result.resolved.x, 1.015, "YCoCg conversion saturates history, not current")


func test_confidence_motion_offscreen() -> void:
	var current := gray_neighborhood()
	var history := field(Vector2i(3, 3), Vector4(.25, .25, .25, 200))
	history.pixels[4].w = 10
	var result := resolve(current, history, Vector2.ZERO)
	expect(result.history.w, 20, "static confidence")
	expect(result.resolved.x, .2625, "static blend 1/20")
	# A sub-.01 CURRENT-pixel shift crosses history texels at the larger extent.
	# Original UV hits confidence 10; reprojected UV hits confidence 200.
	history = field(Vector2i(512, 1), Vector4(.25, .25, .25, 200))
	history.pixels[256].w = 10
	result = resolve(current, history, Vector2(.009 / 3.0, 0))
	expect(result.history.w, 20, "confidence samples unreprojected nearest UV")
	result = resolve(current, history, Vector2(.01 / 3.0, 0))
	expect(result.history.w, 1, "strict .01 pixel threshold resets confidence")
	expect(result.resolved.x, .275, "motion reset uses .1 current weight")
	result = resolve(current, history, Vector2(1, 0))
	expect(result.history.w, 1, "offscreen resets confidence")
	expect(result.resolved.x, .5, "offscreen rejects history")
	expect(result.resolved.w, .3, "offscreen original alpha")
	result = resolve(current, history, Vector2(.5, 0))
	expect(result.resolved.x, .275, "history exactly on border is accepted")
	var hdr := field(Vector2i(3, 3), Vector4(4, 2, 1, .6))
	result = resolve(hdr, history, Vector2(1, 0), false, true)
	expect(result.history.x, .8, "nonreset HDR stores mapped history")
	expect(result.resolved.x, 4, "nonreset HDR restores radiance")


func test_depth_ties() -> void:
	var depth := field(Vector2i(7, 7), Vector4.ZERO)
	var motion := field(Vector2i(7, 7), Vector4.ZERO)
	# Center then TL, TR, BL, BR; offsets are TWO texels, positive Y first.
	var indices := [24, 36, 40, 8, 12]
	for i in range(5):
		depth.pixels[indices[i]].x = .8
		motion.pixels[indices[i]].x = (i + 1) * .1
	var selected: Vector2 = reference.closest_motion(depth, motion, Vector2(.5, .5), Vector2(7, 7))
	expect(selected.x, .1, "equal depth retains center")
	depth.pixels[24].x = .2
	selected = reference.closest_motion(depth, motion, Vector2(.5, .5), Vector2(7, 7))
	expect(selected.x, .2, "equal corner maxima retain TL")
	for i in range(1, 4):
		depth.pixels[indices[i]].x = .2
		selected = reference.closest_motion(depth, motion, Vector2(.5, .5), Vector2(7, 7))
		expect(selected.x, (i + 2) * .1, "next tied maximum preserves corner order")
	for i in range(1, 5):
		depth.pixels[indices[i]].x = .8 + i * .02
	selected = reference.closest_motion(depth, motion, Vector2(.5, .5), Vector2(7, 7))
	expect(selected.x, .5, "strict increasing depth selects BR")


func test_catmull() -> void:
	var history := field(Vector2i(2, 2), Vector4(.5, .25, .75, 10))
	# At f=(.5,.5), w0=w3=-1/16, w12=9/8.
	# Five weights sum to 63/64, NOT one. History extent differs from current.
	var value: Vector3 = reference.sample_history(history, Vector2(.5, .5), Vector2(4, 4))
	expect(value.x, .5 * 63.0 / 64.0, "unnormalized five taps red")
	expect(value.y, .25 * 63.0 / 64.0, "unnormalized five taps green")
	expect(value.z, .75 * 63.0 / 64.0, "unnormalized five taps blue")
	value = reference.sample_history(history, Vector2(.375, .375), Vector2(4, 4))
	expect(value.x, .5, "current-extent texel center gives unit weight")
