extends CompositorEffect
## M2 particle pools composited in authored (gamma) space, as WebWowViewerCpp blends
## its particles into an R8G8B8A8_UNORM target (GFrameBufferVLK.cpp:21) with the M2
## GL factors (GDeviceGL20.cpp:21-35). After Godot's transparent pass and MSAA resolve:
## encode the scene colour to sRGB in place, draw every published pool far to near
## (depth tested against the scene depth, no depth writes), decode back to linear.
## Pools come from particles.rs (`particle_composite::publish`) each frame; with none
## published the callback records nothing.
## Named gap PARTICLES_COMPOSITE_AFTER_ALL_TRANSPARENT_GEOMETRY: pools draw after all
## transparent geometry instead of sorted among it.

const TEMPLATE_PATH := "res://shaders/particle_composite.glsl.in"
const FOG_PATH := "res://shaders/retail_fog.gdshaderinc"
const GAMMA_PATH := "res://shaders/particle_gamma.glsl"
const SCENE_BYTES := 256
const FLOATS_PER_INSTANCE := 20
# ParticleBlendingModeToEGxBlendEnum, then the EGxBlend GL factors (src, dst); 0/1
# draw unblended and rely on the alpha test.
const NO_BLEND := -1
const BLEND_FACTORS := {
	0: [NO_BLEND, NO_BLEND],
	1: [NO_BLEND, NO_BLEND],
	2: [RenderingDevice.BLEND_FACTOR_SRC_ALPHA, RenderingDevice.BLEND_FACTOR_ONE_MINUS_SRC_ALPHA],
	10: [RenderingDevice.BLEND_FACTOR_ONE, RenderingDevice.BLEND_FACTOR_ONE],
	3: [RenderingDevice.BLEND_FACTOR_SRC_ALPHA, RenderingDevice.BLEND_FACTOR_ONE],
	4: [RenderingDevice.BLEND_FACTOR_DST_COLOR, RenderingDevice.BLEND_FACTOR_ZERO],
	5: [RenderingDevice.BLEND_FACTOR_DST_COLOR, RenderingDevice.BLEND_FACTOR_SRC_COLOR],
	13: [RenderingDevice.BLEND_FACTOR_ONE, RenderingDevice.BLEND_FACTOR_ONE_MINUS_SRC_ALPHA],
}

var _mutex := Mutex.new()
var _draws: Array = []
var _fog := PackedFloat32Array()
var _rd: RenderingDevice
var _vertex_source := ""
var _fragment_source := ""
var _gamma_source := ""
var _draw_shader := RID()
var _gamma_shader := RID()
var _gamma_pipeline := RID()
var _pipelines := {}
var _pipeline_format := -1
var _scene_buffer := RID()
var _linear_sampler := RID()
var _depth_sampler := RID()
var _instance_buffers: Array[RID] = []
var _instance_sizes: Array[int] = []
var _disposed := false
var _failed := false


func _init() -> void:
	effect_callback_type = EFFECT_CALLBACK_TYPE_POST_TRANSPARENT
	# Never perform filesystem IO in the callback.
	var template := FileAccess.get_file_as_string(TEMPLATE_PATH)
	var fog := ""
	for line in FileAccess.get_file_as_string(FOG_PATH).split("\n"):
		# Its uniforms are the Scene block members and push constants of the template.
		if not line.begins_with("uniform ") and not line.begins_with("global uniform "):
			fog += line + "\n"
	var stages := template.split("#[fragment]\n")
	if stages.size() == 2:
		_vertex_source = stages[0].substr(stages[0].find("#[vertex]\n") + "#[vertex]\n".length())
		_fragment_source = stages[1].replace("#RETAIL_FOG", fog)
	_gamma_source = FileAccess.get_file_as_string(GAMMA_PATH).replace("#[compute]\n", "")


## This frame's pools of the camera's world: dictionaries of `instances` (float32
## bytes, FLOATS_PER_INSTANCE per quad), `count`, `textures` (three RD texture RIDs),
## `params` (push-constant bytes), `gx_blend` and `center`; `fog` is the scene fog
## block of the Scene uniform.
func publish(draws: Array, fog: PackedFloat32Array) -> void:
	_mutex.lock()
	_draws = draws
	_fog = fog
	_mutex.unlock()


## First among `camera`'s effects, so TAA and Bloom see the composited particles as
## they saw transparent-pass particles.
func attach(camera: Camera3D) -> void:
	var compositor := camera.compositor
	if compositor != null and not compositor.compositor_effects.is_empty() and compositor.compositor_effects[0] == self:
		return
	# Keep existing effects shared, but isolate this camera's effect list.
	compositor = Compositor.new() if compositor == null else compositor.duplicate() as Compositor
	var effects := compositor.compositor_effects
	effects.erase(self)
	effects.push_front(self)
	compositor.compositor_effects = effects
	camera.compositor = compositor


func _render_callback(callback_type: int, render_data: RenderData) -> void:
	if _disposed or _failed:
		return
	_mutex.lock()
	var draws := _draws
	var fog := _fog
	_mutex.unlock()
	if draws.is_empty():
		return
	if callback_type != EFFECT_CALLBACK_TYPE_POST_TRANSPARENT:
		_fail("Expected POST_TRANSPARENT callback")
		return
	var buffers := render_data.get_render_scene_buffers() as RenderSceneBuffersRD
	var scene := render_data.get_render_scene_data() as RenderSceneDataRD
	if buffers == null or scene == null:
		_fail("Callback has no RenderSceneBuffersRD or RenderSceneDataRD")
		return
	if buffers.get_view_count() != 1:
		_fail("Expected one view, got %d" % buffers.get_view_count())
		return
	var size := buffers.get_internal_size()
	if size.x <= 0 or size.y <= 0:
		return
	if _rd == null and not _initialize_device():
		return
	var color := buffers.get_color_layer(0)
	var depth := buffers.get_depth_layer(0)
	if not _validate_color(color, size):
		return
	var attachments: Array[RID] = [color]
	var framebuffer := _rd.framebuffer_create(attachments)
	if not framebuffer.is_valid():
		_fail("Could not create the scene colour framebuffer")
		return
	if _cache_pipelines(framebuffer):
		_composite(draws, fog, scene, color, depth, framebuffer, size)
	# RD defers destruction until GPU work completes.
	_rd.free_rid(framebuffer)


func _initialize_device() -> bool:
	_rd = RenderingServer.get_rendering_device()
	if _rd == null:
		return _fail("Requires a render-thread RenderingDevice")
	if _vertex_source.is_empty() or _fragment_source.is_empty() or _gamma_source.is_empty():
		return _fail("Missing particle composite shader source")
	var draw_source := RDShaderSource.new()
	draw_source.source_vertex = _vertex_source
	draw_source.source_fragment = _fragment_source
	_draw_shader = _compile(draw_source, [RenderingDevice.SHADER_STAGE_VERTEX, RenderingDevice.SHADER_STAGE_FRAGMENT], "particle-composite")
	var gamma_source := RDShaderSource.new()
	gamma_source.source_compute = _gamma_source
	_gamma_shader = _compile(gamma_source, [RenderingDevice.SHADER_STAGE_COMPUTE], "particle-gamma")
	if not _draw_shader.is_valid() or not _gamma_shader.is_valid():
		return false
	_gamma_pipeline = _rd.compute_pipeline_create(_gamma_shader)
	_scene_buffer = _rd.uniform_buffer_create(SCENE_BYTES)
	# The particle shader samples its textures filter_linear_mipmap, repeat_enable.
	var linear := RDSamplerState.new()
	linear.min_filter = RenderingDevice.SAMPLER_FILTER_LINEAR
	linear.mag_filter = RenderingDevice.SAMPLER_FILTER_LINEAR
	linear.mip_filter = RenderingDevice.SAMPLER_FILTER_LINEAR
	linear.repeat_u = RenderingDevice.SAMPLER_REPEAT_MODE_REPEAT
	linear.repeat_v = RenderingDevice.SAMPLER_REPEAT_MODE_REPEAT
	_linear_sampler = _rd.sampler_create(linear)
	_depth_sampler = _rd.sampler_create(RDSamplerState.new())
	for resource in [_gamma_pipeline, _scene_buffer, _linear_sampler, _depth_sampler]:
		if not resource.is_valid():
			return _fail("Could not create particle composite device resources")
	return true


func _compile(source: RDShaderSource, stages: Array, name: String) -> RID:
	var spirv := _rd.shader_compile_spirv_from_source(source, false)
	if spirv == null:
		_fail("Could not compile %s" % name)
		return RID()
	for stage in stages:
		var error := spirv.get_stage_compile_error(stage)
		if not error.is_empty():
			_fail("%s stage %d: %s" % [name, stage, error])
			return RID()
	var shader := _rd.shader_create_from_spirv(spirv, name)
	if not shader.is_valid():
		_fail("Could not create %s" % name)
	return shader


func _validate_color(color: RID, size: Vector2i) -> bool:
	if not color.is_valid() or not _rd.texture_is_valid(color):
		return _fail("Invalid scene color layer")
	var format := _rd.texture_get_format(color)
	if format.format != RenderingDevice.DATA_FORMAT_R16G16B16A16_SFLOAT:
		return _fail("Expected scene RGBA16F, got format %d" % format.format)
	if Vector2i(format.width, format.height) != size:
		return _fail("Scene color dimensions differ from internal render size")
	var required := RenderingDevice.TEXTURE_USAGE_STORAGE_BIT | RenderingDevice.TEXTURE_USAGE_COLOR_ATTACHMENT_BIT
	if (format.usage_bits & required) != required:
		return _fail("Scene color layer requires storage and color attachment usage")
	return true


func _cache_pipelines(framebuffer: RID) -> bool:
	var format := _rd.framebuffer_get_format(framebuffer)
	if format == _pipeline_format:
		return true
	_release(_rd, _pipelines.values())
	_pipelines.clear()
	for gx_blend in BLEND_FACTORS:
		var factors: Array = BLEND_FACTORS[gx_blend]
		var attachment := RDPipelineColorBlendStateAttachment.new()
		attachment.enable_blend = factors[0] != NO_BLEND
		if attachment.enable_blend:
			attachment.src_color_blend_factor = factors[0]
			attachment.dst_color_blend_factor = factors[1]
			attachment.color_blend_op = RenderingDevice.BLEND_OP_ADD
		# Scene alpha is preserved.
		attachment.write_a = false
		var blend := RDPipelineColorBlendState.new()
		var attachments: Array[RDPipelineColorBlendStateAttachment] = [attachment]
		blend.attachments = attachments
		var pipeline := _rd.render_pipeline_create(
			_draw_shader,
			format,
			-1,
			RenderingDevice.RENDER_PRIMITIVE_TRIANGLE_STRIPS,
			RDPipelineRasterizationState.new(),
			RDPipelineMultisampleState.new(),
			RDPipelineDepthStencilState.new(),
			blend
		)
		if not pipeline.is_valid():
			return _fail("Could not create particle pipeline for EGxBlend %d" % gx_blend)
		_pipelines[gx_blend] = pipeline
	_pipeline_format = format
	return true


func _composite(draws: Array, fog: PackedFloat32Array, scene: RenderSceneDataRD, color: RID, depth: RID, framebuffer: RID, size: Vector2i) -> void:
	var camera := scene.get_cam_transform()
	var view_projection := scene.get_view_projection(0) * Projection(camera.affine_inverse())
	var scene_floats := PackedFloat32Array()
	for column in [view_projection.x, view_projection.y, view_projection.z, view_projection.w]:
		scene_floats.append_array([column.x, column.y, column.z, column.w])
	scene_floats.append_array([camera.origin.x, camera.origin.y, camera.origin.z, 0.0])
	scene_floats.append_array(fog)
	var scene_bytes := scene_floats.to_byte_array()
	if scene_bytes.size() != SCENE_BYTES:
		_fail("Scene block is %d bytes, expected %d" % [scene_bytes.size(), SCENE_BYTES])
		return
	_rd.buffer_update(_scene_buffer, 0, SCENE_BYTES, scene_bytes)
	# Godot sorted each pool among transparents by its centre's distance: far first.
	var ordered := draws.duplicate()
	ordered.sort_custom(func(a, b): return camera.origin.distance_squared_to(a.center) > camera.origin.distance_squared_to(b.center))
	for index in ordered.size():
		if not _upload(index, ordered[index].instances):
			return
	var scene_set := _cached_set(_draw_shader, 0, [
		_uniform(RenderingDevice.UNIFORM_TYPE_UNIFORM_BUFFER, 0, [_scene_buffer]),
		_uniform(RenderingDevice.UNIFORM_TYPE_SAMPLER_WITH_TEXTURE, 1, [_depth_sampler, depth]),
	])
	_rd.capture_timestamp("Particle gamma encode")
	_convert(color, size, 1)
	_rd.capture_timestamp("Particle composite draws")
	var list := _rd.draw_list_begin(framebuffer, RenderingDevice.DRAW_DEFAULT_ALL)
	for index in ordered.size():
		var draw: Dictionary = ordered[index]
		var textures: Array = draw.textures
		var pool_set := _cached_set(_draw_shader, 1, [
			_uniform(RenderingDevice.UNIFORM_TYPE_STORAGE_BUFFER, 0, [_instance_buffers[index]]),
			_uniform(RenderingDevice.UNIFORM_TYPE_SAMPLER_WITH_TEXTURE, 1, [_linear_sampler, textures[0]]),
			_uniform(RenderingDevice.UNIFORM_TYPE_SAMPLER_WITH_TEXTURE, 2, [_linear_sampler, textures[1]]),
			_uniform(RenderingDevice.UNIFORM_TYPE_SAMPLER_WITH_TEXTURE, 3, [_linear_sampler, textures[2]]),
		])
		_rd.draw_list_bind_render_pipeline(list, _pipelines[draw.gx_blend])
		_rd.draw_list_bind_uniform_set(list, scene_set, 0)
		_rd.draw_list_bind_uniform_set(list, pool_set, 1)
		var params: PackedByteArray = draw.params
		_rd.draw_list_set_push_constant(list, params, params.size())
		_rd.draw_list_draw(list, false, draw.count, 4)
	_rd.draw_list_end()
	_rd.capture_timestamp("Particle gamma decode")
	_convert(color, size, 0)
	_rd.capture_timestamp("Particle composite end")


func _upload(index: int, instances: PackedByteArray) -> bool:
	if index == _instance_buffers.size():
		_instance_buffers.append(RID())
		_instance_sizes.append(0)
	if _instance_sizes[index] < instances.size():
		if _instance_buffers[index].is_valid():
			_rd.free_rid(_instance_buffers[index])
		var bytes := nearest_po2(instances.size())
		_instance_buffers[index] = _rd.storage_buffer_create(bytes)
		_instance_sizes[index] = bytes
		if not _instance_buffers[index].is_valid():
			return _fail("Could not allocate a %d-byte particle instance buffer" % bytes)
	_rd.buffer_update(_instance_buffers[index], 0, instances.size(), instances)
	return true


func _convert(color: RID, size: Vector2i, encode: int) -> void:
	var image_set := _cached_set(_gamma_shader, 0, [
		_uniform(RenderingDevice.UNIFORM_TYPE_IMAGE, 0, [color]),
	])
	var commands := _rd.compute_list_begin()
	_rd.compute_list_bind_compute_pipeline(commands, _gamma_pipeline)
	_rd.compute_list_bind_uniform_set(commands, image_set, 0)
	var params := PackedInt32Array([size.x, size.y, encode, 0]).to_byte_array()
	_rd.compute_list_set_push_constant(commands, params, params.size())
	_rd.compute_list_dispatch(commands, ceili(size.x / 8.0), ceili(size.y / 8.0), 1)
	_rd.compute_list_end()


static func _cached_set(shader: RID, set_index: int, uniforms: Array) -> RID:
	var typed: Array[RDUniform] = []
	typed.assign(uniforms)
	return UniformSetCacheRD.get_cache(shader, set_index, typed)


static func _uniform(type: int, binding: int, ids: Array) -> RDUniform:
	var uniform := RDUniform.new()
	uniform.uniform_type = type
	uniform.binding = binding
	for id in ids:
		uniform.add_id(id)
	return uniform


func _fail(message: String) -> bool:
	_failed = true
	push_error("Particle composite: " + message)
	return false


func dispose() -> void:
	RenderingServer.call_on_render_thread(_dispose_on_render_thread)


func _dispose_on_render_thread() -> void:
	if _disposed:
		return
	_disposed = true
	var owned: Array[RID] = []
	owned.assign(_pipelines.values() + _instance_buffers)
	owned.append_array([_gamma_pipeline, _draw_shader, _gamma_shader, _scene_buffer, _linear_sampler, _depth_sampler])
	_release(_rd, owned)
	_pipelines.clear()
	_instance_buffers.clear()


static func _release(rd: RenderingDevice, rids: Array) -> void:
	if rd == null:
		return
	for rid in rids:
		if rid.is_valid():
			rd.free_rid(rid)


func _notification(what: int) -> void:
	if what == NOTIFICATION_PREDELETE and _rd != null and not _disposed:
		# PREDELETE cannot call instance methods; bind the static release.
		var owned: Array[RID] = []
		owned.assign(_pipelines.values() + _instance_buffers)
		owned.append_array([_gamma_pipeline, _draw_shader, _gamma_shader, _scene_buffer, _linear_sampler, _depth_sampler])
		RenderingServer.call_on_render_thread(_release.bind(_rd, owned))
