## Per-frame CPU time of the main and render threads over a measured segment, so a frame
## splits into each thread's work and its waiting (wait = wall - CPU: the thread was not
## running, e.g. the main thread blocked on the render thread, or the render thread on
## present or the GPU). Each thread clock is read once at each end of the segment:
## Windows charges thread time per scheduler tick (about 15.6 ms), so per-frame reads
## would be noise. Needs `GameClient.thread_cpu_ms`; older builds report the error field.
extends RefCounted

## Frames to wait for the render thread to run the queued clock read.
const RENDER_SAMPLE_FRAMES := 30

var _lock := Mutex.new()
var _render_sample: Array = []
var _start := {}

func start(tree: SceneTree) -> void:
	_start = await _mark(tree)

## `main_thread_cpu_ms`, `main_thread_wait_ms`, `render_thread_cpu_ms`,
## `render_thread_wait_ms` and `split_frame_ms` per frame since `start`.
func finish(tree: SceneTree) -> Dictionary:
	var end := await _mark(tree)
	if _start.has("error") or end.has("error"):
		return {"thread_split_error": _start.get("error", end.get("error"))}
	var frames: int = end.frames - _start.frames
	if frames <= 0:
		return {"thread_split_error": "no frames between thread clock reads"}
	var main_wall: float = (end.usec - _start.usec) / 1000.0 / frames
	var render_wall: float = (end.render_usec - _start.render_usec) / 1000.0 / frames
	var main_cpu: float = (end.main - _start.main) / frames
	var render_cpu: float = (end.render - _start.render) / frames
	return {
		"split_frame_ms": main_wall,
		"main_thread_cpu_ms": main_cpu, "main_thread_wait_ms": main_wall - main_cpu,
		"render_thread_cpu_ms": render_cpu, "render_thread_wait_ms": render_wall - render_cpu,
	}

func _mark(tree: SceneTree) -> Dictionary:
	if not ClassDB.class_has_method("GameClient", "thread_cpu_ms"):
		return {"error": "GameClient.thread_cpu_ms is missing in this build"}
	_lock.lock()
	_render_sample = []
	_lock.unlock()
	RenderingServer.call_on_render_thread(_sample_render_thread)
	var mark := {"frames": Engine.get_frames_drawn(), "usec": Time.get_ticks_usec(),
		"main": _thread_cpu_ms()}
	for frame in RENDER_SAMPLE_FRAMES:
		_lock.lock()
		var sample := _render_sample.duplicate()
		_lock.unlock()
		if not sample.is_empty():
			mark["render"] = sample[0]
			mark["render_usec"] = sample[1]
			if mark.main < 0.0 or mark.render < 0.0:
				return {"error": "thread CPU clock unavailable"}
			return mark
		await tree.process_frame
	return {"error": "render thread did not run the clock read in %d frames" % RENDER_SAMPLE_FRAMES}

## Runs on the render thread.
func _sample_render_thread() -> void:
	var sample := [_thread_cpu_ms(), Time.get_ticks_usec()]
	_lock.lock()
	_render_sample = sample
	_lock.unlock()

static func _thread_cpu_ms() -> float:
	return ClassDB.class_call_static("GameClient", "thread_cpu_ms")
