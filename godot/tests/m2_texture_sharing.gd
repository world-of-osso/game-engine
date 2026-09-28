extends SceneTree

# Two placements of one model, and two models authored with the same texture (FDID
# 1006709, a 512x512 DXT1 BLP), must bind one GPU texture, uploaded block-compressed
# with its mip chain. Each placement used to upload its own RGBA8 copy.

const DATA := "res://../data/models/"

func fail(message: String) -> void:
	push_error(message)
	quit(1)

func _initialize() -> void:
	if not ClassDB.class_exists("WowAssetLoader"):
		fail("WowAssetLoader not registered")
		return
	check.call_deferred()

func base_textures(model: Node) -> Array:
	var textures := []
	for mesh_instance in model.find_children("*", "MeshInstance3D", true, false):
		var mesh := (mesh_instance as MeshInstance3D).mesh
		for surface in mesh.get_surface_count():
			var material := (mesh_instance as MeshInstance3D).get_active_material(surface) as ShaderMaterial
			if material != null:
				var texture = material.get_shader_parameter("base_texture")
				if texture != null:
					textures.append(texture)
	return textures

func load_model(loader: Object, fdid: int) -> Node3D:
	var result: Dictionary = loader.load_m2(DATA + "%d.m2" % fdid)
	if result.has("error"):
		fail("%d load: %s" % [fdid, result.error])
		return null
	return result.node

func check() -> void:
	var loader = ClassDB.instantiate("WowAssetLoader")
	var models := []
	for fdid in [1016191, 1016191, 1016200]:
		var model := load_model(loader, fdid)
		if model == null:
			return
		models.append(model)
	var shared = null
	for model in models:
		var textures := base_textures(model)
		if textures.is_empty():
			fail("model has no base texture")
			return
		for texture in textures:
			if shared == null:
				shared = texture
			elif texture != shared:
				fail("placements bind separate copies of texture 1006709")
				return
	var image: Image = (shared as ImageTexture).get_image()
	if image == null or image.get_format() != Image.FORMAT_DXT1:
		fail("texture 1006709 uploaded as %s, expected DXT1" % [image.get_format() if image else "nothing"])
		return
	if not image.has_mipmaps():
		fail("texture 1006709 uploaded without its mip chain")
		return
	print("PASS: 3 placements of 2 models share one DXT1 texture with mipmaps")
	for model in models:
		model.free()
	quit(0)
