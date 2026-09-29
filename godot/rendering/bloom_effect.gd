extends CompositorEffect
## Configured Bevy 0.19 OLD_SCHOOL additive bloom, before Godot tonemapping.
## Full-image viewport, uniform scale; 512-height aspect-preserving 8-level chain.
## Separate textures for all neighbourhood reads; own-pixel target addition only.
## Packed R11G11B10 intermediates retain upstream unsigned-float quantization.
## Scene input/output is RGBA16F; alpha is preserved. Finite nonnegative scene
## input is required; final RGB exceeding 65504 can overflow the half framebuffer
## (as upstream can). No NaN sanitization, saturation, stock-glow substitution,
## anamorphic scale, energy-conserving mode, subviewport, or format fallback.

const LEVEL_COUNT := 8
const MIP_HEIGHT := 512
const PACKED_FORMAT := RenderingDevice.DATA_FORMAT_B10G11R11_UFLOAT_PACK32
const TEXTURE_USAGE := (
	RenderingDevice.TEXTURE_USAGE_STORAGE_BIT | RenderingDevice.TEXTURE_USAGE_SAMPLING_BIT
)
const SHADER_PATHS := [
	"res://shaders/bloom_downsample.glsl",
	"res://shaders/bloom_upsample.glsl",
	"res://shaders/bloom_upsample.glsl",
]

var _mutex := Mutex.new()
var _settings := Vector2(0.0, 0.08)
var _rd: RenderingDevice
var _sources: Array[String] = []
var _shaders: Array[RID] = []
var _pipelines: Array[RID] = []
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
		_sources.append(FileAccess.get_file_as_string(path).replace("#[compute]\n", ""))


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
		if not _validate_scene(color, size) or not _dispatch_chain(color, size, settings.y):
			return


func _initialize_device() -> bool:
	_rd = RenderingServer.get_rendering_device()
	if _rd == null:
		return _fail("Requires a render-thread RenderingDevice")
	if not _rd.texture_is_format_supported_for_usage(PACKED_FORMAT, TEXTURE_USAGE):
		return _fail("R11G11B10 packed storage/sampling unsupported; no format fallback")
	var state := RDSamplerState.new()
	state.min_filter = RenderingDevice.SAMPLER_FILTER_LINEAR
	state.mag_filter = RenderingDevice.SAMPLER_FILTER_LINEAR
	state.repeat_u = RenderingDevice.SAMPLER_REPEAT_MODE_CLAMP_TO_EDGE
	state.repeat_v = RenderingDevice.SAMPLER_REPEAT_MODE_CLAMP_TO_EDGE
	_sampler = _rd.sampler_create(state)
	if not _sampler.is_valid():
		return _fail("Could not create bloom linear clamp sampler")
	for index in range(SHADER_PATHS.size()):
		if not _compile_pipeline(index):
			return false
	return true


func _compile_pipeline(index: int) -> bool:
	var code := _sources[index]
	if code.is_empty():
		return _fail("Missing shader " + SHADER_PATHS[index])
	if index == 2:
		code = code.replace("#version 450\n", "#version 450\n#define FINAL_COMPOSITE\n")
	var source := RDShaderSource.new()
	source.source_compute = code
	var spirv := _rd.shader_compile_spirv_from_source(source, false)
	var error := spirv.get_stage_compile_error(RenderingDevice.SHADER_STAGE_COMPUTE)
	if not error.is_empty():
		return _fail("Shader %s: %s" % [SHADER_PATHS[index], error])
	var shader := _rd.shader_create_from_spirv(spirv, "bevy-bloom-%d" % index)
	if not shader.is_valid():
		return _fail("Could not create bloom shader %d" % index)
	_shaders.append(shader)
	var pipeline := _rd.compute_pipeline_create(shader)
	if not pipeline.is_valid():
		return _fail("Could not create bloom pipeline %d" % index)
	_pipelines.append(pipeline)
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
	if (format.usage_bits & TEXTURE_USAGE) != TEXTURE_USAGE:
		return _fail("Scene layer requires storage and sampling usage")
	return true


func _create_binding(index: int, input: RID, target: RID) -> RID:
	var sampled := RDUniform.new()
	sampled.uniform_type = RenderingDevice.UNIFORM_TYPE_SAMPLER_WITH_TEXTURE
	sampled.binding = 0
	sampled.add_id(_sampler)
	sampled.add_id(input)
	var output := RDUniform.new()
	output.uniform_type = RenderingDevice.UNIFORM_TYPE_IMAGE
	output.binding = 1
	output.add_id(target)
	var uniforms: Array[RDUniform] = [sampled, output]
	return _rd.uniform_set_create(uniforms, _shaders[index], 0)


func _dispatch_chain(color: RID, size: Vector2i, intensity: float) -> bool:
	var bindings: Array[RID] = []
	# Build every binding before beginning commands, so failure never leaves an
	# open compute list or a partially bloomed scene. Scratch contents are overwritten.
	for level in range(LEVEL_COUNT):
		bindings.append(
			_create_binding(0, color if level == 0 else _textures[level - 1], _textures[level])
		)
	for level in range(LEVEL_COUNT - 1, 0, -1):
		bindings.append(_create_binding(1, _textures[level], _textures[level - 1]))
	bindings.append(_create_binding(2, _textures[0], color))
	for binding in bindings:
		if not binding.is_valid():
			_release(_rd, bindings)
			return _fail("Could not create bloom uniform set")
	var commands := _rd.compute_list_begin()
	for level in range(LEVEL_COUNT):
		_dispatch(commands, 0, bindings[level], _sizes[level], 1.0 if level == 0 else 0.0)
	for level in range(LEVEL_COUNT - 1, 0, -1):
		var binding_index := LEVEL_COUNT + (LEVEL_COUNT - 1 - level)
		_dispatch(
			commands, 1, bindings[binding_index], _sizes[level - 1], _blend_factor(level, intensity)
		)
	_dispatch(commands, 2, bindings[bindings.size() - 1], size, intensity)
	_rd.compute_list_end()
	# RD defers destruction until GPU work completes. No retained binding refers
	# to a scene layer that may have been replaced by a viewport resize.
	_release(_rd, bindings)
	return true


func _dispatch(commands: int, index: int, binding: RID, size: Vector2i, value: float) -> void:
	_rd.compute_list_bind_compute_pipeline(commands, _pipelines[index])
	_rd.compute_list_bind_uniform_set(commands, binding, 0)
	var parameters := PackedFloat32Array([value, 0, 0, 0]).to_byte_array()
	_rd.compute_list_set_push_constant(commands, parameters, parameters.size())
	_rd.compute_list_dispatch(commands, ceili(size.x / 8.0), ceili(size.y / 8.0), 1)
	# Make every previous storage write visible to the next sampled/read-write
	# binding, including between consecutive stereo views using the same scratch.
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
