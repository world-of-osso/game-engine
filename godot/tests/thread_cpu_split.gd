## Per-frame CPU time of the main and render threads over a measured segment, so a frame
## splits into each thread's work and its waiting (wait = wall - CPU: the thread was not
## running, e.g. the main thread blocked on the render thread, or the render thread on
## present or the GPU). Both clocks are read on the main thread, once at each end of the
## segment: Windows charges thread time per scheduler tick (about 15.6 ms), so per-frame
## reads would be noise. The render thread is found by the name it is given here (the
## extension may not be called on it). Needs `GameClient.named_thread_cpu_ms`; older
## builds report `thread_split_error`.
extends RefCounted

const RENDER_THREAD_NAME := "FPS120_RENDER"
## Frames to wait for the render thread to take its name.
const NAME_FRAMES := 30

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
	var wall: float = (end.usec - _start.usec) / 1000.0 / frames
	var main_cpu: float = (end.main - _start.main) / frames
	var render_cpu: float = (end.render - _start.render) / frames
	return {
		"split_frame_ms": wall,
		"main_thread_cpu_ms": main_cpu, "main_thread_wait_ms": wall - main_cpu,
		"render_thread_cpu_ms": render_cpu, "render_thread_wait_ms": wall - render_cpu,
	}

func _mark(tree: SceneTree) -> Dictionary:
	if not ClassDB.class_has_method("GameClient", "named_thread_cpu_ms"):
		return {"error": "GameClient.named_thread_cpu_ms is missing in this build"}
	RenderingServer.call_on_render_thread(_name_render_thread)
	for frame in NAME_FRAMES:
		var render: float = ClassDB.class_call_static("GameClient", "named_thread_cpu_ms",
			RENDER_THREAD_NAME)
		var main: float = ClassDB.class_call_static("GameClient", "thread_cpu_ms")
		if render >= 0.0 and main >= 0.0:
			return {"frames": Engine.get_frames_drawn(), "usec": Time.get_ticks_usec(),
				"main": main, "render": render}
		await tree.process_frame
	return {"error": "no single thread named %s with a readable clock" % RENDER_THREAD_NAME}

## Runs on the render thread.
static func _name_render_thread() -> void:
	OS.set_thread_name(RENDER_THREAD_NAME)
