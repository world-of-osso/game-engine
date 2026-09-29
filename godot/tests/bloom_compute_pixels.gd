extends SceneTree
## Standalone numeric shader harness; no native extension or Options/controller.
## Run with Vulkan: Godot --path godot/tests --script bloom_compute_pixels.gd
## Creates a local RD, never reads display/tonemapped pixels. Tests the production
## shaders against bloom_reference.gd after every down/up pass, including alpha.
## CPU expected images quantize R/G to unsigned 5e6m and B to unsigned 5e5m
## after every packed store. One packed ULP tolerates GPU conversion rounding
## and boundary crossings; final RGBA16F permits one half ULP. Not bit-exact.
## Bounded 32-height spatial fixtures; full 512-height constant fixture exercises
## all 8 configured levels. This is NOT compositor lifecycle/rendered-UI proof.

const Reference = preload("bloom_reference.gd")
const ABS_TOLERANCE := 0.00008
const REL_TOLERANCE := 0.00003
var rd: RenderingDevice
var sampler := RID()
var shaders: Array[RID] = []
var pipelines: Array[RID] = []
var textures: Array[RID] = []
var failures := 0
var comparisons := 0


func _initialize() -> void:
	call_deferred("run_test")


func run_test() -> void:
	rd = RenderingServer.create_local_rendering_device()
	if rd == null:
		push_error("Bloom compute requires Vulkan RenderingDevice, not headless dummy rendering")
		quit(1)
		return
	var usage := (
		RenderingDevice.TEXTURE_USAGE_STORAGE_BIT | RenderingDevice.TEXTURE_USAGE_SAMPLING_BIT
	)
	if not rd.texture_is_format_supported_for_usage(
		RenderingDevice.DATA_FORMAT_B10G11R11_UFLOAT_PACK32, usage
	):
		failures += 1
		push_error("Original Bevy R11G11B10 storage/sampling unsupported")
		finish()
		return
	var state := RDSamplerState.new()
	state.min_filter = RenderingDevice.SAMPLER_FILTER_LINEAR
	state.mag_filter = RenderingDevice.SAMPLER_FILTER_LINEAR
	state.repeat_u = RenderingDevice.SAMPLER_REPEAT_MODE_CLAMP_TO_EDGE
	state.repeat_v = RenderingDevice.SAMPLER_REPEAT_MODE_CLAMP_TO_EDGE
	sampler = rd.sampler_create(state)
	# Diagnostic-only mode isolates format conversion from all bloom filtering.
	# -- --calibrate-packed prints identical f32 inputs through storage and a
	# raster color attachment, alongside exact nearest-even/truncation bits.
	if OS.get_cmdline_user_args().has("--calibrate-packed"):
		calibrate_packed_stores()
		finish()
		return
	var directory: String = get_script().resource_path.get_base_dir().path_join("../shaders")
	if not compile_shader(directory.path_join("bloom_downsample.glsl"), ""):
		finish()
		return
	if not compile_shader(directory.path_join("bloom_upsample.glsl"), ""):
		finish()
		return
	if not compile_shader(directory.path_join("bloom_upsample.glsl"), "#define FINAL_COMPOSITE\n"):
		finish()
		return
	var spatial := Image.create(17, 11, false, Image.FORMAT_RGBAH)
	spatial.fill(Color(0.15, 0.3, 0.55, 0.375))
	spatial.set_pixel(0, 0, Color(16, 4, 1, 0.25))
	spatial.set_pixel(8, 5, Color(8, 2, 4, 0.5))
	spatial.set_pixel(16, 10, Color(1, 6, 2, 0.75))
	test_fixture(spatial, 32, 0.08, "edge/impulse-colour")
	test_fixture(spatial, 32, 1.0, "intensity-one")
	var black := Image.create(9, 13, false, Image.FORMAT_RGBAH)
	black.fill(Color(0, 0, 0, 0.625))
	test_fixture(black, 32, 0.08, "finite-black")
	var constant := Image.create(19, 11, false, Image.FORMAT_RGBAH)
	constant.fill(Color(2, 1, 0.75, 0.375))
	test_fixture(constant, 512, 0.08, "configured-eight-levels", true)
	finish()


func calibrate_packed_stores() -> void:
	var compute_source := RDShaderSource.new()
	compute_source.source_compute = """#version 450
layout(local_size_x=1,local_size_y=1,local_size_z=1) in;
layout(r11f_g11f_b10f,set=0,binding=0) uniform writeonly image2D target;
layout(push_constant,std430) uniform Parameters { vec4 value; } p;
void main() { imageStore(target,ivec2(0),p.value); }
"""
	var compute_shader := compile_calibration_shader(compute_source)
	if not compute_shader.is_valid():
		return
	var compute_pipeline := rd.compute_pipeline_create(compute_shader)
	pipelines.append(compute_pipeline)
	var raster_source := RDShaderSource.new()
	raster_source.source_vertex = """#version 450
void main() {
    const vec2 vertices[3]=vec2[3](vec2(-1,-1),vec2(3,-1),vec2(-1,3));
    gl_Position=vec4(vertices[gl_VertexIndex],0,1);
}
"""
	raster_source.source_fragment = """#version 450
layout(location=0) out vec4 color;
layout(push_constant,std430) uniform Parameters { vec4 value; } p;
void main() { color=p.value; }
"""
	var raster_shader := compile_calibration_shader(raster_source)
	if not raster_shader.is_valid():
		return
	var format := RDTextureFormat.new()
	format.width = 1
	format.height = 1
	format.format = RenderingDevice.DATA_FORMAT_B10G11R11_UFLOAT_PACK32
	format.usage_bits = (
		RenderingDevice.TEXTURE_USAGE_STORAGE_BIT
		| RenderingDevice.TEXTURE_USAGE_COLOR_ATTACHMENT_BIT
		| RenderingDevice.TEXTURE_USAGE_CAN_COPY_FROM_BIT
	)
	if not rd.texture_is_format_supported_for_usage(format.format, format.usage_bits):
		failures += 1
		push_error("Calibration packed storage/color-attachment/readback unsupported")
		return
	var storage := rd.texture_create(format, RDTextureView.new())
	var attachment := rd.texture_create(format, RDTextureView.new())
	textures.append(storage)
	textures.append(attachment)
	if not storage.is_valid() or not attachment.is_valid():
		failures += 1
		push_error("Calibration texture allocation failed")
		return
	var attachments: Array[RID] = [attachment]
	var framebuffer := rd.framebuffer_create(attachments)
	if not framebuffer.is_valid():
		failures += 1
		push_error("Calibration framebuffer allocation failed")
		return
	var blend := RDPipelineColorBlendState.new()
	var blend_attachments: Array[RDPipelineColorBlendStateAttachment] = [
		RDPipelineColorBlendStateAttachment.new()
	]
	blend.attachments = blend_attachments
	var raster_pipeline := rd.render_pipeline_create(
		raster_shader,
		rd.framebuffer_get_format(framebuffer),
		-1,
		RenderingDevice.RENDER_PRIMITIVE_TRIANGLES,
		RDPipelineRasterizationState.new(),
		RDPipelineMultisampleState.new(),
		RDPipelineDepthStencilState.new(),
		blend
	)
	pipelines.append(raster_pipeline)
	var output := RDUniform.new()
	output.uniform_type = RenderingDevice.UNIFORM_TYPE_IMAGE
	output.binding = 0
	output.add_id(storage)
	var uniforms: Array[RDUniform] = [output]
	var binding := rd.uniform_set_create(uniforms, compute_shader, 0)
	if not compute_pipeline.is_valid() or not raster_pipeline.is_valid() or not binding.is_valid():
		failures += 1
		push_error("Calibration pipeline/uniform creation failed")
		if binding.is_valid():
			rd.free_rid(binding)
		rd.free_rid(framebuffer)
		return
	# Powers of two make all inputs exactly representable in f32. Cases above
	# and below half-ULP distinguish truncation from nearest and ties-to-even;
	# the odd tie, binade crossing and subnormal cover different conversion rules.
	var cases := [
		["quarter", PackedFloat32Array([1.0 + 0.25 / 64, 2.0 + 0.25 / 32, 1.0 + 0.25 / 32, 1])],
		["tie-even", PackedFloat32Array([1.0 + 0.5 / 64, 2.0 + 0.5 / 32, 1.0 + 0.5 / 32, 1])],
		["tie-odd", PackedFloat32Array([1.0 + 1.5 / 64, 2.0 + 1.5 / 32, 1.0 + 1.5 / 32, 1])],
		[
			"three-quarter",
			PackedFloat32Array([1.0 + 0.75 / 64, 2.0 + 0.75 / 32, 1.0 + 0.75 / 32, 1])
		],
		["binade", PackedFloat32Array([2.0 - 0.25 / 64, 4.0 - 0.25 / 32, 2.0 - 0.25 / 32, 1])],
		[
			"subnormal",
			PackedFloat32Array(
				[3.75 * pow(2.0, -20), 3.75 * pow(2.0, -20), 3.75 * pow(2.0, -19), 1]
			)
		],
	]
	for entry in cases:
		var values: PackedFloat32Array = entry[1]
		var parameters := values.to_byte_array()
		var commands := rd.compute_list_begin()
		rd.compute_list_bind_compute_pipeline(commands, compute_pipeline)
		rd.compute_list_bind_uniform_set(commands, binding, 0)
		rd.compute_list_set_push_constant(commands, parameters, parameters.size())
		rd.compute_list_dispatch(commands, 1, 1, 1)
		rd.compute_list_end()
		var draw := rd.draw_list_begin(framebuffer)
		rd.draw_list_bind_render_pipeline(draw, raster_pipeline)
		rd.draw_list_set_push_constant(draw, parameters, parameters.size())
		rd.draw_list_draw(draw, false, 1, 3)
		rd.draw_list_end()
		rd.submit()
		rd.sync()
		var stored := rd.texture_get_data(storage, 0).decode_u32(0)
		var rendered := rd.texture_get_data(attachment, 0).decode_u32(0)
		var nearest := expected_packed_bits(values, true)
		var truncated := expected_packed_bits(values, false)
		print(
			(
				"BLOOM_STORE_CALIBRATION %s f32=%s storage=0x%08x raster=0x%08x nearest=0x%08x trunc=0x%08x storage_nearest=%s storage_trunc=%s raster_nearest=%s raster_trunc=%s"
				% [
					entry[0],
					values,
					stored,
					rendered,
					nearest,
					truncated,
					stored == nearest,
					stored == truncated,
					rendered == nearest,
					rendered == truncated
				]
			)
		)
		for channel in range(3):
			var bits := 5 if channel == 2 else 6
			var shift := 22 if channel == 2 else channel * 11
			var mask := (1 << (bits + 5)) - 1
			print(
				(
					"BLOOM_STORE_CHANNEL %s channel%d input=%.12f storage=%.12f raster=%.12f nearest=%.12f trunc=%.12f"
					% [
						entry[0],
						channel,
						values[channel],
						decode_unsigned_float((stored >> shift) & mask, bits),
						decode_unsigned_float((rendered >> shift) & mask, bits),
						decode_unsigned_float((nearest >> shift) & mask, bits),
						decode_unsigned_float((truncated >> shift) & mask, bits)
					]
				)
			)
	rd.free_rid(binding)
	rd.free_rid(framebuffer)


func compile_calibration_shader(source: RDShaderSource) -> RID:
	var spirv := rd.shader_compile_spirv_from_source(source)
	for stage in [
		RenderingDevice.SHADER_STAGE_VERTEX,
		RenderingDevice.SHADER_STAGE_FRAGMENT,
		RenderingDevice.SHADER_STAGE_COMPUTE
	]:
		var error := spirv.get_stage_compile_error(stage)
		if not error.is_empty():
			failures += 1
			push_error("Calibration shader compile: " + error)
			return RID()
	var shader := rd.shader_create_from_spirv(spirv)
	shaders.append(shader)
	if not shader.is_valid():
		failures += 1
		push_error("Calibration shader creation failed")
	return shader


func expected_packed_bits(values: PackedFloat32Array, nearest: bool) -> int:
	var packed := 0
	for channel in range(3):
		var mantissa := 5 if channel == 2 else 6
		var value := float(values[channel])
		var step := packed_ulp(value, mantissa)
		var rounded := (
			quantize_unsigned_float(value, mantissa) if nearest else floorf(value / step) * step
		)
		var encoded := 0
		if rounded > 0.0 and rounded < pow(2.0, -14):
			encoded = roundi(rounded / pow(2.0, -14 - mantissa))
		elif rounded > 0.0:
			var float_bits := PackedFloat32Array([rounded]).to_byte_array().decode_u32(0)
			var exponent := ((float_bits >> 23) & 0xff) - 127
			var fraction := roundi((rounded / pow(2.0, exponent) - 1.0) * (1 << mantissa))
			encoded = ((exponent + 15) << mantissa) | fraction
		packed |= encoded << (22 if channel == 2 else channel * 11)
	return packed


func compile_shader(path: String, defines: String) -> bool:
	var code := FileAccess.get_file_as_string(path).replace("#[compute]\n", "")
	if code.is_empty():
		failures += 1
		push_error("Missing production shader: " + path)
		return false
	code = code.replace("#version 450\n", "#version 450\n" + defines)
	var source := RDShaderSource.new()
	source.source_compute = code
	var spirv := rd.shader_compile_spirv_from_source(source)
	var error := spirv.get_stage_compile_error(RenderingDevice.SHADER_STAGE_COMPUTE)
	if not error.is_empty():
		failures += 1
		push_error(error)
		return false
	var shader := rd.shader_create_from_spirv(spirv)
	shaders.append(shader)
	var pipeline := rd.compute_pipeline_create(shader)
	pipelines.append(pipeline)
	if not shader.is_valid() or not pipeline.is_valid():
		failures += 1
		push_error("Bloom shader/pipeline creation failed")
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
		| RenderingDevice.TEXTURE_USAGE_CAN_COPY_FROM_BIT
	)
	var initial: Array[PackedByteArray] = []
	if image.get_format() == Image.FORMAT_RGBAH:
		initial.append(image.get_data())
	var texture := rd.texture_create(format, RDTextureView.new(), initial)
	textures.append(texture)
	if not texture.is_valid():
		failures += 1
		push_error("Bloom compute texture creation failed at %s" % image.get_size())
	return texture


func dispatch(index: int, input: RID, target: RID, size: Vector2i, value: float) -> bool:
	var sampled := RDUniform.new()
	sampled.uniform_type = RenderingDevice.UNIFORM_TYPE_SAMPLER_WITH_TEXTURE
	sampled.binding = 0
	sampled.add_id(sampler)
	sampled.add_id(input)
	var output := RDUniform.new()
	output.uniform_type = RenderingDevice.UNIFORM_TYPE_IMAGE
	output.binding = 1
	output.add_id(target)
	var uniforms: Array[RDUniform] = [sampled, output]
	var binding := rd.uniform_set_create(uniforms, shaders[index], 0)
	if not binding.is_valid():
		failures += 1
		push_error("Bloom compute uniform set creation failed")
		return false
	var commands := rd.compute_list_begin()
	rd.compute_list_bind_compute_pipeline(commands, pipelines[index])
	rd.compute_list_bind_uniform_set(commands, binding, 0)
	var parameters := PackedFloat32Array([value, 0, 0, 0]).to_byte_array()
	rd.compute_list_set_push_constant(commands, parameters, parameters.size())
	rd.compute_list_dispatch(commands, ceili(size.x / 8.0), ceili(size.y / 8.0), 1)
	rd.compute_list_end()
	rd.submit()
	rd.sync()
	rd.free_rid(binding)
	return true


func read_texture(texture: RID, size: Vector2i, half := false) -> Image:
	var bytes := rd.texture_get_data(texture, 0)
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


func quantize_unsigned_float(value: float, mantissa_bits: int) -> float:
	var step := packed_ulp(value, mantissa_bits)
	var scaled := maxf(value, 0.0) / step
	var integer := floori(scaled)
	var fraction := scaled - integer
	# Round-to-nearest ties-to-even. GPUs may select a neighbouring packed value;
	# compare() explicitly permits one ULP, but the recurrence remains independent.
	if fraction > 0.5 or (fraction == 0.5 and (integer & 1) != 0):
		integer += 1
	var rounded := integer * step
	return INF if rounded >= 65536.0 else rounded


func quantize_packed(image: Image) -> Image:
	for y in range(image.get_height()):
		for x in range(image.get_width()):
			var color := image.get_pixel(x, y)
			image.set_pixel(
				x,
				y,
				Color(
					quantize_unsigned_float(color.r, 6),
					quantize_unsigned_float(color.g, 6),
					quantize_unsigned_float(color.b, 5),
					1.0
				)
			)
	return image


func test_fixture(
	original: Image, max_height: int, intensity: float, label: String, constant := false
) -> void:
	var sizes: Array[Vector2i] = Reference.mip_sizes(original.get_size(), max_height)
	var scene := create_texture(original)
	if not scene.is_valid():
		return
	var pyramid: Array[RID] = []
	var expected: Array[Image] = []
	var source := scene
	var cpu := original
	# For a constant field, a 1x1 oracle evaluates exactly the same filtering
	# and clamp-to-edge operations without 512-height CPU neighbourhood loops.
	if constant:
		cpu = Image.create(1, 1, false, Image.FORMAT_RGBAF)
		cpu.fill(original.get_pixel(0, 0))
	for level in range(sizes.size()):
		var target := create_texture(
			Image.create(sizes[level].x, sizes[level].y, false, Image.FORMAT_RGBAF)
		)
		if (
			not target.is_valid()
			or not dispatch(0, source, target, sizes[level], 1.0 if level == 0 else 0.0)
		):
			return
		cpu = quantize_packed(
			Reference.downsample(cpu, Vector2i.ONE if constant else sizes[level], level == 0)
		)
		expected.append(cpu)
		compare(read_texture(target, sizes[level]), cpu, "%s down%d" % [label, level], constant)
		pyramid.append(target)
		source = target
	var last := sizes.size() - 1
	for level in range(last, 0, -1):
		var blend: float = Reference.blend_factor(level, last, intensity)
		if not dispatch(1, pyramid[level], pyramid[level - 1], sizes[level - 1], blend):
			return
		cpu = quantize_packed(Reference.upsample_add(cpu, expected[level - 1], blend))
		compare(
			read_texture(pyramid[level - 1], sizes[level - 1]),
			cpu,
			"%s up%d" % [label, level],
			constant
		)
	if not dispatch(2, pyramid[0], scene, original.get_size(), intensity):
		return
	var destination := original
	if constant:
		destination = Image.create(1, 1, false, Image.FORMAT_RGBAF)
		destination.fill(original.get_pixel(0, 0))
	cpu = Reference.upsample_add(cpu, destination, intensity)
	# Match final half-framebuffer rounding after packed quantization at each level.
	cpu.convert(Image.FORMAT_RGBAH)
	compare(read_texture(scene, original.get_size(), true), cpu, label + " final", constant)
	for texture in textures:
		if texture.is_valid():
			rd.free_rid(texture)
	textures.clear()


func compare(actual: Image, expected: Image, label: String, constant: bool) -> void:
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
				if not is_finite(observed[channel]) or error > tolerance:
					failures += 1
					push_error(
						(
							"%s (%d,%d) channel%d: %.9f expected %.9f tolerance %.9f"
							% [label, x, y, channel, observed[channel], wanted[channel], tolerance]
						)
					)
					return
	print("BLOOM_COMPUTE ", label, " extent=", actual.get_size(), " max_error=", max_error)


func finish() -> void:
	for texture in textures:
		if texture.is_valid():
			rd.free_rid(texture)
	for pipeline in pipelines:
		if pipeline.is_valid():
			rd.free_rid(pipeline)
	for shader in shaders:
		if shader.is_valid():
			rd.free_rid(shader)
	if sampler.is_valid():
		rd.free_rid(sampler)
	if rd != null:
		rd.free()
	print("Bloom compute: %d comparisons, %d failures" % [comparisons, failures])
	quit(0 if failures == 0 else 1)
