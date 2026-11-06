extends SceneTree

## Rendered check of thread_cpu_split.gd against an independent clock: the main thread
## does fixed work each frame for FRAMES frames, and the split's main-thread CPU per
## frame must match the kernel's scheduled run time (/proc/thread-self/schedstat, Linux),
## while the render thread's clock advances and stays within the frame. Prints
## `PASS: thread_cpu_split` or `FAIL: ...` with the split, and exits 0 or 1.

const ThreadCpuSplit = preload("res://tests/thread_cpu_split.gd")
const FRAMES := 120
const WORK_ITERATIONS := 20000

func _initialize() -> void:
	call_deferred("run_check")

func run_check() -> void:
	var split := ThreadCpuSplit.new()
	await split.start(self)
	var schedstat_start := schedstat_ms()
	var frames_start := Engine.get_frames_drawn()
	var sink := 0
	for frame in FRAMES:
		for i in WORK_ITERATIONS:
			sink = (sink * 31 + i) % 1000003
		await process_frame
	var schedstat_end := schedstat_ms()
	var frames := Engine.get_frames_drawn() - frames_start
	var threads := await split.finish(self)
	var reference := (schedstat_end - schedstat_start) / frames
	print("THREAD_SPLIT ", JSON.stringify(threads), " schedstat_main_ms=", reference, " sink=", sink)
	var problems: Array[String] = []
	if threads.has("thread_split_error"):
		problems.append(str(threads.thread_split_error))
	else:
		# The split's window adds the frames that read the clocks; allow 10%.
		if absf(threads.main_thread_cpu_ms - reference) > reference * 0.1 + 0.05:
			problems.append("main thread CPU %.3f ms, schedstat %.3f ms" % [threads.main_thread_cpu_ms, reference])
		if threads.main_thread_cpu_ms > threads.split_frame_ms * 1.02:
			problems.append("main thread CPU exceeds the frame")
		if threads.render_thread_cpu_ms <= 0.0:
			problems.append("render thread clock did not advance")
		if threads.render_thread_cpu_ms > threads.split_frame_ms * 1.02:
			problems.append("render thread CPU exceeds the frame")
	if problems.is_empty():
		print("PASS: thread_cpu_split")
		quit(0)
	else:
		print("FAIL: ", ", ".join(problems))
		quit(1)

func schedstat_ms() -> float:
	var file := FileAccess.open("/proc/thread-self/schedstat", FileAccess.READ)
	return int(file.get_line().split(" ")[0]) / 1e6
