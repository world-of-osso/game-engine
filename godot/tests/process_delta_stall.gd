extends SceneTree

# One frame blocks 500 ms; the next process delta must carry that time. Godot drops the part
# of a slow frame past physics/common/max_physics_steps_per_frame physics ticks from the
# process delta, so the default 8 at 60 Hz caps it at 0.133 s.

const STALL_MSEC := 500
const MIN_DELTA := 0.45

class StallProbe:
	extends Node

	var frame := 0
	var stalled_delta := -1.0

	func _process(delta: float) -> void:
		frame += 1
		if frame == 10:
			OS.delay_msec(STALL_MSEC)
		elif frame == 11:
			stalled_delta = delta

var probe := StallProbe.new()

func _initialize() -> void:
	root.add_child(probe)

func _process(_delta: float) -> bool:
	if probe.stalled_delta < 0.0:
		return false
	var steps: int = ProjectSettings.get_setting("physics/common/max_physics_steps_per_frame")
	if probe.stalled_delta < MIN_DELTA:
		push_error("process delta after a %d ms stall was %.3f s (max_physics_steps_per_frame=%d); expected >= %.2f s" % [STALL_MSEC, probe.stalled_delta, steps, MIN_DELTA])
		quit(1)
	else:
		print("PASS: process delta after a %d ms stall was %.3f s (max_physics_steps_per_frame=%d)" % [STALL_MSEC, probe.stalled_delta, steps])
		quit(0)
	return true
