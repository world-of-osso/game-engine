extends CanvasLayer

const REFRESH_SECONDS := 0.5
const GRAPH_SIZE := Vector2(192, 64)

class FrameTimeGraph extends Control:
	const TARGET_FPS := 60.0
	const MIN_FPS := 30.0
	const HISTORY_SIZE := 192

	var frame_times := PackedFloat32Array()

	func add_frame(delta: float) -> void:
		frame_times.append(delta)
		if frame_times.size() > HISTORY_SIZE:
			frame_times.remove_at(0)
		queue_redraw()

	func _draw() -> void:
		for index in frame_times.size():
			var frame_time := frame_times[index]
			if frame_time <= 0.0:
				continue
			var fps := 1.0 / frame_time
			var color := Color.GREEN
			if fps < MIN_FPS:
				color = Color.RED
			elif fps < TARGET_FPS:
				color = Color.YELLOW
			var height := minf(frame_time * TARGET_FPS * size.y, size.y)
			draw_rect(Rect2(float(index), size.y - height, 1.0, height), color)

var elapsed := 0.0
var fps_label: Label
var graph: FrameTimeGraph

func _ready() -> void:
	fps_label = Label.new()
	fps_label.name = "FpsLabel"
	fps_label.text = "FPS: "
	fps_label.add_theme_font_size_override("font_size", 32)
	fps_label.add_theme_color_override("font_color", Color.WHITE)
	fps_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(fps_label)
	fps_label.size = Vector2(192, 40)

	graph = FrameTimeGraph.new()
	graph.name = "FrameTimeGraph"
	graph.position = Vector2(0, fps_label.size.y)
	graph.size = GRAPH_SIZE
	graph.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(graph)
	visible = get_parent().fps_overlay_enabled()

func _process(delta: float) -> void:
	visible = get_parent().fps_overlay_enabled()
	if not visible:
		return
	graph.add_frame(delta)
	elapsed += delta
	if elapsed < REFRESH_SECONDS:
		return
	elapsed = 0.0
	var fps := Engine.get_frames_per_second()
	if fps > 0.0:
		fps_label.text = "FPS: %.2f" % fps
