extends RefCounted
# One terrain chunk's inputs, packed into terrain.gdshader's per-tile inputs the way
# rust/src/terrain/tile_material.rs packs them, for every chunk of the tile: a fixture mesh
# draws them whatever its UV2 (the chunk index). Inputs keep the per-chunk names: config,
# layer_params_N, animation_params_N, ground_N, height_N, alpha_packed.

const CHUNKS_PER_SIDE := 16
const DATA_ROWS := 11
const ALPHA_SIZE := 64
const CLASS_STRIDE := 4096.0
const REF_WHITE := -1.0
const REF_UNTEXTURED := -2.0

var config := Vector4(1.0, 0.0, 1.0, 0.0)
var layer_params: Array[Vector4] = [Vector4(1, 0, 0, 1), Vector4(1, 0, 0, 1), Vector4(1, 0, 0, 1), Vector4(1, 0, 0, 1)]
var animation_params: Array[Vector4] = [Vector4.ZERO, Vector4.ZERO, Vector4.ZERO, Vector4.ZERO]
# Image per slot, or null for Godot's unbound-sampler white.
var ground: Array = [null, null, null, null]
var height: Array = [null, null, null, null]
var alpha: Image = null

func set_input(material: ShaderMaterial, name: String, value) -> void:
	var slot := int(name.right(1)) if name.right(1).is_valid_int() else -1
	if name == "config":
		config = value
	elif name.begins_with("layer_params_"):
		layer_params[slot] = value
	elif name.begins_with("animation_params_"):
		animation_params[slot] = value
	elif name.begins_with("ground_"):
		ground[slot] = _image(value)
	elif name.begins_with("height_"):
		height[slot] = _image(value)
	elif name == "alpha_packed":
		alpha = _image(value)
	else:
		push_error("Unknown terrain chunk input " + name)
		return
	apply(material)

func apply(material: ShaderMaterial) -> void:
	var classes: Array = []
	var ground_refs := _references(ground, classes)
	var height_refs := _references(height, classes)
	for index in 4:
		var images: Array[Image] = []
		for image in (classes[index] if index < classes.size() else []):
			images.append(image)
		if images.is_empty():
			material.set_shader_parameter("layers_%d" % index, null)
			continue
		var array := Texture2DArray.new()
		if array.create_from_images(images) != OK:
			push_error("Could not create terrain layer array %d" % index)
		material.set_shader_parameter("layers_%d" % index, array)
	var data := Image.create(CHUNKS_PER_SIDE * CHUNKS_PER_SIDE, DATA_ROWS, false, Image.FORMAT_RGBAF)
	var rows: Array[Vector4] = [config]
	rows.append_array(layer_params)
	rows.append_array(animation_params)
	rows.append(ground_refs)
	rows.append(height_refs)
	for row in rows.size():
		var value: Vector4 = rows[row]
		data.fill_rect(Rect2i(0, row, CHUNKS_PER_SIDE * CHUNKS_PER_SIDE, 1), Color(value.x, value.y, value.z, value.w))
	material.set_shader_parameter("chunk_data", ImageTexture.create_from_image(data))
	var atlas := Image.create(CHUNKS_PER_SIDE * ALPHA_SIZE, CHUNKS_PER_SIDE * ALPHA_SIZE, false, Image.FORMAT_RGBA8)
	if alpha != null:
		var chunk := alpha.duplicate() as Image
		chunk.resize(ALPHA_SIZE, ALPHA_SIZE, Image.INTERPOLATE_NEAREST)
		for index in CHUNKS_PER_SIDE * CHUNKS_PER_SIDE:
			var cell := Vector2i(index % CHUNKS_PER_SIDE, index / CHUNKS_PER_SIDE) * ALPHA_SIZE
			atlas.blit_rect(chunk, Rect2i(0, 0, ALPHA_SIZE, ALPHA_SIZE), cell)
	material.set_shader_parameter("alpha_atlas", ImageTexture.create_from_image(atlas))
	material.set_shader_parameter("chunk_hidden", ImageTexture.create_from_image(Image.create(CHUNKS_PER_SIDE, CHUNKS_PER_SIDE, false, Image.FORMAT_R8)))

# Each slot's chunk_data reference; images join the size class of their dimensions.
func _references(slots: Array, classes: Array) -> Vector4:
	var refs := [REF_WHITE, REF_WHITE, REF_WHITE, REF_WHITE]
	for slot in 4:
		var image: Image = slots[slot]
		if image == null:
			continue
		var class_index := -1
		for index in classes.size():
			if classes[index][0].get_size() == image.get_size():
				class_index = index
		if class_index < 0:
			classes.append([])
			class_index = classes.size() - 1
		classes[class_index].append(image)
		refs[slot] = class_index * CLASS_STRIDE + classes[class_index].size() - 1
	return Vector4(refs[0], refs[1], refs[2], refs[3])

func _image(value) -> Image:
	if value == null:
		return null
	var image := (value as Texture2D).get_image() if value is Texture2D else (value as Image).duplicate() as Image
	image.convert(Image.FORMAT_RGBA8)
	return image

# A built tile's chunk indices, from its Chunk{x}_{y} collision bodies.
static func tile_chunks(tile: Node) -> Array[Vector2i]:
	var chunks: Array[Vector2i] = []
	for child in tile.get_children():
		if child is StaticBody3D and child.name.begins_with("Chunk"):
			var index: PackedStringArray = child.name.trim_prefix("Chunk").split("_")
			chunks.append(Vector2i(int(index[0]), int(index[1])))
	return chunks

# chunk_data row `row` of chunk `index` in a built tile material.
static func chunk_row(data: Image, index: Vector2i, row: int) -> Vector4:
	var texel := data.get_pixel(index.y * CHUNKS_PER_SIDE + index.x, row)
	return Vector4(texel.r, texel.g, texel.b, texel.a)

# The layer array image a chunk_data texture reference points at.
static func layer_array(material: ShaderMaterial, reference: float) -> Texture2DArray:
	return material.get_shader_parameter("layers_%d" % int(reference / CLASS_STRIDE)) as Texture2DArray
