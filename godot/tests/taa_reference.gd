extends RefCounted
## Test-only independent CPU translation of Bevy 0.19.0 src/taa/taa.wgsl.
## Fields: {extent: Vector2i, pixels: row-major Array[Vector4]}.
## Ideal nearest/bilinear clamp-to-edge sampling; no GPU/format quantization,
## sRGB decode/encode or confidence storage clamp. Vector math uses Godot floats,
## scalar math uses f64. Algorithm oracle, NOT bit-exact GPU/history-format parity.
## Motion already uses Bevy current-minus-previous UV. History extent may differ.
## Bevy contributors, MIT OR Apache-2.0; this port uses MIT.
## YCoCg/AABB math originates from Playdead's MIT temporal shader.
## Permission is hereby granted, free of charge, to any person obtaining a copy
## of this software and associated documentation files (the "Software"), to deal
## in the Software without restriction, including without limitation the rights
## to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
## copies of the Software, and to permit persons to whom the Software is
## furnished to do so, subject to the following conditions:
## The above copyright notice and this permission notice shall be included in all
## copies or substantial portions of the Software.
## THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
## IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
## FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
## AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
## LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
## OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
## SOFTWARE.


static func rgb(value: Vector4) -> Vector3:
	return Vector3(value.x, value.y, value.z)


static func max3(value: Vector3) -> float:
	return maxf(value.x, maxf(value.y, value.z))


static func tonemap(value: Vector3) -> Vector3:
	return value * (1.0 / (max3(value) + 1.0))


static func reverse_tonemap(value: Vector3) -> Vector3:
	return value * (1.0 / (1.0 - max3(value)))


static func rgb_to_ycocg(value: Vector3) -> Vector3:
	return Vector3(
		value.x / 4.0 + value.y / 2.0 + value.z / 4.0,
		value.x / 2.0 - value.z / 2.0,
		-value.x / 4.0 + value.y / 2.0 - value.z / 4.0
	)


static func ycocg_to_rgb(value: Vector3) -> Vector3:
	return (
		Vector3(value.x + value.y - value.z, value.x + value.z, value.x - value.y - value.z)
		. clamp(Vector3.ZERO, Vector3.ONE)
	)


static func clip_to_box(value: Vector3, lower: Vector3, upper: Vector3) -> Vector3:
	var center := .5 * (upper + lower)
	var extent := .5 * (upper - lower) + Vector3.ONE * .00000001
	var delta := value - center
	var distance := max3((delta / extent).abs())
	if distance > 1.0:
		return center + delta / distance
	return value


static func fetch(field: Dictionary, pixel: Vector2i) -> Vector4:
	# These clamps emulate the sampler address mode, not extra shader colour guards.
	var extent: Vector2i = field.extent
	var x := clampi(pixel.x, 0, extent.x - 1)
	var y := clampi(pixel.y, 0, extent.y - 1)
	return field.pixels[y * extent.x + x]


static func nearest(field: Dictionary, uv: Vector2) -> Vector4:
	var position := uv * Vector2(field.extent)
	return fetch(field, Vector2i(floori(position.x), floori(position.y)))


static func linear(field: Dictionary, uv: Vector2) -> Vector3:
	var position := uv * Vector2(field.extent) - Vector2.ONE * .5
	var base := Vector2i(floori(position.x), floori(position.y))
	var fraction := position - Vector2(base)
	var top := rgb(fetch(field, base)).lerp(rgb(fetch(field, base + Vector2i(1, 0))), fraction.x)
	var bottom := rgb(fetch(field, base + Vector2i(0, 1))).lerp(
		rgb(fetch(field, base + Vector2i.ONE)), fraction.x
	)
	return top.lerp(bottom, fraction.y)


static func closest_motion(
	depth: Dictionary, motion: Dictionary, uv: Vector2, current_extent: Vector2
) -> Vector2:
	var closest_uv := uv
	var closest_depth := nearest(depth, uv).x
	# Strict max preserves center, TL, TR, BL, BR ties, at TWO texels.
	for offset in [Vector2(-2, 2), Vector2(2, 2), Vector2(-2, -2), Vector2(2, -2)]:
		var candidate: Vector2 = uv + offset / current_extent
		var value := nearest(depth, candidate).x
		if value > closest_depth:
			closest_uv = candidate
			closest_depth = value
	var selected := nearest(motion, closest_uv)
	return Vector2(selected.x, selected.y)


static func sample_history(history: Dictionary, uv: Vector2, current_extent: Vector2) -> Vector3:
	# Reconstruct weights in current texel space, sample history in its own extent.
	var position := uv * current_extent
	var center := (position - Vector2.ONE * .5).floor() + Vector2.ONE * .5
	var f := position - center
	var w0 := f * (Vector2.ONE * -.5 + f * (Vector2.ONE - .5 * f))
	var w1 := Vector2.ONE + f * f * (Vector2.ONE * -2.5 + 1.5 * f)
	var w2 := f * (Vector2.ONE * .5 + f * (Vector2.ONE * 2.0 - 1.5 * f))
	var w3 := f * f * (Vector2.ONE * -.5 + .5 * f)
	var w12 := w1 + w2
	var p0 := (center - Vector2.ONE) / current_extent
	var p3 := (center + Vector2.ONE * 2.0) / current_extent
	var p12 := (center + w2 / w12) / current_extent
	var value := linear(history, Vector2(p12.x, p0.y)) * w12.x * w0.y
	value += linear(history, Vector2(p0.x, p12.y)) * w0.x * w12.y
	value += linear(history, p12) * w12.x * w12.y
	value += linear(history, Vector2(p3.x, p12.y)) * w3.x * w12.y
	value += linear(history, Vector2(p12.x, p3.y)) * w12.x * w3.y
	return value


static func clip_history(
	history_color: Vector3, current: Dictionary, uv: Vector2, mapped_current: Vector3, hdr: bool
) -> Vector3:
	var moment1 := Vector3.ZERO
	var moment2 := Vector3.ZERO
	# Same summation order, center uses the original mapped current sample.
	for offset in [
		Vector2(-1, 1),
		Vector2(0, 1),
		Vector2(1, 1),
		Vector2(-1, 0),
		Vector2.ZERO,
		Vector2(1, 0),
		Vector2(-1, -1),
		Vector2(0, -1),
		Vector2(1, -1)
	]:
		var color := mapped_current
		if offset != Vector2.ZERO:
			color = rgb(nearest(current, uv + offset / Vector2(current.extent)))
			if hdr:
				color = tonemap(color)
		var sample := rgb_to_ycocg(color)
		moment1 += sample
		moment2 += sample * sample
	var mean := moment1 / 9.0
	var variance := moment2 / 9.0 - mean * mean
	var deviation := Vector3(
		sqrt(maxf(variance.x, 0.0)), sqrt(maxf(variance.y, 0.0)), sqrt(maxf(variance.z, 0.0))
	)
	return ycocg_to_rgb(
		clip_to_box(rgb_to_ycocg(history_color), mean - deviation, mean + deviation)
	)


static func resolve(
	current: Dictionary,
	history: Dictionary,
	depth: Dictionary,
	motion: Dictionary,
	pixel: Vector2i,
	reset: bool,
	hdr: bool
) -> Dictionary:
	var extent := Vector2(current.extent)
	var uv := (Vector2(pixel) + Vector2.ONE * .5) / extent
	var original := nearest(current, uv)
	var color := rgb(original)
	if hdr:
		color = tonemap(color)
	var confidence := 1.0 / .015
	if not reset:
		var velocity := closest_motion(depth, motion, uv, extent)
		var history_uv := uv - velocity
		var previous := clip_history(
			sample_history(history, history_uv, extent), current, uv, color, hdr
		)
		confidence = nearest(history, uv).w
		var pixel_motion := velocity.abs() * extent
		if pixel_motion.x < .01 and pixel_motion.y < .01:
			confidence += 10.0
		else:
			confidence = 1.0
		var factor := clampf(1.0 / confidence, .015, .1)
		if history_uv.clamp(Vector2.ZERO, Vector2.ONE) != history_uv:
			factor = 1.0
			confidence = 1.0
		color = previous.lerp(color, factor)
	var stored_history := Vector4(color.x, color.y, color.z, confidence)
	if hdr:
		color = reverse_tonemap(color)
	return {"history": stored_history, "resolved": Vector4(color.x, color.y, color.z, original.w)}
