extends SceneTree
## RebuildGuard.violations budgets with the 210e7fed1 receipt's counts: per-light-step work
## passes within its steps and fails above them.

const RebuildGuard := preload("res://tests/rebuild_guard.gd")

func _initialize() -> void:
	# 301 frames over 1.2 s span two lit-time steps (LIGHT_STEP_S 0.9375).
	var steady := report({"sky_cubemap": 2, "environment": 2, "shader_broadcast": 301 + 2 * 32})
	if not RebuildGuard.violations(steady).is_empty():
		fail("Two light steps over budget: %s" % RebuildGuard.violations(steady))
		return
	var cube := RebuildGuard.violations(report({"sky_cubemap": 3}))
	if cube.size() != 1 or not cube[0].begins_with("sky_cubemap 3 in 301 frames"):
		fail("A third sky cubemap in two light steps passed: %s" % cube)
		return
	# The receipt's indoor environment before the camera settled: 13 applications.
	var light := RebuildGuard.violations(report({"environment": 13, "shader_broadcast": 301 + 13 * 32}))
	if light.size() != 2:
		fail("Per-frame light applications passed: %s" % light)
		return
	print("PASS: rebuild budgets allow per-light-step sky and light, reject more")
	quit(0)

func report(counts: Dictionary) -> Dictionary:
	return {"rebuild_frames": 301, "rebuild_seconds": 1.2, "rebuild_counts": counts,
		"rebuild_sites": {}}

func fail(message: String) -> void:
	push_error(message)
	quit(1)
