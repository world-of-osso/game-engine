extends RefCounted

# CPU transliteration of WebWowViewerCpp 1a8cccb's retail ("Legion logic") M2 batch rules,
# read straight from the .m2/.skin bytes, independent of the native loader and binder.
# Sources: m2Object.cpp (getPixelShaderId, getVertexShaderId, M2ShaderTable,
# M2BlendingModeToEGxBlendEnum), M2MeshBufferUpdater.cpp (texture weights, texture
# matrix slots, applyWeight), animationManager.cpp calcTextureAnimationTransform,
# commonM2Material.slang (calcM2VertexMat, calcM2FragMaterial), commonFunctions.slang
# (posToTexCoord, edgeScan), commonLightFunctions.slang (calcLight, applyAndMixAmbients),
# m2shader_text.slang (mesh colour/alpha, specular after lighting).

# M2ShaderTable rows: [pixel, vertex].
const SHADER_TABLE := [
	[12, 3], [13, 3], [14, 3], [15, 6], [16, 3], [13, 7], [16, 7], [17, 3], [18, 3], [19, 6],
	[20, 7], [21, 3], [22, 3], [23, 3], [23, 7], [20, 2], [24, 3], [25, 6], [26, 0], [33, 9],
	[27, 11], [6, 12], [28, 2], [29, 7], [25, 11], [33, 13], [30, 14], [31, 2], [32, 14], [34, 7],
	[35, 15], [35, 16], [0, 0], [7, 12], [1, 9], [36, 12]]
const GX_BLEND := [0, 1, 2, 10, 3, 4, 5, 13]

var md20: PackedByteArray
var skin: PackedByteArray
var txid_values := PackedInt32Array()

func _init(model_bytes: PackedByteArray, skin_bytes: PackedByteArray) -> void:
	assert(model_bytes.slice(0, 4).get_string_from_ascii() == "MD21")
	md20 = model_bytes.slice(8, 8 + model_bytes.decode_u32(4))
	skin = skin_bytes
	var offset := 0
	while offset + 8 <= model_bytes.size():
		var size := model_bytes.decode_u32(offset + 4)
		if model_bytes.slice(offset, offset + 4).get_string_from_ascii() == "TXID":
			for index in size / 4:
				txid_values.append(model_bytes.decode_u32(offset + 8 + index * 4))
		offset += 8 + size

# --- file access ----------------------------------------------------------------------

func array_at(bytes: PackedByteArray, offset: int) -> Vector2i:
	return Vector2i(bytes.decode_u32(offset), bytes.decode_u32(offset + 4))

func lookup_i16(header_offset: int, index: int) -> int:
	var table := array_at(md20, header_offset)
	if index < 0 or index >= table.x:
		return -1
	return md20.decode_s16(table.y + index * 2)

func batch_count() -> int:
	return array_at(skin, 36).x

func batch(index: int) -> Dictionary:
	var offset := array_at(skin, 36).y + index * 24
	var material := skin.decode_u16(offset + 10)
	var materials := array_at(md20, 0x70)
	var submesh := skin.decode_u16(offset + 4)
	return {
		"flags": skin[offset],
		"priority_plane": skin.decode_s8(offset + 1),
		"shader_id": skin.decode_u16(offset + 2),
		"submesh": submesh,
		"color_index": skin.decode_s16(offset + 8),
		"render_flags": md20.decode_u16(materials.y + material * 4),
		"blend_mode": md20.decode_u16(materials.y + material * 4 + 2),
		"material_layer": skin.decode_u16(offset + 12),
		"texture_count": skin.decode_u16(offset + 14),
		"texture_combo": skin.decode_u16(offset + 16),
		"texture_coord_combo": skin.decode_u16(offset + 18),
		"weight_combo": skin.decode_u16(offset + 20),
		"transform_combo": skin.decode_u16(offset + 22),
		"triangle_count": skin.decode_u16(array_at(skin, 28).y + submesh * 48 + 10),
	}

func global_sequences() -> PackedInt32Array:
	var table := array_at(md20, 0x14)
	var values := PackedInt32Array()
	for index in table.x:
		values.append(md20.decode_u32(table.y + index * 4))
	return values

# M2Track at `offset`, sequence 0: interpolation, global sequence, keys and values.
func track(offset: int, value_size: int) -> Dictionary:
	var result := {"interpolation": md20.decode_u16(offset), "global": md20.decode_s16(offset + 2), "times": PackedInt32Array(), "values": []}
	var times := array_at(md20, offset + 4)
	var values := array_at(md20, offset + 12)
	if times.x == 0 or values.x == 0:
		return result
	var inner_times := array_at(md20, times.y)
	var inner_values := array_at(md20, values.y)
	for index in inner_times.x:
		result.times.append(md20.decode_u32(inner_times.y + index * 4))
	for index in inner_values.x:
		var at := inner_values.y + index * value_size
		match value_size:
			2:
				result.values.append(float(md20.decode_s16(at)) / 32767.0)
			12:
				result.values.append(Vector3(md20.decode_float(at), md20.decode_float(at + 4), md20.decode_float(at + 8)))
			16:
				result.values.append(Quaternion(md20.decode_float(at), md20.decode_float(at + 4), md20.decode_float(at + 8), md20.decode_float(at + 12)))
	return result

# animateTrack at application time: global tracks wrap their global sequence, local
# tracks loop sequence 0 (the Stand animation the reference keeps playing).
func sample(data: Dictionary, elapsed_ms: int, fallback: Variant) -> Variant:
	if data.values.is_empty():
		return fallback
	var duration: int
	if data.global >= 0:
		duration = global_sequences()[data.global]
	else:
		duration = md20.decode_u32(array_at(md20, 0x1c).y + 4)
	var time := 0 if duration == 0 else elapsed_ms % duration
	var times: PackedInt32Array = data.times
	if time <= times[0] or data.values.size() == 1:
		return data.values[0]
	for index in range(1, times.size()):
		if time < times[index]:
			if data.interpolation == 0:
				return data.values[index - 1]
			var t := float(time - times[index - 1]) / float(times[index] - times[index - 1])
			var a: Variant = data.values[index - 1]
			var b: Variant = data.values[index]
			if a is Quaternion:
				return (a as Quaternion).slerp(b, t)
			return lerp(a, b, t)
	return data.values[data.values.size() - 1]

# --- WebWowViewer resolution ----------------------------------------------------------

func pixel_shader(texture_count: int, shader_id: int) -> int:
	if shader_id & 0x8000:
		return SHADER_TABLE[shader_id & 0x7fff][0]
	if texture_count == 1:
		return 1 if shader_id & 0x70 else 0
	var low := shader_id & 7
	if shader_id & 0x70:
		return {0: 11, 1: 6, 2: 6, 5: 6, 3: 8, 4: 7, 6: 9, 7: 10}[low]
	return {0: 5, 1: 2, 2: 2, 5: 2, 3: 13, 7: 13, 4: 3, 6: 4}[low]

func vertex_shader(texture_count: int, shader_id: int) -> int:
	if shader_id & 0x8000:
		return SHADER_TABLE[shader_id & 0x7fff][1]
	if texture_count == 1:
		if shader_id & 0x80 == 0:
			return 10 if shader_id & 0x4000 else 0
		return 1
	if shader_id & 0x80 == 0:
		if shader_id & 8:
			return 3
		return 2 if shader_id & 0x4000 else 7
	return 5 if shader_id & 8 else 4

# Texture slot j of the batch: [fdid or 0 for replaceable, wrap_x, wrap_y].
func texture_slot(info: Dictionary, slot: int) -> Array:
	var index := lookup_i16(0x80, info.texture_combo + slot)
	var textures := array_at(md20, 0x50)
	var offset := textures.y + index * 16
	var type := md20.decode_u32(offset)
	var flags := md20.decode_u32(offset + 4)
	var txid := 0
	if type == 0:
		txid = txid_fdid(index)
	return [txid, flags & 1 != 0, flags & 2 != 0]

# The M2 texture type of slot j: 0 a file, else the replaceable type (1 body, 6 hair...).
func texture_type(info: Dictionary, slot: int) -> int:
	var index := lookup_i16(0x80, info.texture_combo + slot)
	return md20.decode_u32(array_at(md20, 0x50).y + index * 16)

func txid_fdid(index: int) -> int:
	return txid_values[index] if index < txid_values.size() else 0

func weight(info: Dictionary, slot: int, elapsed_ms: int) -> float:
	if slot >= info.texture_count:
		return 1.0
	var index := lookup_i16(0x90, info.weight_combo + slot)
	var weights := array_at(md20, 0x58)
	if index < 0 or index >= weights.x:
		return 1.0
	return sample(track(weights.y + index * 20, 2), elapsed_ms, 1.0)

func mesh_color(info: Dictionary, elapsed_ms: int) -> Color:
	var colors := array_at(md20, 0x48)
	var color := Color(1.0, 1.0, 1.0, 1.0)
	if info.color_index >= 0 and info.color_index < colors.x:
		var offset: int = colors.y + info.color_index * 40
		var rgb: Vector3 = sample(track(offset, 12), elapsed_ms, Vector3.ONE)
		color = Color(rgb.x, rgb.y, rgb.z, sample(track(offset + 20, 2), elapsed_ms, 1.0))
	if info.texture_count > 0 and info.flags & 0x40 == 0:
		color.a *= weight(info, 0, elapsed_ms)
	return color

# calcTextureAnimationTransform as a 2D affine map of (u, v, 0, 1).
func texture_matrix(info: Dictionary, slot: int, elapsed_ms: int) -> Transform2D:
	var transform_index := lookup_i16(0x98, info.transform_combo + slot)
	var transforms := array_at(md20, 0x60)
	if transform_index < 0 or transform_index >= transforms.x:
		return Transform2D.IDENTITY
	var base: int = transforms.y + transform_index * 60
	var matrix := Projection.IDENTITY
	var pivot := Projection(Transform3D(Basis.IDENTITY, Vector3(0.5, 0.5, 0.0)))
	var unpivot := Projection(Transform3D(Basis.IDENTITY, Vector3(-0.5, -0.5, 0.0)))
	var rotation := track(base + 20, 16)
	if not rotation.values.is_empty():
		var q: Quaternion = sample(rotation, elapsed_ms, Quaternion.IDENTITY)
		matrix = matrix * pivot * Projection(Transform3D(Basis(q), Vector3.ZERO)) * unpivot
	var scaling := track(base + 40, 12)
	if not scaling.values.is_empty():
		var s: Vector3 = sample(scaling, elapsed_ms, Vector3.ONE)
		matrix = matrix * pivot * Projection(Transform3D(Basis.from_scale(s), Vector3.ZERO)) * unpivot
	var translation := track(base, 12)
	if not translation.values.is_empty():
		var t: Vector3 = sample(translation, elapsed_ms, Vector3.ZERO)
		matrix = matrix * Projection(Transform3D(Basis.IDENTITY, t))
	return Transform2D(Vector2(matrix.x.x, matrix.x.y), Vector2(matrix.y.x, matrix.y.y), Vector2(matrix.w.x, matrix.w.y))

func matrix_slots(info: Dictionary, vertex: int) -> Array:
	var slots := [0, 2] if vertex == 11 else [0, 1]
	var result := [-1, -1]
	for index in mini(info.texture_count, 2):
		result[index] = slots[index]
	return result

# Everything the per-pixel evaluation needs, sampled once for the frozen time.
func material(info: Dictionary, elapsed_ms: int) -> Dictionary:
	var vertex := vertex_shader(info.texture_count, info.shader_id)
	var slots := matrix_slots(info, vertex)
	var matrices := []
	for slot in slots:
		matrices.append(Transform2D.IDENTITY if slot < 0 else texture_matrix(info, slot, elapsed_ms))
	var textures := []
	for slot in mini(info.texture_count, 4):
		textures.append(texture_slot(info, slot))
	return {
		"pixel": pixel_shader(info.texture_count, info.shader_id),
		"vertex": vertex,
		"matrices": matrices,
		"textures": textures,
		"weights": Vector3(weight(info, 0, elapsed_ms), weight(info, 1, elapsed_ms), weight(info, 2, elapsed_ms)),
		"mesh_color": mesh_color(info, elapsed_ms),
		"gx_blend": GX_BLEND[info.blend_mode] if info.blend_mode < 8 else 0,
		"lit": info.render_flags & 1 == 0,
	}

# --- per-pixel evaluation -------------------------------------------------------------

static func env_coord(view_position: Vector3, normal: Vector3) -> Vector2:
	var reflection := view_position.normalized().reflect(normal.normalized())
	# GLSL reflect(I, N) = I - 2 dot(N, I) N; Godot's Vector3.reflect(n) is its negation.
	reflection = -reflection
	var temp := Vector3(reflection.x, reflection.y, reflection.z + 1.0).normalized()
	return Vector2(0.5, 0.5) - Vector2(temp.x, temp.y) * 0.5

static func edge_scan(view_position: Vector3, normal: Vector3) -> float:
	var d := clampf((-view_position.normalized()).dot(normal), 0.0, 1.0)
	return clampf(2.7 * d * d - 0.4, 0.0, 1.0)

# calcM2VertexMat: [uv1, uv2, uv3, edge_fade].
static func vertex_uvs(vertex: int, view_position: Vector3, normal: Vector3, uv: Vector2, uv2: Vector2, m: Array) -> Array:
	var env := env_coord(view_position, normal)
	var t1: Vector2 = m[0] * uv
	var t2: Vector2 = m[1] * uv2
	var zero := Vector2.ZERO
	match vertex:
		0, 17, 18: return [t1, zero, zero, 1.0]
		1: return [env, zero, zero, 1.0]
		2: return [t1, t2, zero, 1.0]
		3: return [t1, env, zero, 1.0]
		4: return [env, m[0] * uv, zero, 1.0]
		5: return [env, env, zero, 1.0]
		6: return [t1, env, t1, 1.0]
		7: return [t1, t1, zero, 1.0]
		8: return [t1, t1, t1, 1.0]
		9: return [t1, zero, zero, edge_scan(view_position, normal)]
		10: return [m[1] * uv2, zero, zero, 1.0]
		11: return [t1, env, t2, 1.0]
		12: return [t1, t2, zero, edge_scan(view_position, normal)]
		13: return [env, zero, zero, edge_scan(view_position, normal)]
		14: return [t1, t2, t1, 1.0]
		15: return [t1, t2, zero, 1.0]
		16: return [m[1] * uv2, zero, zero, 1.0]
	return [zero, zero, zero, 1.0]

static func rgb(c: Color) -> Vector3:
	return Vector3(c.r, c.g, c.b)

static func mixv(a: Vector3, b: Vector3, t: float) -> Vector3:
	return a.lerp(b, t)

# calcM2FragMaterial: [diffuse, specular, discard_alpha, can_discard].
static func fragment(pixel: int, tex: Color, tex2: Color, tex3: Color, tex4: Color, mesh: Vector3, w: Vector3) -> Array:
	var t1 := rgb(tex)
	var t2 := rgb(tex2)
	var t3 := rgb(tex3)
	var zero := Vector3.ZERO
	match pixel:
		0: return [mesh * t1, zero, 1.0, false]
		1: return [mesh * t1, zero, tex.a, true]
		2: return [mesh * t1 * t2, zero, tex2.a, true]
		3: return [mesh * t1 * t2 * 2.0, zero, tex2.a * 2.0, true]
		4: return [mesh * t1 * t2 * 2.0, zero, 1.0, false]
		5: return [mesh * t1 * t2, zero, 1.0, false]
		6: return [mesh * t1 * t2, zero, tex.a * tex2.a, true]
		7: return [mesh * t1 * t2 * 2.0, zero, tex.a * tex2.a * 2.0, true]
		8: return [mesh * t1, t2, tex.a + tex2.a, true]
		9: return [mesh * t1 * t2 * 2.0, zero, tex.a, true]
		10: return [mesh * t1, t2, tex.a, true]
		11: return [mesh * t1 * t2, zero, tex.a, true]
		12: return [mesh * mixv(t1 * t2 * 2.0, t1, tex.a), zero, 1.0, false]
		13: return [mesh * t1, t2 * tex2.a, 1.0, false]
		14: return [mesh * t1, t2 * tex2.a * (1.0 - tex.a), 1.0, false]
		15: return [mesh * mixv(t1 * t2 * 2.0, t1, tex.a), t3 * tex3.a * w.z, 1.0, false]
		16: return [mesh * t1, t2 * tex2.a, tex.a, true]
		17: return [mesh * t1, t2 * tex2.a * (1.0 - tex.a), tex.a + tex2.a * (0.3 * tex2.r + 0.59 * tex2.g + 0.11 * tex2.b), true]
		18: return [mesh * mixv(mixv(t1, t2, tex2.a), t1, tex.a), zero, 1.0, false]
		19: return [mesh * mixv(t1 * t2 * 2.0, t3, tex3.a), zero, 1.0, false]
		20: return [mesh * t1, t2 * tex2.a * w.y, 1.0, false]
		21: return [mesh * t1, t2 * (1.0 - tex.a), tex.a + tex2.a, true]
		22: return [mesh * mixv(t1 * t2, t1, tex.a), zero, 1.0, false]
		23: return [mesh * t1, t2 * tex2.a * w.y, tex.a, true]
		24: return [mesh * mixv(t1, t2, tex2.a), t1 * tex.a * w.x, 1.0, false]
		25:
			var glow := clampf(tex3.a * w.z, 0.0, 1.0)
			return [mesh * mixv(t1 * t2 * 2.0, t1, tex.a) * (1.0 - glow), t3 * glow, 1.0, false]
		26, 28:
			var crossfade := tex.lerp(tex2, clampf(w.y, 0.0, 1.0)).lerp(tex3, clampf(w.z, 0.0, 1.0))
			return [mesh * rgb(crossfade), zero, crossfade.a * (tex4.a if pixel == 28 else 1.0), true]
		27: return [mesh * mixv(mixv(t1 * t2 * 2.0, t3, tex3.a), t1, tex.a), zero, 1.0, false]
		29: return [mesh * mixv(t1, t2, tex2.a), zero, 1.0, false]
		30: return [mesh * mixv(t1 * mixv(Vector3.ONE, t2, tex2.a), t3, tex3.a), zero, tex.a, true]
		31: return [mesh * t1 * mixv(Vector3.ONE, t2, tex2.a), zero, tex.a, true]
		32: return [mesh * mixv(t1 * mixv(Vector3.ONE, t2, tex2.a), t3, tex3.a), zero, 1.0, false]
		33: return [mesh * t1, zero, tex.a, true]
		34: return [zero, zero, tex.a, true]
		35:
			var product := Color(tex.r * tex2.r * tex3.r, tex.g * tex2.g * tex3.g, tex.b * tex2.b * tex3.b, tex.a * tex2.a * tex3.a)
			return [mesh * rgb(product), zero, product.a, true]
		36: return [mesh * t1 * t2, zero, tex.a * tex2.a, true]
	return [zero, zero, 1.0, false]

static func final_opacity(gx_blend: int, discard_alpha: float, mesh_opacity: float) -> float:
	if gx_blend == 0 or gx_blend == 1:
		return mesh_opacity
	return discard_alpha * mesh_opacity

static func discards(gx_blend: int, discard_alpha: float, can_discard: bool) -> bool:
	return gx_blend == 1 and can_discard and discard_alpha < 0.501960814

static func apply_ambients(sky: Vector3, horizon: Vector3, ground: Vector3, n_dot_l: float, n_dot_up: float) -> Vector3:
	var current := horizon.lerp(sky, n_dot_up) if n_dot_up >= 0.0 else horizon.lerp(ground, -n_dot_up)
	return (current * 0.7).lerp(current * 1.1, 0.5 + 0.5 * n_dot_l)

# calcLight without the light buffer, then `+ specular`.
static func light(diffuse: Vector3, specular: Vector3, world_normal: Vector3, lit: bool, scene: Dictionary) -> Vector3:
	if not lit:
		return diffuse + specular
	var n := world_normal.normalized()
	var n_dot_l := clampf(n.dot(-(scene.sun_direction as Vector3).normalized()), 0.0, 1.0)
	var ambient := apply_ambients(scene.ambient, scene.horizon, scene.ground, n_dot_l, n.dot(Vector3.UP))
	return diffuse * (ambient + (scene.direct as Vector3) * n_dot_l) + specular
