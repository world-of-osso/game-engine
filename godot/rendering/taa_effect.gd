extends CompositorEffect
## One camera's Bevy 0.19 temporal history, at the physical internal extent.
## Raster stores preserve RGBA16F or RGBA8 sRGB legacy framebuffer semantics.
## Controller owns attachment, enabled state, camera jitter, and disposal.

const HALF_FORMAT := RenderingDevice.DATA_FORMAT_R16G16B16A16_SFLOAT
const LDR_FORMAT := RenderingDevice.DATA_FORMAT_R8G8B8A8_SRGB
const TEXTURE_USAGE := (
	RenderingDevice.TEXTURE_USAGE_SAMPLING_BIT | RenderingDevice.TEXTURE_USAGE_COLOR_ATTACHMENT_BIT
)
const SHADER_PATHS := [
	"res://shaders/taa_copy.glsl",
	"res://shaders/taa_resolve.glsl",
	"res://shaders/bloom_fullscreen.glsl",
]

var _mutex := Mutex.new()
var _jitter_motion := Vector2.ZERO
var _hdr := false
var _disposed := false
var _error := ""
var _rd: RenderingDevice
var _sources: Array[String] = []
var _shaders: Array[RID] = []
var _pipelines: Array[RID] = [RID(), RID(), RID()]
var _pipeline_formats: Array[int] = [-1, -1, -1]
var _samplers: Array[RID] = []
# Current, resolved, history A, history B. All share the legacy format/extent.
var _textures: Array[RID] = []
var _size := Vector2i.ZERO
var _format := -1
var _history_read := 0
var _reset := true


func _init() -> void:
	effect_callback_type = EFFECT_CALLBACK_TYPE_POST_TRANSPARENT
	needs_motion_vectors = true
	access_resolved_depth = true
	access_resolved_color = true
	# Immutable source reads on construction; compilation belongs to render thread.
	for path in SHADER_PATHS:
		_sources.append(
			(
				FileAccess
				. get_file_as_string(path)
				. replace("#[compute]\n", "")
				. replace("#[fragment]\n", "")
				. replace("#[vertex]\n", "")
			)
		)


func configure(jitter_motion: Vector2, hdr: bool) -> void:
	_mutex.lock()
	_jitter_motion = jitter_motion
	_hdr = hdr
	_mutex.unlock()


func snapshot() -> Dictionary:
	_mutex.lock()
	var state := {"disposed": _disposed, "error": _error}
	_mutex.unlock()
	return state


func _render_callback(callback_type: int, render_data: RenderData) -> void:
	_mutex.lock()
	var jitter_motion := _jitter_motion
	var hdr := _hdr
	var stopped := _disposed or not _error.is_empty()
	_mutex.unlock()
	if stopped:
		return
	if callback_type != EFFECT_CALLBACK_TYPE_POST_TRANSPARENT:
		_fail("Expected POST_TRANSPARENT callback")
		return
	if not jitter_motion.is_finite():
		_fail("Jitter motion must be finite")
		return
	var buffers := render_data.get_render_scene_buffers() as RenderSceneBuffersRD
	if buffers == null:
		_fail("Callback has no RenderSceneBuffersRD")
		return
	if buffers.get_view_count() != 1:
		_fail("Only mono rendering is supported")
		return
	var size := buffers.get_internal_size()
	if size.x <= 0 or size.y <= 0:
		return
	if _rd == null and not _initialize_device():
		return
	var color := buffers.get_color_layer(0)
	var depth := buffers.get_depth_layer(0)
	var motion := buffers.get_velocity_layer(0)
	if not _validate_inputs(color, depth, motion, size):
		return
	var legacy_format := HALF_FORMAT if hdr else LDR_FORMAT
	if (size != _size or legacy_format != _format) and not _allocate(size, legacy_format):
		return
	if _render_frame(color, depth, motion, jitter_motion, hdr):
		_history_read = 1 - _history_read
		_reset = false


func _initialize_device() -> bool:
	_rd = RenderingServer.get_rendering_device()
	if _rd == null:
		return _fail("Requires a render-thread RenderingDevice")
	for filter_mode in [
		RenderingDevice.SAMPLER_FILTER_NEAREST, RenderingDevice.SAMPLER_FILTER_LINEAR
	]:
		var state := RDSamplerState.new()
		state.min_filter = filter_mode
		state.mag_filter = filter_mode
		state.repeat_u = RenderingDevice.SAMPLER_REPEAT_MODE_CLAMP_TO_EDGE
		state.repeat_v = RenderingDevice.SAMPLER_REPEAT_MODE_CLAMP_TO_EDGE
		var sampler := _rd.sampler_create(state)
		if not sampler.is_valid():
			return _fail("Could not create clamp sampler")
		_samplers.append(sampler)
	for index in range(2):
		if not _compile_shader(index):
			return false
	return true


func _compile_shader(index: int) -> bool:
	if _sources[index].is_empty() or _sources[2].is_empty():
		return _fail("Missing shader source: " + SHADER_PATHS[index])
	var source := RDShaderSource.new()
	source.source_vertex = _sources[2]
	var fragment := _sources[index]
	if index == 1:
		fragment = fragment.replace(
			"#version 450", "#version 450\n#define TAA_RASTER\n#define TAA_GODOT_MOTION"
		)
	source.source_fragment = fragment
	var spirv := _rd.shader_compile_spirv_from_source(source, false)
	if spirv == null:
		return _fail("Could not compile " + SHADER_PATHS[index])
	for stage in [RenderingDevice.SHADER_STAGE_VERTEX, RenderingDevice.SHADER_STAGE_FRAGMENT]:
		var error := spirv.get_stage_compile_error(stage)
		if not error.is_empty():
			return _fail("Shader %s stage %d: %s" % [SHADER_PATHS[index], stage, error])
	var shader := _rd.shader_create_from_spirv(spirv, "bevy-taa-%d" % index)
	if not shader.is_valid():
		return _fail("Could not create " + SHADER_PATHS[index])
	_shaders.append(shader)
	return true


func _allocate(size: Vector2i, legacy_format: int) -> bool:
	if not _rd.texture_is_format_supported_for_usage(legacy_format, TEXTURE_USAGE):
		return _fail("Legacy format %d lacks sampled/attachment support" % legacy_format)
	_release(_rd, _pipelines)
	_release(_rd, _textures)
	_pipelines.assign([RID(), RID(), RID()])
	_pipeline_formats.assign([-1, -1, -1])
	_textures.clear()
	for index in range(4):
		var format := RDTextureFormat.new()
		format.width = size.x
		format.height = size.y
		format.format = legacy_format
		format.usage_bits = TEXTURE_USAGE
		var texture := _rd.texture_create(format, RDTextureView.new())
		if not texture.is_valid():
			return _fail("Could not allocate temporal texture %d at %s" % [index, size])
		_textures.append(texture)
	_size = size
	_format = legacy_format
	_history_read = 0
	_reset = true
	return true


func _validate_inputs(color: RID, depth: RID, motion: RID, size: Vector2i) -> bool:
	var inputs: Array[RID] = [color, depth, motion]
	for index in range(inputs.size()):
		var texture := inputs[index]
		if not texture.is_valid() or not _rd.texture_is_valid(texture):
			return _fail("Invalid scene input %d" % index)
		var format := _rd.texture_get_format(texture)
		if (
			Vector2i(format.width, format.height) != size
			or format.samples != RenderingDevice.TEXTURE_SAMPLES_1
		):
			return _fail("Scene input %d must be resolved at internal extent %s" % [index, size])
		var required := TEXTURE_USAGE if index == 0 else RenderingDevice.TEXTURE_USAGE_SAMPLING_BIT
		if (format.usage_bits & required) != required:
			return _fail("Scene input %d lacks required usage" % index)
		if index == 0 and format.format != HALF_FORMAT:
			return _fail("Expected scene RGBA16F, got format %d" % format.format)
	return true


func _create_binding(shader_index: int, textures: Array[RID]) -> RID:
	var uniforms: Array[RDUniform] = []
	for index in range(textures.size()):
		var uniform := RDUniform.new()
		uniform.uniform_type = RenderingDevice.UNIFORM_TYPE_SAMPLER_WITH_TEXTURE
		uniform.binding = index
		uniform.add_id(_samplers[1 if shader_index == 1 and index == 1 else 0])
		uniform.add_id(textures[index])
		uniforms.append(uniform)
	return _rd.uniform_set_create(uniforms, _shaders[shader_index], 0)


func _cache_pipeline(index: int, framebuffer: RID) -> bool:
	var format := _rd.framebuffer_get_format(framebuffer)
	if _pipelines[index].is_valid():
		if _pipeline_formats[index] != format:
			return _fail("Framebuffer format changed for pipeline %d" % index)
		return true
	var attachments: Array[RDPipelineColorBlendStateAttachment] = []
	for attachment in range(2 if index == 1 else 1):
		attachments.append(RDPipelineColorBlendStateAttachment.new())
	var blend := RDPipelineColorBlendState.new()
	blend.attachments = attachments
	var pipeline := _rd.render_pipeline_create(
		_shaders[1 if index == 1 else 0],
		format,
		-1,
		RenderingDevice.RENDER_PRIMITIVE_TRIANGLES,
		RDPipelineRasterizationState.new(),
		RDPipelineMultisampleState.new(),
		RDPipelineDepthStencilState.new(),
		blend
	)
	_pipelines[index] = pipeline
	if not pipeline.is_valid() or not _rd.render_pipeline_is_valid(pipeline):
		return _fail("Could not create raster pipeline %d" % index)
	_pipeline_formats[index] = format
	return true


func _render_frame(color: RID, depth: RID, motion: RID, jitter_motion: Vector2, hdr: bool) -> bool:
	var history := _textures[2 + _history_read]
	var bindings: Array[RID] = [
		_create_binding(0, [color]),
		_create_binding(1, [_textures[0], history, history, depth, motion]),
		_create_binding(0, [_textures[1]]),
	]
	var current_target: Array[RID] = [_textures[0]]
	var resolve_targets: Array[RID] = [_textures[1], _textures[2 + 1 - _history_read]]
	var scene_target: Array[RID] = [color]
	var framebuffers: Array[RID] = [
		_rd.framebuffer_create(current_target),
		_rd.framebuffer_create(resolve_targets),
		_rd.framebuffer_create(scene_target),
	]
	var valid := true
	for resource in bindings + framebuffers:
		if not resource.is_valid():
			valid = _fail("Could not create temporal uniform set or framebuffer")
			break
	var parameters := PackedByteArray()
	parameters.resize(16)
	parameters.encode_u32(0, 1 if _reset else 0)
	parameters.encode_u32(4, 1 if hdr else 0)
	parameters.encode_float(8, jitter_motion.x)
	parameters.encode_float(12, jitter_motion.y)
	# Distinct input/output textures throughout; RD tracks attachment/sample hazards.
	for index in range(3):
		if not valid:
			break
		valid = _cache_pipeline(index, framebuffers[index])
		if valid:
			valid = _draw(index, framebuffers[index], bindings[index], parameters)
	_release(_rd, bindings)
	_release(_rd, framebuffers)
	return valid


func _draw(index: int, framebuffer: RID, binding: RID, parameters: PackedByteArray) -> bool:
	var draw := _rd.draw_list_begin(framebuffer, RenderingDevice.DRAW_DEFAULT_ALL)
	if draw == RenderingDevice.INVALID_ID:
		return _fail("Could not begin raster draw %d" % index)
	_rd.draw_list_bind_render_pipeline(draw, _pipelines[index])
	_rd.draw_list_bind_uniform_set(draw, binding, 0)
	if index == 1:
		_rd.draw_list_set_push_constant(draw, parameters, parameters.size())
	_rd.draw_list_draw(draw, false, 1, 3)
	_rd.draw_list_end()
	return true


func _fail(message: String) -> bool:
	_mutex.lock()
	_error = message
	_mutex.unlock()
	enabled = false
	push_error("TAA: " + message)
	return false


func dispose() -> void:
	RenderingServer.call_on_render_thread(_dispose_on_render_thread)


func _dispose_on_render_thread() -> void:
	_mutex.lock()
	var already_disposed := _disposed
	_disposed = true
	_mutex.unlock()
	if already_disposed:
		return
	_release(_rd, _textures)
	_release(_rd, _pipelines)
	_release(_rd, _shaders)
	_release(_rd, _samplers)
	_textures.clear()
	_pipelines.clear()
	_shaders.clear()
	_samplers.clear()


static func _release(rd: RenderingDevice, rids: Array[RID]) -> void:
	if rd == null:
		return
	for rid in rids:
		if rid.is_valid():
			rd.free_rid(rid)


func _notification(what: int) -> void:
	if what == NOTIFICATION_PREDELETE and _rd != null and not _disposed:
		# Capture only owned handles; PREDELETE cannot call instance methods.
		var owned: Array[RID] = []
		owned.assign(_textures + _pipelines + _shaders + _samplers)
		RenderingServer.call_on_render_thread(_release.bind(_rd, owned))
