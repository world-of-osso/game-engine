extends SceneTree
## Standalone differential harness; main owns Vulkan runs. No native extension.
## Godot --path godot/tests --script bloom_compute_pixels.gd
## Production compute vs independently translated Bevy 0.19.0 raster, each
## sampling its own identically initialized packed pyramid. Every down/up/final
## output and alpha checked. CPU ideal bilinear is not a spatial GPU oracle:
## actual hardware filtering differs even when raster/compute taps agree.
## CPU reference self-tests and constant-field golden math remain independent.
## Existing one packed/half ULP limits unchanged; no driver rounding gate.
## This is NOT compositor lifecycle, rendered-UI, or production-fix proof.

const Reference = preload("bloom_reference.gd")
const ABS_TOLERANCE := 0.00008
const REL_TOLERANCE := 0.00003
const LEGACY_VERTEX := """#version 450
// Bevy fullscreen.wgsl top-left UV convention, adapted to Vulkan clip Y.
layout(location=0) out vec2 output_uv;
void main() {
    vec2 uv = vec2(float(gl_VertexIndex >> 1), float(gl_VertexIndex & 1)) * 2.0;
    gl_Position = vec4(uv * 2.0 - 1.0, 0.0, 1.0);
    output_uv = uv;
}
"""


class ErrorObserver:
	extends Logger
	var mutex := Mutex.new()
	var errors := 0

	func _log_error(
		_function: String,
		_file: String,
		_line: int,
		_code: String,
		_rationale: String,
		_editor_notify: bool,
		error_type: int,
		_script_backtraces: Array[ScriptBacktrace]
	) -> void:
		if error_type != ERROR_TYPE_WARNING:
			mutex.lock()
			errors += 1
			mutex.unlock()

	func count() -> int:
		mutex.lock()
		var result := errors
		mutex.unlock()
		return result


var observer := ErrorObserver.new()
var rd: RenderingDevice
var sampler := RID()
var shaders: Array[RID] = []
var pipelines: Array[RID] = []
var raster_shaders: Array[RID] = []
var textures: Array[RID] = []
var failures := 0
var comparisons := 0
var finished := false


func _initialize() -> void:
	OS.add_logger(observer)
	call_deferred("run_test")


func run_test() -> void:
	if RenderingServer.get_current_rendering_driver_name().to_lower() != "vulkan":
		push_error("Bloom differential requires Vulkan, not dummy/headless rendering")
		finish()
		return
	rd = RenderingServer.create_local_rendering_device()
	if rd == null:
		push_error("Bloom differential requires a local RenderingDevice")
		finish()
		return
	var state := RDSamplerState.new()
	state.min_filter = RenderingDevice.SAMPLER_FILTER_LINEAR
	state.mag_filter = RenderingDevice.SAMPLER_FILTER_LINEAR
	state.repeat_u = RenderingDevice.SAMPLER_REPEAT_MODE_CLAMP_TO_EDGE
	state.repeat_v = RenderingDevice.SAMPLER_REPEAT_MODE_CLAMP_TO_EDGE
	sampler = rd.sampler_create(state)
	if not sampler.is_valid():
		push_error("Bloom LINEAR CLAMP_TO_EDGE sampler creation failed")
		finish()
		return
	var directory: String = get_script().resource_path.get_base_dir()
	var production := directory.path_join("../shaders")
	if (
		not compile_compute(production.path_join("bloom_downsample.glsl"), "")
		or not compile_compute(production.path_join("bloom_upsample.glsl"), "")
		or not compile_compute(
			production.path_join("bloom_upsample.glsl"), "#define FINAL_COMPOSITE\n"
		)
		or not compile_raster(directory.path_join("bloom_legacy_raster.glsl"))
	):
		finish()
		return
	var spatial := Image.create(17, 11, false, Image.FORMAT_RGBAH)
	spatial.fill(Color(0.15, 0.3, 0.55, 0.375))
	spatial.set_pixel(0, 0, Color(16, 4, 1, 0.25))
	spatial.set_pixel(8, 5, Color(8, 2, 4, 0.5))
	spatial.set_pixel(16, 10, Color(1, 6, 2, 0.75))
	var black := Image.create(9, 13, false, Image.FORMAT_RGBAH)
	black.fill(Color(0, 0, 0, 0.625))
	# Both bounded original corpus and actual OLD_SCHOOL 512-height/8-level corpus.
	for height in [32, 512]:
		test_fixture(spatial, height, 0.08, "edge/impulse-colour-%d" % height)
		test_fixture(spatial, height, 1.0, "intensity-one-%d" % height)
		test_fixture(black, height, 0.08, "finite-black-%d" % height)
		if observer.count() != 0:
			finish()
			return
	var constant := Image.create(19, 11, false, Image.FORMAT_RGBAH)
	constant.fill(Color(2, 1, 0.75, 0.375))
	test_fixture(constant, 512, 0.08, "configured-eight-levels", true)
	finish()


func compile_source(source: RDShaderSource, stages: Array[int]) -> RID:
	var spirv := rd.shader_compile_spirv_from_source(source, false)
	if spirv == null:
		push_error("Bloom SPIR-V compilation returned null")
		return RID()
	for stage in stages:
		var error := spirv.get_stage_compile_error(stage)
		if not error.is_empty():
			push_error("Bloom stage%d: %s" % [stage, error])
			return RID()
	var shader := rd.shader_create_from_spirv(spirv)
	if not shader.is_valid():
		push_error("Bloom shader creation failed")
	return shader


func compile_compute(path: String, defines: String) -> bool:
	var code := FileAccess.get_file_as_string(path).replace("#[compute]\n", "")
	if code.is_empty():
		push_error("Missing production shader: " + path)
		return false
	var source := RDShaderSource.new()
	source.source_compute = code.replace("#version 450\n", "#version 450\n" + defines)
	var shader := compile_source(source, [RenderingDevice.SHADER_STAGE_COMPUTE])
	shaders.append(shader)
	if not shader.is_valid():
		return false
	var pipeline := rd.compute_pipeline_create(shader)
	pipelines.append(pipeline)
	if not pipeline.is_valid() or not rd.compute_pipeline_is_valid(pipeline):
		push_error("Bloom compute pipeline creation failed")
		return false
	return true


func compile_raster(path: String) -> bool:
	var code := FileAccess.get_file_as_string(path)
	if code.is_empty():
		push_error("Missing independent Bevy raster shader: " + path)
		return false
	for defines in ["#define FIRST_DOWNSAMPLE\n", "", "#define UPSAMPLE\n"]:
		var source := RDShaderSource.new()
		source.source_vertex = LEGACY_VERTEX
		source.source_fragment = code.replace("#version 450\n", "#version 450\n" + defines)
		var shader := compile_source(
			source, [RenderingDevice.SHADER_STAGE_VERTEX, RenderingDevice.SHADER_STAGE_FRAGMENT]
		)
		raster_shaders.append(shader)
		if not shader.is_valid():
			return false
	return true


func create_texture(image: Image) -> RID:
	var format := RDTextureFormat.new()
	format.width = image.get_width()
	format.height = image.get_height()
	format.format = (
		RenderingDevice.DATA_FORMAT_R16G16B16A16_SFLOAT
		if image.get_format() == Image.FORMAT_RGBAH
		else RenderingDevice.DATA_FORMAT_B10G11R11_UFLOAT_PACK32
	)
	format.usage_bits = (
		RenderingDevice.TEXTURE_USAGE_SAMPLING_BIT
		| RenderingDevice.TEXTURE_USAGE_STORAGE_BIT
		| RenderingDevice.TEXTURE_USAGE_COLOR_ATTACHMENT_BIT
		| RenderingDevice.TEXTURE_USAGE_CAN_COPY_FROM_BIT
	)
	if not rd.texture_is_format_supported_for_usage(format.format, format.usage_bits):
		push_error(
			"Bloom format%d storage/sampling/attachment/readback unsupported" % format.format
		)
		return RID()
	# Both paths receive exactly the same bytes, including every packed level.
	var bytes := image.get_data()
	if image.get_format() != Image.FORMAT_RGBAH:
		bytes = PackedByteArray()
		bytes.resize(format.width * format.height * 4)
		bytes.fill(0)
	var initial: Array[PackedByteArray] = [bytes]
	var texture := rd.texture_create(format, RDTextureView.new(), initial)
	textures.append(texture)
	if not texture.is_valid():
		push_error("Bloom texture creation failed at %s" % image.get_size())
	return texture


func sampled_uniform(input: RID) -> RDUniform:
	var sampled := RDUniform.new()
	sampled.uniform_type = RenderingDevice.UNIFORM_TYPE_SAMPLER_WITH_TEXTURE
	sampled.binding = 0
	sampled.add_id(sampler)
	sampled.add_id(input)
	return sampled


func dispatch(index: int, input: RID, target: RID, size: Vector2i, value: float) -> bool:
	var output := RDUniform.new()
	output.uniform_type = RenderingDevice.UNIFORM_TYPE_IMAGE
	output.binding = 1
	output.add_id(target)
	var uniforms: Array[RDUniform] = [sampled_uniform(input), output]
	var binding := rd.uniform_set_create(uniforms, shaders[index], 0)
	if not binding.is_valid():
		push_error("Bloom compute uniform set creation failed")
		return false
	var commands := rd.compute_list_begin()
	if commands == RenderingDevice.INVALID_ID:
		rd.free_rid(binding)
		push_error("Bloom compute list begin failed")
		return false
	rd.compute_list_bind_compute_pipeline(commands, pipelines[index])
	rd.compute_list_bind_uniform_set(commands, binding, 0)
	var parameters := PackedFloat32Array([value, 0, 0, 0]).to_byte_array()
	rd.compute_list_set_push_constant(commands, parameters, parameters.size())
	rd.compute_list_dispatch(commands, ceili(size.x / 8.0), ceili(size.y / 8.0), 1)
	rd.compute_list_end()
	rd.submit()
	rd.sync()
	rd.free_rid(binding)
	return observer.count() == 0


func raster_pipeline(shader: RID, framebuffer: RID, additive: bool) -> RID:
	var attachment := RDPipelineColorBlendStateAttachment.new()
	if additive:
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
	return rd.render_pipeline_create(
		shader,
		rd.framebuffer_get_format(framebuffer),
		-1,
		RenderingDevice.RENDER_PRIMITIVE_TRIANGLES,
		RDPipelineRasterizationState.new(),
		RDPipelineMultisampleState.new(),
		RDPipelineDepthStencilState.new(),
		blend,
		RenderingDevice.DYNAMIC_STATE_BLEND_CONSTANTS if additive else 0
	)


func draw_raster(index: int, input: RID, target: RID, value: float) -> bool:
	var attachments: Array[RID] = [target]
	var framebuffer := rd.framebuffer_create(attachments)
	if not framebuffer.is_valid():
		push_error("Bloom raster framebuffer creation failed")
		return false
	var pipeline := raster_pipeline(raster_shaders[index], framebuffer, index == 2)
	var uniforms: Array[RDUniform] = [sampled_uniform(input)]
	var binding := rd.uniform_set_create(uniforms, raster_shaders[index], 0)
	var valid := pipeline.is_valid() and binding.is_valid()
	if valid:
		valid = rd.render_pipeline_is_valid(pipeline)
	if valid:
		valid = record_raster(framebuffer, pipeline, binding, index == 2, value)
	else:
		push_error("Bloom raster pipeline/uniform creation failed")
	# Dependents before dependencies; all paths (including invalid draw) tear down.
	if binding.is_valid():
		rd.free_rid(binding)
	if pipeline.is_valid():
		rd.free_rid(pipeline)
	rd.free_rid(framebuffer)
	return valid and observer.count() == 0


func record_raster(
	framebuffer: RID, pipeline: RID, binding: RID, additive: bool, value: float
) -> bool:
	# LOAD existing destination for additive recursion/final; never clear it.
	var draw := rd.draw_list_begin(framebuffer, RenderingDevice.DRAW_DEFAULT_ALL)
	if draw == RenderingDevice.INVALID_ID:
		push_error("Bloom raster draw list begin failed")
		return false
	rd.draw_list_bind_render_pipeline(draw, pipeline)
	rd.draw_list_bind_uniform_set(draw, binding, 0)
	# BloomUniforms from downsampling_pipeline.rs, full viewport, scale=(1,1).
	var knee := Reference.THRESHOLD * Reference.SOFTNESS
	var parameters := (
		PackedFloat32Array(
			[
				Reference.THRESHOLD,
				Reference.THRESHOLD - knee,
				2.0 * knee,
				0.25 / (knee + 0.00001),
				0,
				0,
				1,
				1,
				1,
				1,
				1,
				0
			]
		)
		. to_byte_array()
	)
	rd.draw_list_set_push_constant(draw, parameters, parameters.size())
	if additive:
		rd.draw_list_set_blend_constants(draw, Color(value, value, value, value))
	rd.draw_list_draw(draw, false, 1, 3)
	rd.draw_list_end()
	rd.submit()
	rd.sync()
	return observer.count() == 0


func test_fixture(
	original: Image, max_height: int, intensity: float, label: String, constant := false
) -> void:
	run_fixture(original, max_height, intensity, label, constant)
	release_textures()


func run_fixture(
	original: Image, max_height: int, intensity: float, label: String, constant: bool
) -> void:
	var sizes: Array[Vector2i] = Reference.mip_sizes(original.get_size(), max_height)
	var scene := create_texture(original)
	var raster_scene := create_texture(original)
	if not scene.is_valid() or not raster_scene.is_valid():
		return
	var pyramid: Array[RID] = []
	var raster_pyramid: Array[RID] = []
	var golden_levels: Array[Image] = []
	var cpu: Image
	if constant:
		cpu = Image.create(1, 1, false, Image.FORMAT_RGBAF)
		cpu.fill(original.get_pixel(0, 0))
	for level in range(sizes.size()):
		var blank := Image.create(sizes[level].x, sizes[level].y, false, Image.FORMAT_RGBAF)
		var target := create_texture(blank)
		var raster_target := create_texture(blank)
		if not target.is_valid() or not raster_target.is_valid():
			return
		var source := scene if level == 0 else pyramid[level - 1]
		var raster_source := raster_scene if level == 0 else raster_pyramid[level - 1]
		if (
			not dispatch(0, source, target, sizes[level], 1.0 if level == 0 else 0.0)
			or not draw_raster(0 if level == 0 else 1, raster_source, raster_target, 0.0)
		):
			return
		var actual := read_texture(target, sizes[level])
		compare(actual, read_texture(raster_target, sizes[level]), "%s down%d" % [label, level])
		if constant:
			cpu = quantize_packed(Reference.downsample(cpu, Vector2i.ONE, level == 0))
			golden_levels.append(cpu)
			compare(actual, cpu, "%s golden down%d" % [label, level], true)
		pyramid.append(target)
		raster_pyramid.append(raster_target)
	var last := sizes.size() - 1
	for level in range(last, 0, -1):
		var blend: float = Reference.blend_factor(level, last, intensity)
		if (
			not dispatch(1, pyramid[level], pyramid[level - 1], sizes[level - 1], blend)
			or not draw_raster(2, raster_pyramid[level], raster_pyramid[level - 1], blend)
		):
			return
		var actual := read_texture(pyramid[level - 1], sizes[level - 1])
		compare(
			actual,
			read_texture(raster_pyramid[level - 1], sizes[level - 1]),
			"%s up%d" % [label, level]
		)
		if constant:
			cpu = quantize_packed(Reference.upsample_add(cpu, golden_levels[level - 1], blend))
			compare(actual, cpu, "%s golden up%d" % [label, level], true)
	if (
		not dispatch(2, pyramid[0], scene, original.get_size(), intensity)
		or not draw_raster(2, raster_pyramid[0], raster_scene, intensity)
	):
		return
	var actual := read_texture(scene, original.get_size(), true)
	compare(actual, read_texture(raster_scene, original.get_size(), true), label + " final")
	# Independent alpha expectation, not merely agreement between two paths.
	for y in range(original.get_height()):
		for x in range(original.get_width()):
			comparisons += 1
			if actual == null or actual.get_pixel(x, y).a != original.get_pixel(x, y).a:
				failures += 1
				push_error("%s final alpha changed at (%d,%d)" % [label, x, y])
				return
	if constant:
		var destination := Image.create(1, 1, false, Image.FORMAT_RGBAF)
		destination.fill(original.get_pixel(0, 0))
		cpu = Reference.upsample_add(cpu, destination, intensity)
		cpu.convert(Image.FORMAT_RGBAH)
		compare(actual, cpu, label + " golden final", true)


func read_texture(texture: RID, size: Vector2i, half := false) -> Image:
	var bytes := rd.texture_get_data(texture, 0)
	if bytes.size() != size.x * size.y * (8 if half else 4):
		push_error("Bloom readback byte count mismatch at %s" % size)
		return null
	if half:
		return Image.create_from_data(size.x, size.y, false, Image.FORMAT_RGBAH, bytes)
	var image := Image.create(size.x, size.y, false, Image.FORMAT_RGBAF)
	for y in range(size.y):
		for x in range(size.x):
			var packed := bytes.decode_u32((y * size.x + x) * 4)
			image.set_pixel(
				x,
				y,
				Color(
					decode_unsigned_float(packed & 0x7ff, 6),
					decode_unsigned_float((packed >> 11) & 0x7ff, 6),
					decode_unsigned_float((packed >> 22) & 0x3ff, 5),
					1.0
				)
			)
	return image


func decode_unsigned_float(bits: int, mantissa_bits: int) -> float:
	var exponent := bits >> mantissa_bits
	var fraction := float(bits & ((1 << mantissa_bits) - 1)) / (1 << mantissa_bits)
	if exponent == 0:
		return fraction * pow(2.0, -14)
	if exponent == 31:
		return INF if fraction == 0.0 else NAN
	return (1.0 + fraction) * pow(2.0, exponent - 15)


func packed_ulp(value: float, mantissa_bits: int) -> float:
	if value < pow(2.0, -14):
		return pow(2.0, -14 - mantissa_bits)
	var bits := PackedFloat32Array([value]).to_byte_array().decode_u32(0)
	var exponent := ((bits >> 23) & 0xff) - 127
	return pow(2.0, exponent - mantissa_bits)


func quantize_packed(image: Image) -> Image:
	# Constant-field golden only: preserve measured packed-store math, without
	# claiming to emulate hardware's spatial interpolation or selecting a driver.
	for y in range(image.get_height()):
		for x in range(image.get_width()):
			var color := image.get_pixel(x, y)
			for channel in range(3):
				var step := packed_ulp(color[channel], 5 if channel == 2 else 6)
				color[channel] = floorf(maxf(color[channel], 0.0) / step) * step
			color.a = 1.0
			image.set_pixel(x, y, color)
	return image


func compare(actual: Image, expected: Image, label: String, constant := false) -> void:
	if actual == null or expected == null:
		return
	var max_error := 0.0
	for y in range(actual.get_height()):
		for x in range(actual.get_width()):
			var observed := actual.get_pixel(x, y)
			var wanted := expected.get_pixel(0 if constant else x, 0 if constant else y)
			for channel in range(4):
				comparisons += 1
				var error := absf(observed[channel] - wanted[channel])
				max_error = maxf(max_error, error)
				var tolerance := ABS_TOLERANCE + REL_TOLERANCE * absf(wanted[channel])
				if actual.get_format() == Image.FORMAT_RGBAF and channel < 3:
					tolerance = maxf(
						tolerance, packed_ulp(wanted[channel], 5 if channel == 2 else 6)
					)
				# Final rgba16f results may straddle a rounding boundary. Permit
				# one half ULP, not an arbitrary display-space pixel threshold.
				if actual.get_format() == Image.FORMAT_RGBAH and channel < 3:
					tolerance = maxf(tolerance, absf(wanted[channel]) / 1024.0)
				if channel == 3:
					tolerance = 0.0
				if (
					not is_finite(observed[channel])
					or not is_finite(wanted[channel])
					or error > tolerance
				):
					failures += 1
					push_error(
						(
							"%s (%d,%d) channel%d: %.9f expected %.9f tolerance %.9f"
							% [label, x, y, channel, observed[channel], wanted[channel], tolerance]
						)
					)
					return
	print("BLOOM_COMPUTE ", label, " extent=", actual.get_size(), " max_error=", max_error)


func release_textures() -> void:
	for texture in textures:
		if texture.is_valid():
			rd.free_rid(texture)
	textures.clear()


func finish() -> void:
	if finished:
		return
	finished = true
	if rd != null:
		release_textures()
		for pipeline in pipelines:
			if pipeline.is_valid():
				rd.free_rid(pipeline)
		pipelines.clear()
		for shader in shaders + raster_shaders:
			if shader.is_valid():
				rd.free_rid(shader)
		shaders.clear()
		raster_shaders.clear()
		if sampler.is_valid():
			rd.free_rid(sampler)
		rd.free()
		rd = null
	var engine_errors := observer.count()
	OS.remove_logger(observer)
	print(
		(
			"Bloom compute: %d comparisons, %d failures, %d engine errors"
			% [comparisons, failures, engine_errors]
		)
	)
	quit(0 if failures == 0 and engine_errors == 0 else 1)
