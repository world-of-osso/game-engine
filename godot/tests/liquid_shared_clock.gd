extends SceneTree

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	var clock = root.get_node("M2MaterialClock")
	clock.set_process(false)
	var start: float = clock.elapsed_time_ms()
	if start >= 3599500.0 or not clock.advance_time_ms(3599500.0 - start):
		fail("Could not set the shared clock near its one-hour wrap")
		return
	if not close_clock(3599500.0):
		return
	if not clock.advance_time_ms(750.0) or not close_clock(250.0):
		return
	await process_frame
	if not close_clock(250.0):
		return
	print("PASS LIQUID_SHARED_CLOCK")
	quit(0)

func close_clock(expected: float) -> bool:
	var actual: float = root.get_node("M2MaterialClock").liquid_time_ms()
	if absf(actual - expected) > 0.01:
		fail("Liquid clock %s, expected %s" % [actual, expected])
		return false
	return true

func fail(message: String) -> void:
	push_error(message)
	quit(1)
