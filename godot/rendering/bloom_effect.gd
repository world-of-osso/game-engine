extends CompositorEffect
## Configured Bevy 0.19 OLD_SCHOOL additive bloom, before Godot tonemapping.
## Full-image viewport, uniform scale; 512-height aspect-preserving 8-level chain.
## Compute downsample; raster tent upsample with hardware additive blending.
## Separate textures for all neighbourhood reads; destination is attachment-only.
## Packed R11G11B10 intermediates retain upstream unsigned-float quantization.
## Scene input/output is RGBA16F; alpha is preserved. Finite nonnegative scene
## input is required; final RGB exceeding 65504 can overflow the half framebuffer
## (as upstream can). No NaN sanitization, saturation, stock-glow substitution,
## anamorphic scale, energy-conserving mode, subviewport, or format fallback.

const LEVEL_COUNT := 8
const MIP_HEIGHT := 512
const PACKED_FORMAT := RenderingDevice.DATA_FORMAT_B10G11R11_UFLOAT_PACK32
const TEXTURE_USAGE := (
	RenderingDevice.TEXTURE_USAGE_STORAGE_BIT
	| RenderingDevice.TEXTURE_USAGE_SAMPLING_BIT
	| RenderingDevice.TEXTURE_USAGE_COLOR_ATTACHMENT_BIT
)
const SHADER_PATHS := [
	"res://shaders/bloom_downsample.glsl",
	"res://shaders/bloom_upsample.glsl",
	"res://shaders/bloom_fullscreen.glsl",
]

var _mutex := Mutex.new()
var _settings := Vector2(0.0, 0.08)
var _rd: RenderingDevice
var _sources: Array[String] = []
var _shaders: Array[RID] = []
var _pipelines: Array[RID] = []
var _raster_formats: Array[int] = [-1, -1]
var _sampler := RID()
var _textures: Array[RID] = []
var _sizes: Array[Vector2i] = []
var _internal_size := Vector2i.ZERO
var _disposed := false
var _failed := false


func _init() -> void:
	effect_callback_type = EFFECT_CALLBACK_TYPE_POST_TRANSPARENT
	access_resolved_color = true
	# Read files on construction, never perform filesystem IO in the callback.
	for path in SHADER_PATHS:
		_sources.append(
			FileAccess.get_file_as_string(path)
			. replace("#[compute]\n", "")
			. replace("#[fragment]\n", "")
			. replace("#[vertex]\n", "")
		)


func configure(active: bool, intensity: float) -> void:
	if not is_finite(intensity):
		push_error("Bloom intensity must be finite")
		return
	_mutex.lock()
	_settings = Vector2(1.0 if active else 0.0, clampf(intensity, 0.0, 1.0))
	_mutex.unlock()


func _render_callback(callback_type: int, render_data: RenderData) -> void:
	if _disposed or _failed:
		return
	_mutex.lock()
	var settings := _settings
	_mutex.unlock()
	if settings.x == 0.0 or settings.y == 0.0:
		return
	if callback_type != EFFECT_CALLBACK_TYPE_POST_TRANSPARENT:
		_fail("Expected POST_TRANSPARENT callback")
		return
	var buffers := render_data.get_render_scene_buffers() as RenderSceneBuffersRD
	if buffers == null:
		_fail("Callback has no RenderSceneBuffersRD")
		return
	var size := buffers.get_internal_size()
	if size.x <= 0 or size.y <= 0:
		return
	if _rd == null and not _initialize_device():
		return
	if size != _internal_size and not _allocate_pyramid(size):
		return
	# Views reuse the scratch chain sequentially, never share partially filtered
	# contents. Uniform sets are frame-owned; scene layer RIDs may change on resize.
	for view in range(buffers.get_view_count()):
		var color := buffers.get_color_layer(view)
		if not _validate_scene(color, size) or not _dispatch_chain(color, settings.y):
			return


func _initialize_device() -> bool:
	_rd = RenderingServer.get_rendering_device()
	if _rd == null:
		return _fail("Requires a render-thread RenderingDevice")
	if not _rd.texture_is_format_supported_for_usage(PACKED_FORMAT, TEXTURE_USAGE):
		return _fail("R11G11B10 packed storage/sampling/attachment unsupported; no format fallback")
	var state := RDSamplerState.new()
	state.min_filter = RenderingDevice.SAMPLER_FILTER_LINEAR
	state.mag_filter = RenderingDevice.SAMPLER_FILTER_LINEAR
	state.repeat_u = RenderingDevice.SAMPLER_REPEAT_MODE_CLAMP_TO_EDGE
	state.repeat_v = RenderingDevice.SAMPLER_REPEAT_MODE_CLAMP_TO_EDGE
	_sampler = _rd.sampler_create(state)
	if not _sampler.is_valid():
		return _fail("Could not create bloom linear clamp sampler")
	for index in range(2):
		if not _compile_shader(index):
			return false
	return true


func _compile_shader(index: int) -> bool:
	if _sources[index].is_empty() or (index == 1 and _sources[2].is_empty()):
		return _fail("Missing bloom shader source")
	var source := RDShaderSource.new()
	var stages: Array[int] = [RenderingDevice.SHADER_STAGE_COMPUTE]
	if index == 0:
		source.source_compute = _sources[0]
	else:
		source.source_vertex = _sources[2]
		source.source_fragment = _sources[1]
		stages = [RenderingDevice.SHADER_STAGE_VERTEX, RenderingDevice.SHADER_STAGE_FRAGMENT]
	var spirv := _rd.shader_compile_spirv_from_source(source, false)
	if spirv == null:
		return _fail("Could not compile bloom shader %d" % index)
	for stage in stages:
		var error := spirv.get_stage_compile_error(stage)
		if not error.is_empty():
			return _fail("Shader %s stage %d: %s" % [SHADER_PATHS[index], stage, error])
	var shader := _rd.shader_create_from_spirv(spirv, "bevy-bloom-%d" % index)
	if not shader.is_valid():
		return _fail("Could not create bloom shader %d" % index)
	_shaders.append(shader)
	if index == 0:
		var pipeline := _rd.compute_pipeline_create(shader)
		if not pipeline.is_valid():
			return _fail("Could not create bloom downsample pipeline")
		_pipelines.append(pipeline)
	else:
		# Packed-mip and half-scene pipelines are cached by framebuffer format.
		_pipelines.append(RID())
		_pipelines.append(RID())
	return true


func _cache_raster_pipeline(index: int, framebuffer: RID) -> bool:
	var format := _rd.framebuffer_get_format(framebuffer)
	if _pipelines[index].is_valid():
		if _raster_formats[index - 1] != format:
			return _fail("Bloom framebuffer format changed for pipeline %d" % index)
		return true
	var attachment := RDPipelineColorBlendStateAttachment.new()
	attachment.enable_blend = true
	attachment.src_color_blend_factor = RenderingDevice.BLEND_FACTOR_CONSTANT_COLOR
	attachment.dst_color_blend_factor = RenderingDevice.BLEND_FACTOR_ONE
	attachment.color_blend_op = RenderingDevice.BLEND_OP_ADD
	attachment.src_alpha_blend_factor = RenderingDevice.BLEND_FACTOR_ZERO
	attachment.dst_alpha_blend_factor = RenderingDevice.BLEND_FACTOR_ONE
	attachment.alpha_blend_op = RenderingDevice.BLEND_OP_ADD
	var blend := RDPipelineColorBlendState.new()
	var attachments: Array[RDPipelineColorBlendStateAttachment] = [attachment]
	blend.attachments = attachments
	var pipeline := _rd.render_pipeline_create(
		_shaders[1],
		format,
		-1,
		RenderingDevice.RENDER_PRIMITIVE_TRIANGLES,
		RDPipelineRasterizationState.new(),
		RDPipelineMultisampleState.new(),
		RDPipelineDepthStencilState.new(),
		blend,
		RenderingDevice.DYNAMIC_STATE_BLEND_CONSTANTS
	)
	_pipelines[index] = pipeline
	if not pipeline.is_valid() or not _rd.render_pipeline_is_valid(pipeline):
		return _fail("Could not create bloom raster pipeline %d" % index)
	_raster_formats[index - 1] = format
	return true


func _allocate_pyramid(size: Vector2i) -> bool:
	var width := maxi(1, roundi(float(size.x) * MIP_HEIGHT / size.y))
	# Uniform render-scale changes often leave all OLD_SCHOOL mip dimensions
	# unchanged. Reuse them even when the scene's internal extent changes.
	if _textures.size() == LEVEL_COUNT and _sizes[0].x == width:
		_internal_size = size
		return true
	_release(_rd, _textures)
	_textures.clear()
	_sizes.clear()
	for level in range(LEVEL_COUNT):
		var extent := Vector2i(maxi(1, width >> level), maxi(1, MIP_HEIGHT >> level))
		var format := RDTextureFormat.new()
		format.width = extent.x
		format.height = extent.y
		format.format = PACKED_FORMAT
		format.usage_bits = TEXTURE_USAGE
		var texture := _rd.texture_create(format, RDTextureView.new())
		if not texture.is_valid():
			return _fail("Could not allocate bloom mip %d at %s" % [level, extent])
		_textures.append(texture)
		_sizes.append(extent)
	_internal_size = size
	return true


func _validate_scene(color: RID, size: Vector2i) -> bool:
	if not color.is_valid() or not _rd.texture_is_valid(color):
		return _fail("Invalid scene color layer")
	var format := _rd.texture_get_format(color)
	if format.format != RenderingDevice.DATA_FORMAT_R16G16B16A16_SFLOAT:
		return _fail("Expected scene RGBA16F, got format %d" % format.format)
	if Vector2i(format.width, format.height) != size:
		return _fail("Scene color dimensions differ from internal render size")
	var required := (
		RenderingDevice.TEXTURE_USAGE_SAMPLING_BIT
		| RenderingDevice.TEXTURE_USAGE_COLOR_ATTACHMENT_BIT
	)
	if (format.usage_bits & required) != required:
		return _fail("Scene layer requires sampling and color attachment usage")
	return true


func _create_binding(index: int, input: RID, target: RID) -> RID:
	var sampled := RDUniform.new()
	sampled.uniform_type = RenderingDevice.UNIFORM_TYPE_SAMPLER_WITH_TEXTURE
	sampled.binding = 0
	sampled.add_id(_sampler)
	sampled.add_id(input)
	var uniforms: Array[RDUniform] = [sampled]
	if index == 0:
		var output := RDUniform.new()
		output.uniform_type = RenderingDevice.UNIFORM_TYPE_IMAGE
		output.binding = 1
		output.add_id(target)
		uniforms.append(output)
	return _rd.uniform_set_create(uniforms, _shaders[index], 0)


func _dispatch_chain(color: RID, intensity: float) -> bool:
	var bindings: Array[RID] = []
	var framebuffers: Array[RID] = []
	# Build frame-owned resources before commands. Framebuffers always reference
	# current target RIDs, never stale scene layers from before a viewport resize.
	for level in range(LEVEL_COUNT):
		bindings.append(
			_create_binding(0, color if level == 0 else _textures[level - 1], _textures[level])
		)
	for level in range(LEVEL_COUNT - 1, -1, -1):
		var target := _textures[level - 1] if level > 0 else color
		bindings.append(_create_binding(1, _textures[level], target))
		var attachments: Array[RID] = [target]
		framebuffers.append(_rd.framebuffer_create(attachments))
	var valid := true
	for resource in bindings + framebuffers:
		if not resource.is_valid():
			valid = _fail("Could not create bloom uniform set or framebuffer")
			break
	if valid:
		valid = (
			_cache_raster_pipeline(1, framebuffers[0])
			and _cache_raster_pipeline(2, framebuffers[LEVEL_COUNT - 1])
		)
	if valid:
		valid = _record_chain(bindings, framebuffers, intensity)
	# RD defers destruction until GPU work completes, on the owning render thread.
	_release(_rd, bindings)
	_release(_rd, framebuffers)
	return valid


func _record_chain(bindings: Array[RID], framebuffers: Array[RID], intensity: float) -> bool:
	var commands := _rd.compute_list_begin()
	if commands == RenderingDevice.INVALID_ID:
		return _fail("Could not begin bloom compute list")
	for level in range(LEVEL_COUNT):
		_dispatch(commands, 0, bindings[level], _sizes[level], 1.0 if level == 0 else 0.0)
	_rd.compute_list_end()
	# RD tracks the storage-to-attachment and attachment-to-sampled transitions.
	# Every draw samples a distinct lower mip, never its own destination.
	for pass_index in range(LEVEL_COUNT):
		var level := LEVEL_COUNT - 1 - pass_index
		var pipeline := _pipelines[1 if level > 0 else 2]
		var value := _blend_factor(level, intensity) if level > 0 else intensity
		if not _draw_upsample(
			framebuffers[pass_index], pipeline, bindings[LEVEL_COUNT + pass_index], value
		):
			return false
	return true


func _draw_upsample(framebuffer: RID, pipeline: RID, binding: RID, value: float) -> bool:
	# LOAD destination; hardware blend preserves alpha and upstream conversion.
	var draw := _rd.draw_list_begin(framebuffer, RenderingDevice.DRAW_DEFAULT_ALL)
	if draw == RenderingDevice.INVALID_ID:
		return _fail("Could not begin bloom raster draw list")
	_rd.draw_list_bind_render_pipeline(draw, pipeline)
	_rd.draw_list_bind_uniform_set(draw, binding, 0)
	_rd.draw_list_set_blend_constants(draw, Color(value, value, value, value))
	_rd.draw_list_draw(draw, false, 1, 3)
	_rd.draw_list_end()
	return true


func _dispatch(commands: int, index: int, binding: RID, size: Vector2i, value: float) -> void:
	_rd.compute_list_bind_compute_pipeline(commands, _pipelines[index])
	_rd.compute_list_bind_uniform_set(commands, binding, 0)
	var parameters := PackedFloat32Array([value, 0, 0, 0]).to_byte_array()
	_rd.compute_list_set_push_constant(commands, parameters, parameters.size())
	_rd.compute_list_dispatch(commands, ceili(size.x / 8.0), ceili(size.y / 8.0), 1)
	# Make every downsample storage write visible to the next sampled binding.
	_rd.compute_list_add_barrier(commands)


func _blend_factor(level: int, intensity: float) -> float:
	var frequency := float(level) / (LEVEL_COUNT - 1)
	var boost := (1.0 - pow(1.0 - frequency, 1.0 / (1.0 - 0.95))) * 0.7
	var high_pass := 1.0 - clampf((frequency - 1.0) / 1.0, 0.0, 1.0)
	return (intensity + boost) * high_pass


func _fail(message: String) -> bool:
	_failed = true
	push_error("Bloom: " + message)
	return false


func dispose() -> void:
	# Callable retains this resource until cleanup executes on the owning thread.
	RenderingServer.call_on_render_thread(_dispose_on_render_thread)


func _dispose_on_render_thread() -> void:
	if _disposed:
		return
	_disposed = true
	var owned: Array[RID] = []
	owned.assign(_textures + _pipelines + _shaders)
	owned.append(_sampler)
	_release(_rd, owned)
	_textures.clear()
	_shaders.clear()
	_pipelines.clear()
	_sampler = RID()


static func _release(rd: RenderingDevice, rids: Array[RID]) -> void:
	if rd == null:
		return
	for rid in rids:
		if rid.is_valid():
			rd.free_rid(rid)


func _notification(what: int) -> void:
	if what == NOTIFICATION_PREDELETE and _rd != null and not _disposed:
		# PREDELETE cannot call instance methods: the script's base instance is
		# already null. Capture handles directly and bind only the static release.
		var owned: Array[RID] = []
		owned.assign(_textures + _pipelines + _shaders)
		owned.append(_sampler)
		RenderingServer.call_on_render_thread(_release.bind(_rd, owned))
