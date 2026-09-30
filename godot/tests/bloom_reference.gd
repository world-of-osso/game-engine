extends RefCounted
## Test-only CPU oracle for the project's uniform-scale, additive Bevy bloom.
## Linear RGB floating-point input; full-image viewport. No native extension.
## Port of Bevy 0.19.0 src/bloom/{bloom.wgsl,mod.rs,settings.rs,
## downsampling_pipeline.rs,upsampling_pipeline.rs}, with project overrides from
## src/rendering/camera/camera_post_process.rs (intensity defaults to .08).
##
## Algorithm oracle, NOT bit-exact GPU emulation: RGBAF stores f32 channels,
## GDScript scalar arithmetic is f64; upstream intermediates are Rg11b10Ufloat.
## No packed-format rounding, GPU interpolation precision, MSAA resolve,
## subviewport, tonemapping, nonuniform scale, or energy-conserving mode.
## max_mip_dimension overrides exist only to bound CPU fixtures; default 512
## preserves OLD_SCHOOL sizing. Composite requires >=2 mips (max dimension >=8);
## upstream's one-mip blend has a zero denominator, outside this configured slice.
##
## Portions adapted from Bevy (Bevy contributors), distributed under MIT OR
## Apache-2.0. This port uses the MIT option; upstream LICENSE-MIT follows:
##
## MIT License
##
## Permission is hereby granted, free of charge, to any person obtaining a copy
## of this software and associated documentation files (the "Software"), to deal
## in the Software without restriction, including without limitation the rights
## to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
## copies of the Software, and to permit persons to whom the Software is
## furnished to do so, subject to the following conditions:
##
## The above copyright notice and this permission notice shall be included in all
## copies or substantial portions of the Software.
##
## THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
## IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
## FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
## AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
## LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
## OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
## SOFTWARE.

const THRESHOLD := 0.65
const SOFTNESS := 0.1
const INTENSITY := 0.08
const MAX_MIP_DIMENSION := 512
const OFFSETS_13 := [
	Vector2(-2, 2),
	Vector2(0, 2),
	Vector2(2, 2),
	Vector2(-2, 0),
	Vector2(0, 0),
	Vector2(2, 0),
	Vector2(-2, -2),
	Vector2(0, -2),
	Vector2(2, -2),
	Vector2(-1, 1),
	Vector2(1, 1),
	Vector2(-1, -1),
	Vector2(1, -1),
]
const OFFSETS_9 := [
	Vector2(-1, 1),
	Vector2(0, 1),
	Vector2(1, 1),
	Vector2(-1, 0),
	Vector2(0, 0),
	Vector2(1, 0),
	Vector2(-1, -1),
	Vector2(0, -1),
	Vector2(1, -1),
]


static func mip_sizes(
	viewport: Vector2i, max_mip_dimension := MAX_MIP_DIMENSION
) -> Array[Vector2i]:
	assert(
		viewport.x > 0 and viewport.y > 0 and max_mip_dimension > 0,
		"Bloom extents must be positive"
	)
	# Integer ilog2(max).max(2)-1, independent of screen resolution.
	var level := 0
	var dimension := max_mip_dimension
	while dimension > 1:
		dimension >>= 1
		level += 1
	var count := maxi(level, 2) - 1
	var ratio := float(max_mip_dimension) / viewport.y
	var base := Vector2i(maxi(1, roundi(viewport.x * ratio)), max_mip_dimension)
	var sizes: Array[Vector2i] = []
	for mip in range(count):
		sizes.append(Vector2i(maxi(1, base.x >> mip), maxi(1, base.y >> mip)))
	return sizes


static func soft_threshold(color: Vector3, threshold := THRESHOLD, softness := SOFTNESS) -> Vector3:
	var knee := threshold * clampf(softness, 0.0, 1.0)
	var brightness := maxf(color.x, maxf(color.y, color.z))
	var soft := clampf(brightness - (threshold - knee), 0.0, 2.0 * knee)
	soft = soft * soft * (0.25 / (knee + 0.00001))
	var contribution := maxf(brightness - threshold, soft) / maxf(brightness, 0.00001)
	return color * contribution


static func sample_bilinear(image: Image, uv: Vector2) -> Vector3:
	# Normalized texture UV maps texel centres to (i+.5)/extent. Clamp each
	# contributing texel, not the fractional part; implements ClampToEdge.
	var position := uv * Vector2(image.get_size()) - Vector2(0.5, 0.5)
	var x := floori(position.x)
	var y := floori(position.y)
	var fraction := position - Vector2(x, y)
	var top := _texel(image, x, y).lerp(_texel(image, x + 1, y), fraction.x)
	var bottom := _texel(image, x, y + 1).lerp(_texel(image, x + 1, y + 1), fraction.x)
	return top.lerp(bottom, fraction.y)


static func _texel(image: Image, x: int, y: int) -> Vector3:
	var color := image.get_pixel(
		clampi(x, 0, image.get_width() - 1), clampi(y, 0, image.get_height() - 1)
	)
	return Vector3(color.r, color.g, color.b)


static func _sample_offsets(image: Image, uv: Vector2, offsets: Array) -> Array[Vector3]:
	var samples: Array[Vector3] = []
	var texel_size := Vector2.ONE / Vector2(image.get_size())
	for offset in offsets:
		samples.append(sample_bilinear(image, uv + offset * texel_size))
	return samples


static func _karis(group: Vector3) -> Vector3:
	var luma := group.dot(Vector3(0.2126, 0.7152, 0.0722)) / 4.0
	return group * (1.0 / (1.0 + luma))


static func _filter_13(image: Image, uv: Vector2, first: bool) -> Vector3:
	var p := _sample_offsets(image, uv, OFFSETS_13)
	if first:
		var group0 := (p[0] + p[1] + p[3] + p[4]) * (0.125 / 4.0)
		var group1 := (p[1] + p[2] + p[4] + p[5]) * (0.125 / 4.0)
		var group2 := (p[3] + p[4] + p[6] + p[7]) * (0.125 / 4.0)
		var group3 := (p[4] + p[5] + p[7] + p[8]) * (0.125 / 4.0)
		var group4 := (p[9] + p[10] + p[11] + p[12]) * (0.5 / 4.0)
		return _karis(group0) + _karis(group1) + _karis(group2) + _karis(group3) + _karis(group4)
	var sample := (p[0] + p[2] + p[6] + p[8]) * 0.03125
	sample += (p[1] + p[3] + p[5] + p[7]) * 0.0625
	sample += (p[4] + p[9] + p[10] + p[11] + p[12]) * 0.125
	return sample


static func downsample(
	image: Image, size: Vector2i, first := false, threshold := THRESHOLD, softness := SOFTNESS
) -> Image:
	assert(
		size.x > 0 and size.y > 0 and not image.is_empty(), "Downsample requires nonempty images"
	)
	var output := Image.create(size.x, size.y, false, Image.FORMAT_RGBAF)
	for y in range(size.y):
		for x in range(size.x):
			var uv := (Vector2(x, y) + Vector2(0.5, 0.5)) / Vector2(size)
			var sample := _filter_13(image, uv, first)
			if first:
				sample = sample.clamp(Vector3.ONE * 0.0001, Vector3.ONE * 3.40282347e37)
				if threshold > 0.0:
					sample = soft_threshold(sample, threshold, softness)
			output.set_pixel(x, y, Color(sample.x, sample.y, sample.z, 1.0))
	return output


static func filtered_pyramid(image: Image, max_mip_dimension := MAX_MIP_DIMENSION) -> Array[Image]:
	var pyramid: Array[Image] = []
	var input := image
	for size in mip_sizes(image.get_size(), max_mip_dimension):
		input = downsample(input, size, pyramid.is_empty())
		pyramid.append(input)
	return pyramid


static func blend_factor(mip: int, max_mip: int, intensity := INTENSITY) -> float:
	assert(max_mip > 0 and mip >= 0 and mip <= max_mip, "Blend requires at least two mips")
	var frequency := float(mip) / max_mip
	var boost := (1.0 - pow(1.0 - frequency, 1.0 / (1.0 - 0.95))) * 0.7
	var high_pass := 1.0 - clampf((frequency - 1.0) / 1.0, 0.0, 1.0)
	return (intensity + boost) * high_pass


static func _filter_tent(image: Image, uv: Vector2) -> Vector3:
	var p := _sample_offsets(image, uv, OFFSETS_9)
	var sample := p[4] * 0.25
	sample += (p[1] + p[3] + p[5] + p[7]) * 0.125
	sample += (p[0] + p[2] + p[6] + p[8]) * 0.0625
	return sample


static func upsample_add(input: Image, destination: Image, blend: float) -> Image:
	var size := destination.get_size()
	var output := Image.create(size.x, size.y, false, Image.FORMAT_RGBAF)
	for y in range(size.y):
		for x in range(size.x):
			var uv := (Vector2(x, y) + Vector2(0.5, 0.5)) / Vector2(size)
			var original := destination.get_pixel(x, y)
			var sample := (
				_filter_tent(input, uv) * blend + Vector3(original.r, original.g, original.b)
			)
			output.set_pixel(x, y, Color(sample.x, sample.y, sample.z, original.a))
	return output


static func composite(original: Image, pyramid: Array, intensity := INTENSITY) -> Image:
	intensity = clampf(intensity, 0.0, 1.0)
	if intensity == 0.0:
		return original.duplicate()
	assert(pyramid.size() >= 2, "Configured additive bloom requires at least two mips")
	var last := pyramid.size() - 1
	var accumulated: Image = pyramid[last]
	# LoadOp::Load and dst_factor=One: each destination remains intact and
	# receives the filtered, already accumulated lower-resolution source.
	for mip in range(last, 0, -1):
		accumulated = upsample_add(
			accumulated, pyramid[mip - 1], blend_factor(mip, last, intensity)
		)
	return upsample_add(accumulated, original, blend_factor(0, last, intensity))


static func render(
	original: Image, intensity := INTENSITY, max_mip_dimension := MAX_MIP_DIMENSION
) -> Image:
	if clampf(intensity, 0.0, 1.0) == 0.0:
		return original.duplicate()
	return composite(original, filtered_pyramid(original, max_mip_dimension), intensity)
