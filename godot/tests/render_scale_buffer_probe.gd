extends CompositorEffect

var _mutex := Mutex.new()
var _sample := {}
var _serial := 0

func _init() -> void:
	effect_callback_type = EFFECT_CALLBACK_TYPE_POST_TRANSPARENT

func _render_callback(_callback_type: int, render_data: RenderData) -> void:
	var buffers := render_data.get_render_scene_buffers() as RenderSceneBuffersRD
	if buffers == null:
		return
	# RenderData and its buffers are valid only inside this callback. Copy values under a lock.
	var internal_size := buffers.get_internal_size()
	var target_size := buffers.get_target_size()
	_mutex.lock()
	_serial += 1
	_sample = {"internal": internal_size, "target": target_size, "serial": _serial}
	_mutex.unlock()

func snapshot() -> Dictionary:
	_mutex.lock()
	var result := _sample.duplicate()
	_mutex.unlock()
	return result
