extends SceneTree

# Actual Ashbringer M2, placed through TerrainObjects' doodad path. Its texture
# receipts point only into the product directory, not a flat textures directory.
const FDID := 148234
const TEXTURES := [167227, 167226, 137243, 166494, 249237]
const KEY := "0123456789abcdef0123456789abcdef"
const ROOT := "user://world_ribbon_product_fixture"
const PRODUCT := ROOT + "/products/wow/" + KEY

func _initialize() -> void:
	call_deferred("run_fixture")

func fail(message: String) -> void:
	push_error(message)
	quit(1)

func write_bytes(path: String, bytes: PackedByteArray) -> bool:
	var file := FileAccess.open(path, FileAccess.WRITE)
	if file == null:
		return false
	file.store_buffer(bytes)
	return true

func run_fixture() -> void:
	root.size = Vector2i(1280, 720)
	for folder in ["/cache", "/textures", "/products/wow/" + KEY + "/models", "/products/wow/" + KEY + "/textures"]:
		DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path(ROOT + folder))
	var source := "res://../data"
	var model_path := PRODUCT + "/models/%d.m2" % FDID
	var skin_path := PRODUCT + "/models/%d00.skin" % FDID
	if not write_bytes(model_path, FileAccess.get_file_as_bytes(source + "/models/%d.m2" % FDID)) or not write_bytes(skin_path, FileAccess.get_file_as_bytes(source + "/models/%d00.skin" % FDID)):
		fail("Cannot copy real M2 and skin")
		return
	var receipts := []
	for id in TEXTURES:
		var bytes := FileAccess.get_file_as_bytes(source + "/textures/%d.blp" % id)
		var relative := "products/wow/" + KEY + "/textures/%d.blp" % id
		if bytes.is_empty() or not write_bytes(ROOT + "/" + relative, bytes):
			fail("Missing real texture %d" % id)
			return
		receipts.append({"product": "wow", "build": "fixture", "build_key": KEY,
			"fdid": id, "kind": "blp", "path": relative, "bytes": bytes.size(),
			"sha256": "a".repeat(64), "content_key": "b".repeat(32)})
	if not write_bytes(ROOT + "/cache/model-asset-index.json", JSON.stringify({"version": 1, "assets": receipts}).to_utf8_buffer()):
		fail("Cannot write texture receipts")
		return
	if FileAccess.file_exists(ROOT + "/textures/167227.blp"):
		fail("Ribbon texture exists at the forbidden flat path")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	client.set_process(false)
	var skin_error: String = client.preview_world_skin(true)
	if skin_error != "" or client.account_state().ui_skin != "Forever":
		fail("Forever skin unavailable: " + skin_error)
		return
	var probe: Node3D = ClassDB.instantiate("WowRibbonWorldProbe")
	root.add_child(probe)
	var error: String = probe.load_model(model_path, skin_path, FDID)
	if error != "":
		fail("Doodad placement: " + error)
		return
	var doodad := probe.find_child("DoodadRibbonFixture", true, false) as Node3D
	if doodad == null:
		fail("Doodad was not attached")
		return
	var bounds := AABB()
	var first := true
	for mesh in doodad.find_children("*", "MeshInstance3D", true, false):
		if str(mesh.name).begins_with("M2RibbonStrip"):
			continue
		var box: AABB = mesh.get_aabb()
		bounds = box if first else bounds.merge(box)
		first = false
	if first:
		fail("Real model has no meshes")
		return
	var camera := Camera3D.new()
	root.add_child(camera)
	camera.position = bounds.get_center() + Vector3(0.0, 0.0, maxf(bounds.size.length() * 0.8, 2.0))
	camera.look_at(bounds.get_center())
	camera.current = true
	for frame in 120:
		doodad.position.x = sin(float(frame) * 0.06) * 0.7
		probe.advance(camera, 1.0 / 60.0)
		await process_frame
	if client.account_state().ui_skin != "Forever":
		fail("Skin changed before screenshot")
		return
	var ribbons := doodad.find_children("M2RibbonStrip*", "MeshInstance3D", true, false)
	var visible := 0
	for ribbon in ribbons:
		if ribbon.visible and ribbon.mesh != null and ribbon.mesh.get_surface_count() > 0:
			visible += 1
	await RenderingServer.frame_post_draw
	var shown := root.get_texture().get_image()
	var screenshot := OS.get_environment("RIBBON_SCREENSHOT")
	if screenshot != "" and shown.save_png(screenshot) != OK:
		fail("Screenshot write failed")
		return
	print("WORLD_RIBBON_FIXTURE fdid=%d texture=167227 nodes=%d visible=%d skin=Forever" % [FDID, ribbons.size(), visible])
	if visible == 0:
		fail("Real world doodad created no visible ribbon geometry")
		return
	for ribbon in ribbons:
		ribbon.visible = false
	await process_frame
	await RenderingServer.frame_post_draw
	var hidden := root.get_texture().get_image()
	var changed := 0
	for y in shown.get_height():
		for x in shown.get_width():
			var a := shown.get_pixel(x, y)
			var b := hidden.get_pixel(x, y)
			if maxf(maxf(absf(a.r - b.r), absf(a.g - b.g)), absf(a.b - b.b)) > 0.02:
				changed += 1
	print("WORLD_RIBBON_PIXELS changed=", changed)
	if changed == 0:
		fail("Visible ribbon geometry changed no rendered pixels")
		return
	for ribbon in ribbons:
		ribbon.visible = true
	camera.look_at(camera.position + Vector3.BACK)
	probe.advance(camera, 1.0 / 60.0)
	for ribbon in ribbons:
		if ribbon.visible:
			fail("Off-screen ribbon remained visible")
			return
	print("PASS: real world doodad draws and culls product-root ribbons")
	quit(0)
