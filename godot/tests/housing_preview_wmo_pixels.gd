extends SceneTree

# Run with the private rendered-cage helper, not Godot's dummy headless renderer.
# Reports the known reference pixel20 AO gap; does not certify Retail AO appearance.
func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	var probe = ClassDB.instantiate("WowHousingPreviewProbe")
	var host := Control.new()
	host.size = Vector2(301, 334)
	root.add_child(host)
	var error: String = probe.select_decor(host, 525)
	if not error.is_empty():
		printerr("FAIL: WMO render source: ", error)
		quit(1)
		return
	for frame in range(20): await process_frame
	await RenderingServer.frame_post_draw
	var viewport: SubViewport = host.find_child("CharacterModelViewport", true, false) as SubViewport
	var image: Image = viewport.get_texture().get_image()
	for mesh in host.find_children("*", "MeshInstance3D", true, false):
		var material: ShaderMaterial = mesh.get_active_material(0) as ShaderMaterial
		var info: Dictionary = {"mesh": str(mesh.name)}
		for key in ["pixel_shader", "vertex_shader", "unlit", "has_mocv", "exterior_lit", "interior_ambient", "ambient", "scene_light", "base_color"]:
			info[key] = material.get_shader_parameter(key)
		print("WMO_PREVIEW_MATERIAL ", info)
	var solid: int = 0
	var readable: int = 0
	for y in range(image.get_height()):
		for x in range(image.get_width()):
			var color: Color = image.get_pixel(x, y)
			if color.a <= 0.1: continue
			solid += 1
			if maxf(color.r, maxf(color.g, color.b)) > 0.02: readable += 1
	var fraction: float = float(readable) / float(maxi(1, solid))
	print("WMO_PREVIEW_PIXELS solid=", solid, " readable=", readable, " fraction=", fraction)
	probe.clear_preview()
	host.free()
	if solid == 0 or readable == 0:
		printerr("FAIL: WMO preview rendered no geometry or source texture colors")
		quit(1)
		return
	print("PASS: source WMO geometry and texture pixels render; Retail pixel20 AO source gap remains")
	quit(0)
