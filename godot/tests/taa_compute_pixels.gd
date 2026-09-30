extends SceneTree
## Bounded kernel differential, NOT SDR history quantization or full-pipeline proof.
## HALF colour/depth/motion inputs and both MRT/compute outputs; single sample,
## no mipmaps. Current/history extents may differ. Fixed pre-run one HALF ULP
## (exponent of reference value) + 2e-5 for every channel; nonfinite always fails.
## Main runs: Godot --path godot --rendering-method forward_plus
## --rendering-driver vulkan --script res://tests/taa_compute_pixels.gd

const ABS_TOLERANCE := 0.00002
const VERTEX := """#version 450
layout(location=0) out vec2 uv;
void main() {
    uv = vec2(float(gl_VertexIndex & 2), float((gl_VertexIndex & 1) * 2));
    gl_Position = vec4(uv * 2.0 - 1.0, 0.0, 1.0);
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
var nearest := RID()
var linear := RID()
var compute_shader := RID()
var compute_pipeline := RID()
var raster_shaders: Array[RID] = []
var textures: Array[RID] = []
var failures := 0
var comparisons := 0
var fixtures := 0
var finished := false


func _initialize() -> void:
	OS.add_logger(observer)
	call_deferred("run_test")


func run_test() -> void:
	if RenderingServer.get_current_rendering_driver_name().to_lower() != "vulkan":
		push_error("TAA differential requires Vulkan, not dummy/headless rendering")
		finish()
		return
	rd = RenderingServer.create_local_rendering_device()
	if rd == null:
		push_error("TAA requires local RenderingDevice")
		finish()
		return
	nearest = make_sampler(RenderingDevice.SAMPLER_FILTER_NEAREST)
	linear = make_sampler(RenderingDevice.SAMPLER_FILTER_LINEAR)
	if not nearest.is_valid() or not linear.is_valid() or not compile_shaders():
		finish()
		return
	test_sampling()
	if failures != 0 or observer.count() != 0:
		finish()
		return
	for tonemap in [false, true]:
		for reset in [false, true]:
			for kind in [
				"constant",
				"black",
				"colour-3x3",
				"ramp",
				"depth-ties",
				"depth-tl",
				"depth-tr",
				"depth-bl",
				"depth-br"
			]:
				test_fixture(kind, reset, tonemap, Vector2i(17, 9))
			for history_size in [Vector2i(7, 13), Vector2i(23, 5), Vector2i.ONE]:
				test_fixture("ramp", reset, tonemap, history_size)
			if observer.count() != 0:
				finish()
				return
	finish()


func make_sampler(filter_mode: int) -> RID:
	var state := RDSamplerState.new()
	state.min_filter = filter_mode
	state.mag_filter = filter_mode
	state.repeat_u = RenderingDevice.SAMPLER_REPEAT_MODE_CLAMP_TO_EDGE
	state.repeat_v = RenderingDevice.SAMPLER_REPEAT_MODE_CLAMP_TO_EDGE
	var result := rd.sampler_create(state)
	if not result.is_valid():
		push_error("TAA sampler creation failed")
	return result


func compile_source(source: RDShaderSource, stages: Array[int]) -> RID:
	var spirv := rd.shader_compile_spirv_from_source(source, false)
	if spirv == null:
		push_error("TAA SPIR-V compilation returned null")
		return RID()
	for stage in stages:
		var error := spirv.get_stage_compile_error(stage)
		if not error.is_empty():
			push_error("TAA stage%d: %s" % [stage, error])
			return RID()
	var shader := rd.shader_create_from_spirv(spirv)
	if not shader.is_valid():
		push_error("TAA shader creation failed")
	return shader


func compile_shaders() -> bool:
	var directory: String = get_script().resource_path.get_base_dir()
	var code := FileAccess.get_file_as_string(directory.path_join("../shaders/taa_resolve.glsl"))
	if code.is_empty():
		push_error("Missing production taa_resolve.glsl")
		return false
	var source := RDShaderSource.new()
	source.source_compute = code.replace("#[compute]\n", "")
	compute_shader = compile_source(source, [RenderingDevice.SHADER_STAGE_COMPUTE])
	if not compute_shader.is_valid():
		return false
	compute_pipeline = rd.compute_pipeline_create(compute_shader)
	if not compute_pipeline.is_valid() or not rd.compute_pipeline_is_valid(compute_pipeline):
		push_error("TAA compute pipeline creation failed")
		return false
	code = FileAccess.get_file_as_string(directory.path_join("taa_legacy_raster.glsl"))
	if code.is_empty():
		push_error("Missing independent legacy raster shader")
		return false
	for defines in [
		"",
		"#define RESET\n",
		"#define TONEMAP\n",
		"#define RESET\n#define TONEMAP\n",
		"#define SAMPLE_PROBE\n",
		"#define AUX_SAMPLE_PROBE\n"
	]:
		source = RDShaderSource.new()
		source.source_vertex = VERTEX
		source.source_fragment = code.replace("#version 450\n", "#version 450\n" + defines)
		var shader := compile_source(
			source, [RenderingDevice.SHADER_STAGE_VERTEX, RenderingDevice.SHADER_STAGE_FRAGMENT]
		)
		raster_shaders.append(shader)
		if not shader.is_valid():
			return false
	return true


func image(size: Vector2i, colour: Color) -> Image:
	var result := Image.create(size.x, size.y, false, Image.FORMAT_RGBAH)
	result.fill(colour)
	return result


func create_texture(input: Image) -> RID:
	var format := RDTextureFormat.new()
	format.width = input.get_width()
	format.height = input.get_height()
	format.format = RenderingDevice.DATA_FORMAT_R16G16B16A16_SFLOAT
	format.usage_bits = (
		RenderingDevice.TEXTURE_USAGE_SAMPLING_BIT
		| RenderingDevice.TEXTURE_USAGE_STORAGE_BIT
		| RenderingDevice.TEXTURE_USAGE_COLOR_ATTACHMENT_BIT
		| RenderingDevice.TEXTURE_USAGE_CAN_COPY_FROM_BIT
	)
	if not rd.texture_is_format_supported_for_usage(format.format, format.usage_bits):
		push_error("TAA HALF sampling/storage/MRT/readback unsupported")
		return RID()
	var bytes: Array[PackedByteArray] = [input.get_data()]
	var result := rd.texture_create(format, RDTextureView.new(), bytes)
	textures.append(result)
	if not result.is_valid():
		push_error("TAA texture creation failed at %s" % input.get_size())
	return result


func uniforms(inputs: Array[RID]) -> Array[RDUniform]:
	var result: Array[RDUniform] = []
	for binding in range(5):
		var uniform := RDUniform.new()
		uniform.binding = binding
		uniform.uniform_type = RenderingDevice.UNIFORM_TYPE_SAMPLER_WITH_TEXTURE
		uniform.add_id(linear if binding == 1 else nearest)
		uniform.add_id(inputs[binding])
		result.append(uniform)
	return result


func dispatch(
	inputs: Array[RID], outputs: Array[RID], size: Vector2i, reset: bool, tonemap: bool
) -> bool:
	var bindings := uniforms(inputs)
	for index in range(2):
		var uniform := RDUniform.new()
		uniform.binding = 5 + index
		uniform.uniform_type = RenderingDevice.UNIFORM_TYPE_IMAGE
		uniform.add_id(outputs[index])
		bindings.append(uniform)
	var binding := rd.uniform_set_create(bindings, compute_shader, 0)
	if not binding.is_valid():
		push_error("TAA compute uniform set creation failed")
		return false
	var commands := rd.compute_list_begin()
	if commands == RenderingDevice.INVALID_ID:
		rd.free_rid(binding)
		push_error("TAA compute list begin failed")
		return false
	rd.compute_list_bind_compute_pipeline(commands, compute_pipeline)
	rd.compute_list_bind_uniform_set(commands, binding, 0)
	var push := PackedByteArray()
	push.resize(8)
	push.encode_u32(0, int(reset))
	push.encode_u32(4, int(tonemap))
	rd.compute_list_set_push_constant(commands, push, 8)
	rd.compute_list_dispatch(commands, ceili(size.x / 8.0), ceili(size.y / 8.0), 1)
	rd.compute_list_end()
	rd.submit()
	rd.sync()
	rd.free_rid(binding)
	return observer.count() == 0


func draw(shader: RID, inputs: Array[RID], outputs: Array[RID]) -> bool:
	var framebuffer := rd.framebuffer_create(outputs)
	if not framebuffer.is_valid():
		push_error("TAA MRT framebuffer creation failed")
		return false
	var blend := RDPipelineColorBlendState.new()
	var attachments: Array[RDPipelineColorBlendStateAttachment] = [
		RDPipelineColorBlendStateAttachment.new(), RDPipelineColorBlendStateAttachment.new()
	]
	blend.attachments = attachments
	var pipeline := rd.render_pipeline_create(
		shader,
		rd.framebuffer_get_format(framebuffer),
		-1,
		RenderingDevice.RENDER_PRIMITIVE_TRIANGLES,
		RDPipelineRasterizationState.new(),
		RDPipelineMultisampleState.new(),
		RDPipelineDepthStencilState.new(),
		blend
	)
	var binding := rd.uniform_set_create(uniforms(inputs), shader, 0)
	var valid := pipeline.is_valid() and binding.is_valid()
	if valid:
		valid = rd.render_pipeline_is_valid(pipeline)
	if valid:
		valid = record_draw(framebuffer, pipeline, binding)
	else:
		push_error("TAA raster pipeline/uniform set creation failed")
	if binding.is_valid():
		rd.free_rid(binding)
	if pipeline.is_valid():
		rd.free_rid(pipeline)
	rd.free_rid(framebuffer)
	return valid and observer.count() == 0


func record_draw(framebuffer: RID, pipeline: RID, binding: RID) -> bool:
	var commands := rd.draw_list_begin(framebuffer, RenderingDevice.DRAW_DEFAULT_ALL)
	if commands == RenderingDevice.INVALID_ID:
		push_error("TAA raster draw list begin failed")
		return false
	rd.draw_list_bind_render_pipeline(commands, pipeline)
	rd.draw_list_bind_uniform_set(commands, binding, 0)
	rd.draw_list_draw(commands, false, 1, 3)
	rd.draw_list_end()
	rd.submit()
	rd.sync()
	return true


func fixture_images(kind: String, history_size: Vector2i) -> Array[Image]:
	var size := Vector2i(17, 9)
	var current := image(size, Color(0.25, 0.5, 0.75, 0.375))
	var past := image(history_size, Color(0.25, 0.5, 0.75, 90))
	var depth := image(size, Color(0.25, 0, 0, 0))
	var motion := image(size, Color(0, 0, 0, 0))
	if kind == "black":
		current.fill(Color(0, 0, 0, 0.625))
		past.fill(Color(0, 0, 0, 120))
	elif kind != "constant":
		for y in range(size.y):
			for x in range(size.x):
				var rgb := Color(
					float(x) / 8.0, float(y) / 4.0, float(x + y) / 16.0, float((x + y) % 5) / 4.0
				)
				if kind == "colour-3x3":
					rgb = Color(
						float(x % 3) / 2.0, float(y % 3) / 2.0, float((x + y) % 3) / 2.0, rgb.a
					)
				current.set_pixel(x, y, rgb)
				# Includes signed fractional motion, offscreen in all directions,
				# and HALF-representable values immediately bracketing <0.01 px.
				var moves := [
					Vector2.ZERO,
					Vector2(0.37 / size.x, -0.23 / size.y),
					Vector2(-0.61 / size.x, 0.43 / size.y),
					Vector2(2, 0),
					Vector2(-2, 0),
					Vector2(0, 2),
					Vector2(0, -2),
					Vector2(0.00999 / size.x, 0),
					Vector2(0.01001 / size.x, 0),
					Vector2(0, 0.00999 / size.y),
					Vector2(0, 0.01001 / size.y),
					Vector2(-0.00999 / size.x, 0),
					Vector2(-0.01001 / size.x, 0)
				]
				var move: Vector2 = moves[(x + 3 * y) % moves.size()]
				motion.set_pixel(x, y, Color(move.x, move.y, 0, 0))
		for y in range(history_size.y):
			for x in range(history_size.x):
				var confidence: float = [-20.0, -10.0, -9.0, 0.0, 1.0, 56.0, 90.0, 120.0][
					(x + y) % 8
				]
				past.set_pixel(
					x,
					y,
					Color(
						float(x + 1) / (history_size.x + 1),
						float(y + 1) / (history_size.y + 1),
						0.375,
						confidence
					)
				)
	if kind.begins_with("depth-"):
		# At (8,4), four 2-texel diagonals select distinct motions.
		# All equal to center must retain center. Equal winning corners must
		# retain first TL, not last BR. A one-texel distractor beats every
		# intended depth, exposing an incorrect one-texel search.
		motion.set_pixel(8, 4, Color(0, 0, 0, 0))
		var corners := [Vector2i(6, 6), Vector2i(10, 6), Vector2i(6, 2), Vector2i(10, 2)]
		var names := ["tl", "tr", "bl", "br"]
		for index in range(4):
			var point: Vector2i = corners[index]
			depth.set_pixelv(
				point,
				Color(
					0.75 if kind == "depth-ties" or kind == "depth-" + names[index] else 0.25,
					0,
					0,
					0
				)
			)
			motion.set_pixelv(point, Color(float(index + 1) / 32.0, -float(index) / 32.0, 0, 0))
		depth.set_pixel(7, 5, Color(1, 0, 0, 0))
	return [current, past, depth, motion]


func test_fixture(kind: String, reset: bool, tonemap: bool, history_size: Vector2i) -> void:
	var label := "%s reset=%s tonemap=%s history=%s" % [kind, reset, tonemap, history_size]
	var source := fixture_images(kind, history_size)
	var current := create_texture(source[0])
	var past := create_texture(source[1])
	var inputs: Array[RID] = [
		current, past, past, create_texture(source[2]), create_texture(source[3])
	]
	var blank := image(source[0].get_size(), Color(-5, -5, -5, -5))
	var output: Array[RID] = [create_texture(blank), create_texture(blank)]
	var oracle: Array[RID] = [create_texture(blank), create_texture(blank)]
	if all_valid(inputs + output + oracle):
		if (
			dispatch(inputs, output, blank.get_size(), reset, tonemap)
			and draw(raster_shaders[int(reset) + 2 * int(tonemap)], inputs, oracle)
		):
			fixtures += 1
			for index in range(2):
				var actual := read_texture(output[index], blank.get_size())
				var expected := read_texture(oracle[index], blank.get_size())
				compare(actual, expected, label + (" resolved" if index == 0 else " history"))
				if reset or kind in ["constant", "black"]:
					var wanted := golden(source, kind, reset, tonemap, index)
					compare(actual, wanted, label + " compute golden%d" % index)
					compare(expected, wanted, label + " raster golden%d" % index)
			# Alpha preservation checked separately, not just differential agreement.
			var actual := read_texture(output[0], blank.get_size())
			if actual != null:
				for y in range(blank.get_height()):
					for x in range(blank.get_width()):
						check_value(
							actual.get_pixel(x, y).a,
							source[0].get_pixel(x, y).a,
							label + " original-alpha",
							x,
							y,
							3,
							true
						)
	release_textures()


func golden(source: Array[Image], kind: String, reset: bool, tonemap: bool, output: int) -> Image:
	# Handcomputed constant: max=3/4, denominator=7/4, history RGB=(1,2,3)/7.
	# Static confidence 90+10=100; reset confidence 1/(3/200)=200/3.
	if kind == "constant":
		var value := Color(0.25, 0.5, 0.75, 0.375)
		if output == 1:
			value = (
				Color(1.0 / 7.0, 2.0 / 7.0, 3.0 / 7.0, 100)
				if tonemap
				else Color(0.25, 0.5, 0.75, 100)
			)
			if reset:
				value.a = 66.66666666666667
		return image(source[0].get_size(), value)
	# Reset: no neighbourhood/history math. Black static confidence=120+10=130.
	var result := source[0].duplicate() as Image
	for y in range(result.get_height()):
		for x in range(result.get_width()):
			var value := result.get_pixel(x, y)
			if output == 1:
				if tonemap:
					var reciprocal := 1.0 / (1.0 + maxf(value.r, maxf(value.g, value.b)))
					value.r *= reciprocal
					value.g *= reciprocal
					value.b *= reciprocal
				value.a = 1.0 / 0.015 if reset else source[1].get_pixel(0, 0).a + 10.0
			result.set_pixel(x, y, value)
	return result


func all_valid(rids: Array[RID]) -> bool:
	for rid in rids:
		if not rid.is_valid():
			return false
	return true


func read_texture(texture: RID, size: Vector2i) -> Image:
	var bytes := rd.texture_get_data(texture, 0)
	if bytes.size() != size.x * size.y * 8:
		push_error("TAA HALF readback byte count mismatch at %s" % size)
		return null
	return Image.create_from_data(size.x, size.y, false, Image.FORMAT_RGBAH, bytes)


func half_ulp(value: float) -> float:
	if absf(value) < pow(2.0, -14):
		return pow(2.0, -24)
	var bits := PackedFloat32Array([absf(value)]).to_byte_array().decode_u32(0)
	return pow(2.0, ((bits >> 23) & 255) - 127 - 10)


func check_value(
	actual: float, expected: float, label: String, x: int, y: int, channel: int, exact := false
) -> void:
	comparisons += 1
	var tolerance := 0.0 if exact else ABS_TOLERANCE + half_ulp(expected)
	if not is_finite(actual) or not is_finite(expected) or absf(actual - expected) > tolerance:
		failures += 1
		# Bound failure stream without suppressing failure count or exit status.
		if failures <= 32:
			push_error(
				(
					"%s (%d,%d) channel%d: %.9f expected %.9f tolerance %.9f"
					% [label, x, y, channel, actual, expected, tolerance]
				)
			)


func compare(actual: Image, expected: Image, label: String) -> void:
	if actual == null or expected == null:
		return
	if actual.get_size() != expected.get_size():
		push_error("TAA extent mismatch " + label)
		return
	var maximum := 0.0
	for y in range(actual.get_height()):
		for x in range(actual.get_width()):
			var observed := actual.get_pixel(x, y)
			var wanted := expected.get_pixel(x, y)
			for channel in range(4):
				check_value(observed[channel], wanted[channel], label, x, y, channel)
				maximum = maxf(maximum, absf(observed[channel] - wanted[channel]))
	print("TAA_COMPUTE ", label, " extent=", actual.get_size(), " max_error=", maximum)


func test_sampling() -> void:
	# Dyadic UVs avoid nearest-boundary ties and nonrepresentable filter weights
	# in this CPU sampler oracle. Temporal differential fixtures stay 17x9.
	var size := Vector2i(8, 8)
	var source := image(Vector2i(3, 2), Color())
	for y in range(2):
		for x in range(3):
			source.set_pixel(x, y, Color(x * 0.25, y * 0.5, (x + y) * 0.125, x + 4 * y))
	var texture := create_texture(source)
	var inputs: Array[RID] = [texture, texture, texture, texture, texture]
	var outputs: Array[RID] = [
		create_texture(image(size, Color())), create_texture(image(size, Color()))
	]
	if all_valid(inputs + outputs):
		for variant in [4, 5]:
			if not draw(raster_shaders[variant], inputs, outputs):
				break
			for index in range(2):
				var expected := image(size, Color())
				for y in range(size.y):
					for x in range(size.x):
						var uv := Vector2(
							2.0 * (x + 0.5) / size.x - 0.5, 2.0 * (y + 0.5) / size.y - 0.5
						)
						var point := Vector2i(
							clampi(floori(uv.x * 3), 0, 2), clampi(floori(uv.y * 2), 0, 1)
						)
						var nearest_value := source.get_pixelv(point)
						var value := nearest_value
						if variant == 4 and index == 1:
							value = linear_sample(source, uv)
							value.a = nearest_value.a
						elif variant == 5:
							value = Color(nearest_value.r, nearest_value.r, nearest_value.g, 1)
						expected.set_pixel(x, y, value)
				compare(
					read_texture(outputs[index], size),
					expected,
					"physical nearest/linear clamp probe%d output%d" % [variant, index]
				)
	release_textures()


func linear_sample(source: Image, uv: Vector2) -> Color:
	# Physical sampler oracle only: explicit four texels, no TAA CPU reference.
	var position := uv * Vector2(source.get_size()) - Vector2(0.5, 0.5)
	var lower := Vector2i(floori(position.x), floori(position.y))
	var fraction := position - Vector2(lower)
	var rows: Array[Color] = []
	for y in range(2):
		var left := source.get_pixel(
			clampi(lower.x, 0, source.get_width() - 1),
			clampi(lower.y + y, 0, source.get_height() - 1)
		)
		var right := source.get_pixel(
			clampi(lower.x + 1, 0, source.get_width() - 1),
			clampi(lower.y + y, 0, source.get_height() - 1)
		)
		rows.append(left.lerp(right, fraction.x))
	return rows[0].lerp(rows[1], fraction.y)


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
		if compute_pipeline.is_valid():
			rd.free_rid(compute_pipeline)
		for shader in raster_shaders + [compute_shader]:
			if shader.is_valid():
				rd.free_rid(shader)
		for sampler in [nearest, linear]:
			if sampler.is_valid():
				rd.free_rid(sampler)
		rd.free()
		rd = null
	var errors := observer.count()
	OS.remove_logger(observer)
	print(
		(
			"TAA HALF differential: %d fixtures, %d comparisons, %d failures, %d engine errors"
			% [fixtures, comparisons, failures, errors]
		)
	)
	quit(0 if failures == 0 and errors == 0 and fixtures == 48 else 1)
