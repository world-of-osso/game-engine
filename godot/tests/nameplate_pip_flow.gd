extends SceneTree

# Offline proof through the production nameplate renderer, not a synthetic sprite.
# Run with --screen nameplatedebug and NAMEPLATE_PIP_ARTIFACTS set to an existing dir.
var client: Node
var scene: Node
var artifacts := ""

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	artifacts = OS.get_environment("NAMEPLATE_PIP_ARTIFACTS")
	if not DirAccess.dir_exists_absolute(artifacts):
		fail("Missing NAMEPLATE_PIP_ARTIFACTS directory")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var screen_deadline := Time.get_ticks_msec() + 90000
	while scene == null and Time.get_ticks_msec() < screen_deadline:
		scene = client.get_node_or_null("NameplateDebug")
		await process_frame
	if scene == null:
		fail("Nameplate debug screen did not open")
		return
	var deadline := Time.get_ticks_msec() + 90000
	var ready := false
	while Time.get_ticks_msec() < deadline:
		var plates: Dictionary = scene.debug_state().plates
		var cast: Dictionary = plates.get("Zolramus Sorcerer", {}).get("cast", {})
		if cast.get("icon_visible", false) and cast.get("fraction", 0.0) > 0.4 and cast.get("fraction", 1.0) < 0.6:
			ready = true
			break
		await process_frame
	if not ready:
		fail("No mid-cast preview with icon before deadline")
		return
	var key := InputEventKey.new()
	key.keycode = KEY_SPACE
	key.pressed = true
	root.push_input(key, true)
	await process_frame
	if not scene.debug_state().paused:
		fail("Preview did not pause")
		return
	var sparks := scene.find_children("Spark", "TextureRect", true, false)
	if sparks.is_empty():
		fail("No production spark nodes")
		return
	var rectangles: Array = []
	var tracks: Array[Rect2] = []
	for spark: TextureRect in sparks:
		var track := spark.get_parent().get_node("Background") as TextureRect
		var fill := spark.get_parent().get_node("Fill") as TextureRect
		var glow_rect := spark.get_global_rect()
		var track_rect := track.get_global_rect()
		if glow_rect.position.y < track_rect.position.y - 0.001 or glow_rect.end.y > track_rect.end.y + 0.001:
			fail("Glow extends vertically outside cast track")
			return
		if absf(glow_rect.get_center().x - fill.get_global_rect().end.x) > 0.001:
			fail("Glow not centered on fill edge")
			return
		var material := spark.material as CanvasItemMaterial
		if material == null or material.blend_mode != CanvasItemMaterial.BLEND_MODE_ADD:
			fail("Glow does not blend additively")
			return
		# AtlasTexture.get_image() cannot crop compressed BLP pixels; inspect the source.
		var texture: Texture2D = spark.texture
		while texture is AtlasTexture:
			texture = (texture as AtlasTexture).atlas
		var pixels := texture.get_image()
		if pixels.is_compressed() and pixels.decompress() != OK:
			fail("Cannot decompress glow source for pixel assertion")
			return
		var middle := Vector2i(pixels.get_width() / 2, pixels.get_height() / 2)
		if pixels.get_pixelv(middle).a <= pixels.get_pixel(0, middle.y).a:
			fail("Glow lacks bright center and soft falloff")
			return
		tracks.append(track_rect)
		rectangles.append({"track": [track_rect.position.x, track_rect.position.y, track_rect.size.x, track_rect.size.y], "edge": fill.get_global_rect().end.x})
	var shown := await capture()
	for spark: TextureRect in sparks:
		spark.visible = false
	var hidden := await capture()
	var brightened := 0
	# Compare only cast tracks plus vertical margins; unrelated FPS text may animate.
	for track: Rect2 in tracks:
		for y in range(floori(track.position.y) - 2, ceili(track.end.y) + 2):
			for x in range(floori(track.position.x), ceili(track.end.x)):
				var a := shown.get_pixel(x, y)
				var b := hidden.get_pixel(x, y)
				if maxf(absf(a.r - b.r), maxf(absf(a.g - b.g), absf(a.b - b.b))) > 0.02:
					if y < floori(track.position.y) or y >= ceili(track.end.y):
						fail("Glow drew pixels above or below track")
						return
					if a.r + a.g + a.b <= b.r + b.g + b.b:
						fail("Glow darkens underlying bar")
						return
					brightened += 1
	if brightened == 0:
		fail("Glow drew no pixels")
		return
	shown.save_png(artifacts.path_join("debug-pip.png"))
	hidden.save_png(artifacts.path_join("debug-no-pip.png"))
	var file := FileAccess.open(artifacts.path_join("geometry.json"), FileAccess.WRITE)
	file.store_string(JSON.stringify(rectangles))
	file.close()
	print("PASS: nameplate glow within track, centered on fill edge, additive brightening; pixels=", brightened)
	client.free()
	quit(0)

func capture() -> Image:
	await process_frame
	await process_frame
	await RenderingServer.frame_post_draw
	return root.get_texture().get_image()

func fail(message: String) -> void:
	push_error("Nameplate pip: " + message)
	quit(1)
