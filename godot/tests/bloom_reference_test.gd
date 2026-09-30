extends SceneTree
## CPU-only numerical test. No project import, native extension, GPU, or user settings.
## Run from checkout:
## Godot --headless --path godot/tests --script bloom_reference_test.gd

var reference: Script
var failures := 0
var checks := 0


func _initialize() -> void:
	var path: String = get_script().resource_path.get_base_dir().path_join("bloom_reference.gd")
	if not FileAccess.file_exists(path):
		push_error("Missing bloom CPU reference: " + path)
		quit(1)
		return
	reference = load(path)
	if reference == null or not reference.can_instantiate():
		push_error("Bloom CPU reference failed to load")
		quit(1)
		return
	test_threshold()
	test_bilinear()
	test_mip_sizes()
	test_first_pass()
	test_constant_pyramid()
	test_impulse()
	print("Bloom reference: %d checks, %d failures" % [checks, failures])
	quit(0 if failures == 0 else 1)


func expect_close(actual: float, expected: float, label: String, tolerance := 0.000002) -> void:
	checks += 1
	if not is_finite(actual) or absf(actual - expected) > tolerance:
		failures += 1
		push_error("%s: got %.10f, expected %.10f" % [label, actual, expected])


func expect_true(value: bool, label: String) -> void:
	checks += 1
	if not value:
		failures += 1
		push_error(label)


func field(size: Vector2i, color: Color) -> Image:
	var image := Image.create(size.x, size.y, false, Image.FORMAT_RGBAF)
	image.fill(color)
	return image


func expect_field(image: Image, expected: Color, label: String) -> void:
	for y in range(image.get_height()):
		for x in range(image.get_width()):
			var color := image.get_pixel(x, y)
			for channel in range(4):
				expect_close(
					color[channel],
					expected[channel],
					"%s (%d,%d) channel %d" % [label, x, y, channel]
				)


func test_threshold() -> void:
	# knee=.065; quadratic divisor=4*(.065+.00001). Expected values
	# derived directly from that scalar curve, not from another reference call.
	for entry in [
		[0.0, 0.0],
		[0.5, 0.0],
		[0.585, 0.0],
		[0.65, 0.01624750038455625],
		[0.7, 0.05085756037532686],
		[0.715, 0.065],
		[1.0, 0.35]
	]:
		var value: Vector3 = reference.soft_threshold(Vector3.ONE * entry[0])
		expect_close(value.x, entry[1], "threshold %.3f" % entry[0])
	var color: Vector3 = reference.soft_threshold(Vector3(1.0, 0.5, 0.25))
	expect_close(color.y, 0.175, "threshold uses maximum RGB, retains colour ratio")
	expect_close(color.z, 0.0875, "threshold blue ratio")
	var hard: Vector3 = reference.soft_threshold(Vector3.ONE * 0.7, 0.65, -2.0)
	expect_close(hard.x, 0.05, "negative softness clamps to hard threshold")
	var soft: Vector3 = reference.soft_threshold(Vector3.ONE * 0.65, 0.65, 2.0)
	expect_close(soft.x, 0.162497500038461, "softness clamps at one")


func test_bilinear() -> void:
	var image := field(Vector2i(2, 2), Color(0, 0, 0, 0.3))
	image.set_pixel(1, 0, Color(2, 0, 0, 0.3))
	image.set_pixel(0, 1, Color(4, 0, 0, 0.3))
	image.set_pixel(1, 1, Color(6, 0, 0, 0.3))
	for entry in [
		[Vector2(0.25, 0.25), 0.0],
		[Vector2(0.75, 0.75), 6.0],
		[Vector2(0.5, 0.5), 3.0],
		[Vector2(0.375, 0.625), 3.5],
		[Vector2(-3, 4), 4.0],
		[Vector2(1, 0), 2.0]
	]:
		var sample: Vector3 = reference.sample_bilinear(image, entry[0])
		expect_close(sample.x, entry[1], "bilinear texel centres/clamped edge %s" % entry[0])


func test_mip_sizes() -> void:
	# prepare_bloom_textures uses round(viewport*512/height), NOT half screen.
	var landscape: Array = reference.mip_sizes(Vector2i(1920, 1080))
	expect_true(
		(
			landscape
			== [
				Vector2i(910, 512),
				Vector2i(455, 256),
				Vector2i(227, 128),
				Vector2i(113, 64),
				Vector2i(56, 32),
				Vector2i(28, 16),
				Vector2i(14, 8),
				Vector2i(7, 4)
			]
		),
		"default 512 eight-level landscape extent"
	)
	var portrait: Array = reference.mip_sizes(Vector2i(1, 1024))
	expect_true(
		portrait[0] == Vector2i(1, 512) and portrait[7] == Vector2i(1, 4),
		"rounded width and mip width clamp to one"
	)
	var bounded: Array = reference.mip_sizes(Vector2i(5, 7), 16)
	expect_true(
		bounded == [Vector2i(11, 16), Vector2i(5, 8), Vector2i(2, 4)],
		"bounded three-level odd extent"
	)


func test_first_pass() -> void:
	var image := field(Vector2i(2, 2), Color(0, 0, 0, 1))
	image.set_pixel(1, 0, Color(2, 2, 2, 1))
	image.set_pixel(0, 1, Color(4, 4, 4, 1))
	image.set_pixel(1, 1, Color(6, 6, 6, 1))
	# At uv=(.5,.5), clamp-edge bilinear gives a..i=(4,5,6,2,3,4,0,1,2),
	# j..m=(4,6,0,2). Weighted groups=(7,9,3,5)/16 and 3/2.
	# Each grayscale group g contributes g/(1+g/4); total=645060092/263571099.
	# Total exceeds upper knee .715, so firstpass = total-.65.
	var first: Image = reference.downsample(image, Vector2i.ONE, true)
	expect_field(
		first,
		Color(1.797385523099405, 1.797385523099405, 1.797385523099405, 1),
		"hand-derived firstpass"
	)
	# Constant RGB=(1,.5,.25) has Rec.709 luma=.58825. Weighted Karis
	# red = 4*.125/(1+.125*.58825/4)+.5/(1+.5*.58825/4).
	# Threshold subtracts .65 from red while preserving channel ratios.
	var colored := field(Vector2i(3, 3), Color(1, 0.5, 0.25, 1))
	var colored_first: Image = reference.downsample(colored, Vector2i.ONE, true)
	expect_field(
		colored_first,
		Color(0.30672713481889624, 0.15336356740944812, 0.07668178370472406, 1),
		"colored firstpass Rec.709 Karis weighting"
	)
	var plain: Image = reference.downsample(image, Vector2i.ONE)
	expect_field(plain, Color(3, 3, 3, 1), "ordinary normalized 13-tap, no Karis or threshold")
	var black := field(Vector2i(3, 3), Color(0, 0, 0, 0.4))
	var floor_pass: Image = reference.downsample(black, Vector2i.ONE, true, 0.0)
	expect_field(
		floor_pass, Color(0.0001, 0.0001, 0.0001, 1), "firstpass floor with threshold disabled"
	)


func test_constant_pyramid() -> void:
	var source := field(Vector2i(7, 5), Color(1, 1, 1, 0.37))
	var pyramid: Array = reference.filtered_pyramid(source, 16)
	# Four groups .125/(1+.125/4), one .5/(1+.5/4), then subtract .65.
	# Unit-sum subsequent kernels preserve f=4*(4/33)+4/9-.65.
	for mip in pyramid:
		expect_field(
			mip,
			Color(0.2792929292929293, 0.2792929292929293, 0.2792929292929293, 1),
			"constant filtered mip"
		)
	# Three mips: b2=.78, b1=.08+.7*(1-2^-20), b0=.08.
	# Recursive accumulation is f*[1+b1*(1+b2)], NOT f*(1+b1+b2).
	var output: Image = reference.composite(source, pyramid)
	expect_field(
		output,
		Color(1.0533650320356427, 1.0533650320356427, 1.0533650320356427, 0.37),
		"recursive additive composite"
	)
	expect_field(source, Color(1, 1, 1, 0.37), "original unchanged")
	expect_field(
		pyramid[0],
		Color(0.2792929292929293, 0.2792929292929293, 0.2792929292929293, 1),
		"filtered pyramid unchanged"
	)
	# Exercise actual OLD_SCHOOL 512 sizing and all eight blend stages without
	# a large CPU frame: a 1x1024 input produces widths of one throughout.
	# f above, b_i=.08+.7*(1-(1-i/7)^20), i=1..7;
	# result=1+.08*f*(1+b_1*(1+b_2*(...*(1+b_7)))).
	var full_depth := field(Vector2i(1, 1024), Color(1, 1, 1, 0.37))
	var default_output: Image = reference.render(full_depth)
	expect_field(
		default_output,
		Color(1.0849110584191668, 1.0849110584191668, 1.0849110584191668, 0.37),
		"default 512 eight-level recursive composite"
	)
	var bypass: Image = reference.render(source, 0.0, 16)
	expect_field(bypass, Color(1, 1, 1, 0.37), "intensity zero skips low-frequency boost too")
	for value in [0.0, 0.5]:
		var dim := field(Vector2i(3, 3), Color(value, value, value, 0.6))
		var result: Image = reference.render(dim, 0.08, 16)
		expect_field(result, Color(value, value, value, 0.6), "black/below-knee frame unchanged")
	var stronger: Image = reference.render(source, 0.16, 16)
	# b1=.8599993324279785,b2=.86; b0=.16. Intensity changes every stage.
	expect_close(
		stronger.get_pixel(0, 0).r,
		1.1161679283514159,
		"nondefault intensity affects recursive weights"
	)


func test_impulse() -> void:
	var image := field(Vector2i(11, 11), Color(0, 0, 0, 0.2))
	image.set_pixel(5, 5, Color(16, 16, 16, 0.8))
	var first: Image = reference.downsample(image, Vector2i(11, 11), true)
	# Only e=16 is nonzero at centre: each of four groups =16/32=.5.
	# Four Karis-weighted groups -> 4*.5/(1+.5/4)=16/9; then threshold.
	expect_close(first.get_pixel(5, 5).r, 1.1277777777777778, "hand-derived impulse firstpass")
	for y in range(11):
		for x in range(11):
			var sample := first.get_pixel(x, y).r
			expect_true(is_finite(sample) and sample >= 0.0, "impulse finite nonnegative")
			expect_close(sample, first.get_pixel(10 - x, y).r, "impulse horizontal symmetry")
			expect_close(sample, first.get_pixel(x, 10 - y).r, "impulse vertical symmetry")
	# Separate known tent sample: centre weight 1/4, edge 1/8, corner 1/16.
	var tent: Image = reference.upsample_add(
		image, field(Vector2i(11, 11), Color(0, 0, 0, 0.4)), 0.5
	)
	expect_close(tent.get_pixel(5, 5).r, 2.0, "tent centre numeric weight")
	expect_close(tent.get_pixel(4, 5).r, 1.0, "tent axial numeric weight")
	expect_close(tent.get_pixel(4, 4).r, 0.5, "tent corner numeric weight")
	var output: Image = reference.render(image, 0.08, 16)
	for y in range(11):
		for x in range(11):
			var color := output.get_pixel(x, y)
			expect_true(
				is_finite(color.r) and color.r >= image.get_pixel(x, y).r,
				"impulse additive, finite"
			)
			expect_close(
				color.r, output.get_pixel(y, x).r, "full-pyramid impulse diagonal symmetry", 0.00001
			)
			expect_close(color.a, image.get_pixel(x, y).a, "composite retains destination alpha")
	expect_true(output.get_pixel(4, 5).r > 0.0, "impulse spreads outside original pixel")
