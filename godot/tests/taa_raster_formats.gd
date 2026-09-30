extends "res://tests/taa_compute_pixels.gd"
## Preparation only. Main owns Vulkan execution; no renderer/controller wiring.
## Production raster vs independent legacy raster, every HALF/SRGB resolved/history
## pair, RESET x TONEMAP, five single-frame cases and four feedback frames.
## Opt-in Godot motion ABI gets separate raw-motion + jitter differential cases;
## these synthetic cases do not prove native motion/jitter producer semantics.
## SRGB compares stored encoded RGB and linear A bytes: <=1 code, fixed pre-run.
## HALF retains parent's one reference HALF ULP + 2e-5; nonfinite always fails.
## Run: Godot --path godot --rendering-method forward_plus --rendering-driver vulkan
## --script res://tests/taa_raster_formats.gd

const HALF := RenderingDevice.DATA_FORMAT_R16G16B16A16_SFLOAT
const SRGB := RenderingDevice.DATA_FORMAT_R8G8B8A8_SRGB
const FLOAT := RenderingDevice.DATA_FORMAT_R32G32B32A32_SFLOAT
const INPUT_SAMPLE := """#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 decoded;
layout(location = 1) out vec4 duplicate_sample;
layout(set = 0, binding = 0) uniform sampler2D uploaded_input;
void main() {
    decoded = texture(uploaded_input, uv);
    duplicate_sample = decoded;
}
"""
const SIZE := Vector2i(3, 3)
const CASES := ["mixed-low", "mixed-high", "opaque", "fractional", "offscreen"]
const EXPECTED_FIXTURES := 128

var production_shader := RID()
var motion_shader := RID()
var input_shader := RID()
var input_draw := false
var motion_adapter := false
var draw_jitter := Vector2.ZERO
var production_draw := false
var motion_draw := false
var draw_reset := false
var draw_tonemap := false
var format_coverage: Dictionary = {}


func run_test() -> void:
	if RenderingServer.get_current_rendering_driver_name().to_lower() != "vulkan":
		push_error("TAA format differential requires Vulkan")
		finish()
		return
	rd = RenderingServer.create_local_rendering_device()
	if rd == null:
		push_error("TAA format differential requires local RenderingDevice")
		finish()
		return
	nearest = make_sampler(RenderingDevice.SAMPLER_FILTER_NEAREST)
	linear = make_sampler(RenderingDevice.SAMPLER_FILTER_LINEAR)
	if not nearest.is_valid() or not linear.is_valid() or not compile_shaders():
		finish()
		return
	for output_format in [HALF, SRGB]:
		for history_format in [HALF, SRGB]:
			var key := format_label(output_format, history_format)
			format_coverage[key] = 0
			for tonemap in [false, true]:
				for reset in [false, true]:
					for kind in CASES:
						test_formats(kind, reset, tonemap, output_format, history_format)
						if observer.count() != 0:
							finish()
							return
					test_motion_adapter(reset, tonemap, output_format, history_format)
					if observer.count() != 0:
						finish()
						return
				test_feedback(tonemap, output_format, history_format)
				if observer.count() != 0:
					finish()
					return
	finish()


func compile_shaders() -> bool:
	var vertex := FileAccess.get_file_as_string("res://shaders/bloom_fullscreen.glsl")
	var production := FileAccess.get_file_as_string("res://shaders/taa_resolve.glsl")
	var legacy := FileAccess.get_file_as_string("res://tests/taa_legacy_raster.glsl")
	if vertex.is_empty() or production.is_empty() or legacy.is_empty():
		push_error("Missing TAA raster/legacy/fullscreen source")
		return false
	vertex = vertex.replace("#[vertex]\n", "")
	production = production.replace("#[compute]\n", "")
	var source := RDShaderSource.new()
	source.source_vertex = vertex
	var stages: Array[int] = [
		RenderingDevice.SHADER_STAGE_VERTEX, RenderingDevice.SHADER_STAGE_FRAGMENT
	]
	source.source_fragment = INPUT_SAMPLE
	input_shader = compile_source(source, stages)
	if not input_shader.is_valid():
		return false
	source.source_fragment = production.replace(
		"#version 450\n", "#version 450\n#define TAA_RASTER\n"
	)
	production_shader = compile_source(source, stages)
	if not production_shader.is_valid():
		return false
	source.source_fragment = production.replace(
		"#version 450\n", "#version 450\n#define TAA_RASTER\n#define TAA_GODOT_MOTION\n"
	)
	motion_shader = compile_source(source, stages)
	if not motion_shader.is_valid():
		return false
	for defines in ["", "#define RESET\n", "#define TONEMAP\n", "#define RESET\n#define TONEMAP\n"]:
		source = RDShaderSource.new()
		source.source_vertex = vertex
		source.source_fragment = legacy.replace("#version 450\n", "#version 450\n" + defines)
		var shader := compile_source(source, stages)
		raster_shaders.append(shader)
		if not shader.is_valid():
			return false
	return true


func uniforms(inputs: Array[RID]) -> Array[RDUniform]:
	if not input_draw:
		return super.uniforms(inputs)
	var uniform := RDUniform.new()
	uniform.binding = 0
	uniform.uniform_type = RenderingDevice.UNIFORM_TYPE_SAMPLER_WITH_TEXTURE
	uniform.add_id(nearest)
	uniform.add_id(inputs[0])
	return [uniform]


func draw(shader: RID, inputs: Array[RID], outputs: Array[RID]) -> bool:
	input_draw = shader == input_shader
	motion_draw = shader == motion_shader
	production_draw = shader == production_shader or motion_draw
	return super.draw(shader, inputs, outputs)


func record_draw(framebuffer: RID, pipeline: RID, binding: RID) -> bool:
	var commands := rd.draw_list_begin(framebuffer, RenderingDevice.DRAW_DEFAULT_ALL)
	if commands == RenderingDevice.INVALID_ID:
		push_error("TAA format draw list begin failed")
		return false
	rd.draw_list_bind_render_pipeline(commands, pipeline)
	rd.draw_list_bind_uniform_set(commands, binding, 0)
	if production_draw:
		var push := PackedByteArray()
		push.resize(16 if motion_draw else 8)
		push.encode_u32(0, int(draw_reset))
		push.encode_u32(4, int(draw_tonemap))
		if motion_draw:
			push.encode_float(8, draw_jitter.x)
			push.encode_float(12, draw_jitter.y)
		rd.draw_list_set_push_constant(commands, push, push.size())
	rd.draw_list_draw(commands, false, 1, 3)
	rd.draw_list_end()
	rd.submit()
	rd.sync()
	return true


func format_label(output_format: int, history_format: int) -> String:
	return "resolved=%s history=%s" % [format_name(output_format), format_name(history_format)]


func format_name(value: int) -> String:
	if value == FLOAT:
		return "RGBA32F"
	return "RGBA16F" if value == HALF else "RGBA8_SRGB"


func encode_image(input: Image, format: int) -> Image:
	if format == HALF or format == FLOAT:
		var result := input.duplicate() as Image
		result.convert(Image.FORMAT_RGBAH if format == HALF else Image.FORMAT_RGBAF)
		return result
	var encoded := Image.create(input.get_width(), input.get_height(), false, Image.FORMAT_RGBA8)
	for y in range(input.get_height()):
		for x in range(input.get_width()):
			# Attachment/sample RGB is sRGB; alpha is always linear UNORM.
			encoded.set_pixel(x, y, input.get_pixel(x, y).linear_to_srgb())
	return encoded


func store_colour(value: Color, format: int) -> Color:
	if format == HALF:
		return image(Vector2i.ONE, value).get_pixel(0, 0)
	# Attachment goldens use nearest UNORM code, independently of upload conversion.
	var encoded := value.linear_to_srgb()
	for channel in range(4):
		encoded[channel] = float(roundi(clampf(encoded[channel], 0.0, 1.0) * 255.0)) / 255.0
	return encoded


func sample_uploaded_input(texture: RID) -> Image:
	# Identity only: same fullscreen UV and nearest/clamp sampler as TAA current.
	# RGBA32F retains decoded precision until CPU tone/blend math and final store.
	var output := format_outputs(FLOAT, FLOAT)
	var resources: Array[RID] = [texture]
	resources.append_array(output)
	if not all_valid(resources) or not draw(input_shader, [texture], output):
		return null
	return read_formatted(output[0], FLOAT)


func sample_expected_history(value: Color, format: int) -> Image:
	# Decode independently predicted attachment bytes, NEVER a TAA output RID.
	# Fill the stored format directly: no HALF intermediate or upload re-encode.
	var stored: Image
	if format == HALF:
		stored = image(SIZE, value)
	else:
		var pixel := encoded_pixel(store_colour(value, format)).get_data()
		var bytes := PackedByteArray()
		for index in range(SIZE.x * SIZE.y):
			bytes.append_array(pixel)
		stored = Image.create_from_data(SIZE.x, SIZE.y, false, Image.FORMAT_RGBA8, bytes)
	return sample_uploaded_input(create_uploaded_texture(stored, format))


func create_formatted_texture(input: Image, format: int) -> RID:
	return create_uploaded_texture(encode_image(input, format), format)


func create_uploaded_texture(input: Image, format: int) -> RID:
	var description := RDTextureFormat.new()
	description.width = input.get_width()
	description.height = input.get_height()
	description.format = format
	description.usage_bits = (
		RenderingDevice.TEXTURE_USAGE_SAMPLING_BIT
		| RenderingDevice.TEXTURE_USAGE_COLOR_ATTACHMENT_BIT
		| RenderingDevice.TEXTURE_USAGE_CAN_COPY_FROM_BIT
	)
	if not rd.texture_is_format_supported_for_usage(format, description.usage_bits):
		push_error("TAA unsupported sampling/MRT/readback format " + format_name(format))
		return RID()
	var bytes: Array[PackedByteArray] = [input.get_data()]
	var texture := rd.texture_create(description, RDTextureView.new(), bytes)
	textures.append(texture)
	if not texture.is_valid():
		push_error("TAA texture creation failed: " + format_name(format))
	return texture


func read_formatted(texture: RID, format: int) -> Image:
	var bytes := rd.texture_get_data(texture, 0)
	var stride := 16 if format == FLOAT else (8 if format == HALF else 4)
	if bytes.size() != SIZE.x * SIZE.y * stride:
		push_error("TAA format readback size mismatch " + format_name(format))
		return null
	var image_format := (
		Image.FORMAT_RGBAF
		if format == FLOAT
		else (Image.FORMAT_RGBAH if format == HALF else Image.FORMAT_RGBA8)
	)
	return Image.create_from_data(SIZE.x, SIZE.y, false, image_format, bytes)


func compare_formatted(actual: Image, expected: Image, format: int, label: String) -> void:
	if actual == null or expected == null:
		return
	if format == HALF:
		compare(actual, expected, label)
		return
	var observed := actual.get_data()
	var wanted := expected.get_data()
	for index in range(wanted.size()):
		comparisons += 1
		if absi(int(observed[index]) - int(wanted[index])) > 1:
			failures += 1
			if failures <= 32:
				push_error(
					(
						"%s byte%d: %d expected %d (fixed <=1 code)"
						% [label, index, observed[index], wanted[index]]
					)
				)


func check_stored(actual: Color, linear_value: Color, format: int, label: String) -> void:
	var wanted := store_colour(linear_value, format)
	if format == HALF:
		compare(image(Vector2i.ONE, actual), image(Vector2i.ONE, wanted), label)
	else:
		# Recover encoded codes directly; no HALF intermediate or second sRGB encode.
		compare_formatted(encoded_pixel(actual), encoded_pixel(wanted), format, label)


func encoded_pixel(stored: Color) -> Image:
	var bytes := PackedByteArray()
	for channel in range(4):
		bytes.append(roundi(clampf(stored[channel], 0.0, 1.0) * 255.0))
	return Image.create_from_data(1, 1, false, Image.FORMAT_RGBA8, bytes)


func mixed_current(opaque := false) -> Image:
	var current := image(SIZE, Color())
	for y in range(SIZE.y):
		for x in range(SIZE.x):
			# Four black + four white + center .5: mean=.5, sigma=sqrt(2/9).
			# History .25 is inside the bound, so confidence changes blend visibly.
			var value := 0.5 if x == 1 and y == 1 else float((x + 3 * y) % 2)
			current.set_pixel(
				x, y, Color(value, value, value, 1.0 if opaque else float(x + 3 * y) / 8.0)
			)
	return current


func format_inputs(current: Image, past: RID, output_format: int, move: Vector2) -> Array[RID]:
	return [
		create_formatted_texture(current, output_format),
		past,
		past,
		create_formatted_texture(image(SIZE, Color(0.25, 0, 0, 0)), HALF),
		create_formatted_texture(image(SIZE, Color(move.x, move.y, 0, 0)), HALF)
	]


func format_outputs(output_format: int, history_format: int) -> Array[RID]:
	return [
		create_formatted_texture(image(SIZE, Color()), output_format),
		create_formatted_texture(image(SIZE, Color()), history_format)
	]


func draw_pair(
	inputs: Array[RID],
	oracle_inputs: Array[RID],
	output: Array[RID],
	oracle: Array[RID],
	reset: bool,
	tonemap: bool,
	output_format: int,
	history_format: int,
	label: String
) -> bool:
	if not all_valid(inputs + oracle_inputs + output + oracle):
		return false
	draw_reset = reset
	draw_tonemap = tonemap
	if not draw(motion_shader if motion_adapter else production_shader, inputs, output):
		return false
	if not draw(raster_shaders[int(reset) + 2 * int(tonemap)], oracle_inputs, oracle):
		return false
	fixtures += 1
	format_coverage[format_label(output_format, history_format)] += 1
	for index in range(2):
		var format := output_format if index == 0 else history_format
		compare_formatted(
			read_formatted(output[index], format),
			read_formatted(oracle[index], format),
			format,
			label + " attachment%d" % index
		)
	print("TAA_RASTER_FORMAT ", label)
	return true


func check_stores(
	output: Array[RID],
	current: RID,
	reset: bool,
	tonemap: bool,
	confidence: float,
	output_format: int,
	history_format: int,
	label: String
) -> Image:
	var resolved := read_formatted(output[0], output_format)
	var history := read_formatted(output[1], history_format)
	var source := read_formatted(current, output_format)
	var decoded := sample_uploaded_input(current)
	if resolved == null or history == null or source == null or decoded == null:
		return null
	if reset and output_format == SRGB:
		var bytes := source.get_data()
		var offset := (1 + SIZE.x) * 4
		var measured := decoded.get_pixel(1, 1)
		var ideal := source.get_pixel(1, 1).srgb_to_linear()
		print(
			"TAA_INPUT_BOUNDARY ",
			label,
			" pixel=(1,1) uploaded_rgb_bytes=",
			[bytes[offset], bytes[offset + 1], bytes[offset + 2]],
			(
				" decoded_current_f32=(%.9f,%.9f,%.9f) CPU_ideal=(%.9f,%.9f,%.9f)"
				% [measured.r, measured.g, measured.b, ideal.r, ideal.g, ideal.b]
			)
		)
	for y in range(SIZE.y):
		for x in range(SIZE.x):
			var original := decoded.get_pixel(x, y)
			check_value(
				resolved.get_pixel(x, y).a,
				source.get_pixel(x, y).a,
				label + " source alpha",
				x,
				y,
				3,
				true
			)
			var stored_confidence := store_colour(Color(0, 0, 0, confidence), history_format).a
			check_value(
				history.get_pixel(x, y).a, stored_confidence, label + " confidence", x, y, 3, true
			)
			if reset:
				check_stored(
					resolved.get_pixel(x, y), original, output_format, label + " reset resolved"
				)
				var wanted := original
				if tonemap:
					wanted = map_colour(wanted)
				wanted.a = confidence
				check_stored(
					history.get_pixel(x, y), wanted, history_format, label + " reset history"
				)
	return decoded


func map_colour(value: Color) -> Color:
	var divisor := 1.0 + maxf(value.r, maxf(value.g, value.b))
	return Color(value.r / divisor, value.g / divisor, value.b / divisor, value.a)


func test_formats(
	kind: String, reset: bool, tonemap: bool, output_format: int, history_format: int
) -> void:
	var label := (
		"%s %s reset=%s tonemap=%s"
		% [format_label(output_format, history_format), kind, reset, tonemap]
	)
	var current := mixed_current(kind == "opaque")
	if kind == "fractional":
		# Non-gray, nonuniform inputs exercise all channels and filtered reprojection.
		for y in range(SIZE.y):
			for x in range(SIZE.x):
				current.set_pixel(
					x, y, Color(float(x) * 1.5, float(y) * 0.75, float(x + y) / 4.0, 0.375)
				)
	var initial_confidence := 66.65625 if kind == "mixed-high" else 1.0
	var past_image := image(SIZE, Color(0.25, 0.25, 0.25, initial_confidence))
	if kind == "fractional":
		for y in range(SIZE.y):
			for x in range(SIZE.x):
				past_image.set_pixel(
					x, y, Color(float(x + 1) / 4.0, float(y + 1) / 4.0, float(x + y + 1) / 8.0, 1)
				)
	var past := create_formatted_texture(past_image, history_format)
	var move := Vector2.ZERO
	if kind == "fractional":
		move = Vector2(0.37 / 3.0, -0.23 / 3.0)
	elif kind == "offscreen":
		move = Vector2(2, -2)
	var inputs := format_inputs(current, past, output_format, move)
	if kind == "offscreen":
		var motion := image(SIZE, Color())
		var directions := [Vector2(2, 0), Vector2(-2, 0), Vector2(0, 2), Vector2(0, -2)]
		for y in range(SIZE.y):
			for x in range(SIZE.x):
				var direction: Vector2 = directions[(x + 3 * y) % 4]
				motion.set_pixel(x, y, Color(direction.x, direction.y, 0, 0))
		inputs[4] = create_formatted_texture(motion, HALF)
	var output := format_outputs(output_format, history_format)
	var oracle := format_outputs(output_format, history_format)
	if draw_pair(
		inputs, inputs, output, oracle, reset, tonemap, output_format, history_format, label
	):
		var confidence := (
			1.0 / 0.015
			if reset
			else (
				1.0
				if move != Vector2.ZERO
				else store_colour(Color(0, 0, 0, initial_confidence), history_format).a + 10.0
			)
		)
		var decoded := check_stores(
			output,
			inputs[0],
			reset or kind == "offscreen",
			tonemap,
			confidence,
			output_format,
			history_format,
			label
		)
		if not reset and kind in ["mixed-low", "mixed-high", "opaque"]:
			var decoded_history := sample_uploaded_input(past)
			if decoded == null or decoded_history == null:
				release_textures()
				return
			var history_colour := decoded_history.get_pixel(1, 1)
			var center := decoded.get_pixel(1, 1)
			if tonemap:
				center = map_colour(center)
			var blend := clampf(1.0 / confidence, 0.015, 0.1)
			var expected := history_colour.lerp(center, blend)
			expected.a = confidence
			check_stored(
				read_formatted(output[1], history_format).get_pixel(1, 1),
				expected,
				history_format,
				label + " center blend"
			)
	release_textures()


func test_motion_adapter(
	reset: bool, tonemap: bool, output_format: int, history_format: int
) -> void:
	var label := (
		"%s Godot-motion reset=%s tonemap=%s"
		% [format_label(output_format, history_format), reset, tonemap]
	)
	var current := mixed_current()
	var history := image(SIZE, Color())
	for y in range(SIZE.y):
		for x in range(SIZE.x):
			history.set_pixel(
				x, y, Color(float(x + 1) / 4.0, float(y + 1) / 4.0, float(x + y + 1) / 8.0, 1)
			)
	var past := create_formatted_texture(history, history_format)
	# Dyadic values remain exact through HALF upload. Independent legacy receives
	# already converted motion; production receives raw motion plus 16-byte ABI.
	var raw := Vector2(0.0625, -0.125)
	draw_jitter = Vector2(0.03125, 0.0625)
	var inputs := format_inputs(current, past, output_format, raw)
	var corrected := -raw + draw_jitter
	var oracle_inputs: Array[RID] = [
		inputs[0],
		past,
		past,
		inputs[3],
		create_formatted_texture(image(SIZE, Color(corrected.x, corrected.y, 0, 0)), HALF)
	]
	var output := format_outputs(output_format, history_format)
	var oracle := format_outputs(output_format, history_format)
	motion_adapter = true
	if draw_pair(
		inputs, oracle_inputs, output, oracle, reset, tonemap, output_format, history_format, label
	):
		check_stores(
			output,
			inputs[0],
			reset,
			tonemap,
			1.0 / 0.015 if reset else 1.0,
			output_format,
			history_format,
			label
		)
	motion_adapter = false
	draw_jitter = Vector2.ZERO
	release_textures()


func test_feedback(tonemap: bool, output_format: int, history_format: int) -> void:
	var past := create_formatted_texture(image(SIZE, Color(0.25, 0.25, 0.25, 1)), history_format)
	var oracle_past := create_formatted_texture(
		image(SIZE, Color(0.25, 0.25, 0.25, 1)), history_format
	)
	var expected_center := Color()
	for frame in range(4):
		var reset := frame == 0
		var current := image(SIZE, Color(0.25, 0.25, 0.25, 0.375)) if reset else mixed_current()
		var label := (
			"%s feedback%d reset=%s tonemap=%s"
			% [format_label(output_format, history_format), frame, reset, tonemap]
		)
		var inputs := format_inputs(current, past, output_format, Vector2.ZERO)
		var oracle_inputs: Array[RID] = [inputs[0], oracle_past, oracle_past, inputs[3], inputs[4]]
		var output := format_outputs(output_format, history_format)
		var oracle := format_outputs(output_format, history_format)
		if not draw_pair(
			inputs,
			oracle_inputs,
			output,
			oracle,
			reset,
			tonemap,
			output_format,
			history_format,
			label
		):
			break
		var confidence := 1.0 / 0.015 if reset else expected_center.a + 10.0
		var decoded := check_stores(
			output, inputs[0], reset, tonemap, confidence, output_format, history_format, label
		)
		if decoded == null:
			break
		var center := decoded.get_pixel(1, 1)
		if tonemap:
			center = map_colour(center)
		var next := (
			center if reset else expected_center.lerp(center, clampf(1.0 / confidence, 0.015, 0.1))
		)
		next.a = confidence
		check_stored(
			read_formatted(output[1], history_format).get_pixel(1, 1),
			next,
			history_format,
			label + " feedback center"
		)
		if frame < 3:
			var decoded_history := sample_expected_history(next, history_format)
			if decoded_history == null:
				break
			expected_center = decoded_history.get_pixel(1, 1)
		# Differential histories stay GPU-resident; CPU goldens decode only predicted bytes.
		past = output[1]
		oracle_past = oracle[1]
	release_textures()


func finish() -> void:
	if finished:
		return
	finished = true
	if rd != null:
		release_textures()
		for shader in raster_shaders + [production_shader, motion_shader, input_shader]:
			if shader.is_valid():
				rd.free_rid(shader)
		for sampler in [nearest, linear]:
			if sampler.is_valid():
				rd.free_rid(sampler)
		rd.free()
		rd = null
	var coverage_ok := format_coverage.size() == 4
	for key in format_coverage:
		print("TAA_RASTER_COVERAGE ", key, " fixtures=", format_coverage[key], " expected=32")
		coverage_ok = coverage_ok and format_coverage[key] == 32
	var errors := observer.count()
	OS.remove_logger(observer)
	print(
		(
			"TAA raster formats: %d fixtures, %d comparisons, %d failures, %d engine errors"
			% [fixtures, comparisons, failures, errors]
		)
	)
	quit(
		0 if failures == 0 and errors == 0 and fixtures == EXPECTED_FIXTURES and coverage_ok else 1
	)
