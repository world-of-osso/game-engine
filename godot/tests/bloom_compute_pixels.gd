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
	var directory := get_script().resource_path.get_base_dir().path_join("../shaders")
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
	return texture


func dispatch(index: int, input: RID, target: RID, size: Vector2i, value: float) -> void:
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
		dispatch(0, source, target, sizes[level], 1.0 if level == 0 else 0.0)
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
		dispatch(1, pyramid[level], pyramid[level - 1], sizes[level - 1], blend)
		cpu = quantize_packed(Reference.upsample_add(cpu, expected[level - 1], blend))
		compare(
			read_texture(pyramid[level - 1], sizes[level - 1]),
			cpu,
			"%s up%d" % [label, level],
			constant
		)
	dispatch(2, pyramid[0], scene, original.get_size(), intensity)
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
							"%s (%d,%d) channel%d: %g expected %g tolerance %g"
							% [label, x, y, channel, observed[channel], wanted[channel], tolerance]
						)
					)
					return
	print("BLOOM_COMPUTE ", label, " extent=", actual.get_size(), " max_error=", max_error)


func finish() -> void:
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
