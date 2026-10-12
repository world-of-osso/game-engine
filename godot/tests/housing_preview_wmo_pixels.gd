extends SceneTree

# Run with the private rendered-cage helper, not Godot's dummy headless renderer.
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
	if solid == 0 or fraction < 0.6:
		printerr("FAIL: source WMO texture is black under isolated preview lighting")
		quit(1)
		return
	print("PASS: authored WMO textures remain readable in isolated model lighting")
	quit(0)
