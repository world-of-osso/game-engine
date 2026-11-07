extends SceneTree
## RebuildGuard.violations budgets with the receipts' counts: per-light-step work passes
## within the lit-time steps the client clock allowed and fails above them.

const RebuildGuard := preload("res://tests/rebuild_guard.gd")

func _initialize() -> void:
	# One step boundary crossed in the segment, one due at its start: two steps.
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
	# f4654f139 outdoor idle_1: the harness screenshot stalled the step due before the
	# segment into it; the clock crossed two more boundaries in it (Windows DIAGNOSTIC).
	var late := report({"sky_cubemap": 3, "environment": 3, "shader_broadcast": 301 + 3 * 32}, 3)
	if not RebuildGuard.violations(late).is_empty():
		fail("A late light step failed the clock budget: %s" % RebuildGuard.violations(late))
		return
	# A fourth application in the same three clock steps is extra light work.
	var extra := RebuildGuard.violations(report({"environment": 4}, 3))
	if extra.size() != 1 or not extra[0].begins_with("environment 4 in 301 frames"):
		fail("An extra light application passed: %s" % extra)
		return
	var guard := RebuildGuard.new()
	guard._minutes = 1915.8125
	if guard._light_steps() != 1:
		fail("No client: %d light steps, expected 1" % guard._light_steps())
		return
	# The clock counts: idle_1's 1915.8125 -> 1915.875 crosses two boundaries, plus the due
	# step; across midnight 2879.99 -> 0.02 crosses one.
	var client := FakeClient.new()
	guard._client = client
	for case in [[1915.8125, 1915.875, 3], [1915.82, 1915.83, 1], [2879.99, 0.02, 2]]:
		guard._minutes = case[0]
		client.minutes = case[1]
		if guard._light_steps() != case[2]:
			fail("%s -> %s: %d light steps, expected %d" % [case[0], case[1], guard._light_steps(), case[2]])
			client.free()
			return
	client.free()
	print("PASS: rebuild budgets allow per-light-step sky and light, reject more")
	quit(0)

class FakeClient extends Node:
	var minutes := 0.0
	func world_minutes() -> float:
		return minutes

func report(counts: Dictionary, light_steps := 2) -> Dictionary:
	return {"rebuild_frames": 301, "rebuild_seconds": 1.6, "rebuild_light_steps": light_steps,
		"rebuild_counts": counts, "rebuild_sites": {}}

func fail(message: String) -> void:
	push_error(message)
	quit(1)
