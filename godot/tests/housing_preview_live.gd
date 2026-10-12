extends "res://tests/housing_catalog_live.gd"

# Physical private-server catalog input. Preview may not place, grant or move gameplay camera.
var preview_receipts: Array = []

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty():
		fail("Preview proof requires private5870/fb_housing_* evidence configuration")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	await click_control(housing_named_control("MicroMenuUI", "HousingMicroButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return client.housing_state().get("confirmed", false) and housing_control("HousingDashboardCatalogTabButton") != null, "authoritative dashboard"): return
	await click_control(housing_control("HousingDashboardCatalogTabButton"), MOUSE_BUTTON_LEFT)
	var before: Dictionary = authority_receipt()
	if not await preview_entry(1693, BOOKCASE_NAME, "preview-bookcase"): return
	if not await rotate_preview(): return
	if not await preview_entry(2330, "Gnomish Tesla Coil", "preview-tesla"): return
	if not await preview_entry(525, "Orgrimmar Round Interior Pillar", "preview-pillar"): return
	if not await preview_entry(1693, BOOKCASE_NAME, "preview-replaced"): return
	await click_control(housing_control("HousingDashboardHouseInfoTabButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return preview_view() == null and client.find_children("HousingCatalogActor", "Node3D", true, false).is_empty(), "tab change frees preview scene"): return
	await click_control(housing_control("HousingDashboardCatalogTabButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func():
		var actor: Node3D = preview_actor()
		return housing_text("HousingCatalogSearchBox") == BOOKCASE_NAME and actor != null and int(actor.get_meta("housing_decor_id", 0)) == 1693, "reopened catalog retains filter and actual bookcase actor"): return
	var search: LineEdit = housing_control("HousingCatalogSearchBox") as LineEdit
	await click_control(search, MOUSE_BUTTON_LEFT)
	search.select_all()
	await housing_type_text("no-such-housing-core7-decor")
	if not await wait_housing(func(): return preview_view() == null and client.find_children("HousingCatalogActor", "Node3D", true, false).is_empty(), "empty search clears actor"): return
	if not await capture_housing("preview-empty"): return
	await click_control(housing_control("HousingDashboardFrameCloseButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return client.get_node_or_null("HousingDashboardUI") == null and client.find_children("HousingCatalogActor", "Node3D", true, false).is_empty(), "close frees every preview actor"): return
	if before != authority_receipt():
		fail("Catalog preview changed owned quantities or layout")
		return
	var receipt: FileAccess = FileAccess.open(shots.path_join("preview-receipts.json"), FileAccess.WRITE)
	if receipt == null:
		fail("Cannot write preview proof receipts")
		return
	receipt.store_string(JSON.stringify({"skin": housing_skin, "before": before, "after": authority_receipt(), "previews": preview_receipts}, "\t"))
	receipt.close()
	print("PASS: private5870 receipt-backed catalog models, physical rotation, selection/search/tab/close teardown, unchanged authority ", housing_skin)
	client.free()
	quit(0)

func authority_receipt() -> Dictionary:
	var state: Dictionary = client.housing_state()
	return {"stored": state.get("stored", []), "placed": state.get("placed", []), "houses": state.get("houses", []), "house_id": state.get("house_id", 0), "decor": state.get("decor", [])}

func preview_view() -> TextureRect:
	var host: Control = housing_control("HousingCatalogModelScene")
	return host.find_child("CharacterModelView", true, false) as TextureRect if host != null else null

func preview_actor() -> Node3D:
	var view: TextureRect = preview_view()
	return view.find_child("HousingCatalogActor", true, false) as Node3D if view != null else null

func preview_entry(decor_id: int, decor_name: String, phase: String) -> bool:
	var search: LineEdit = housing_control("HousingCatalogSearchBox") as LineEdit
	await click_control(search, MOUSE_BUTTON_LEFT)
	search.select_all()
	await housing_type_text(decor_name)
	if not await wait_housing(func():
		var actor: Node3D = preview_actor()
		return housing_text("HousingPreviewName") == decor_name and actor != null and int(actor.get_meta("housing_decor_id", 0)) == decor_id, "actual preview " + decor_name): return false
	var view: TextureRect = preview_view()
	var viewport: SubViewport = view.find_child("CharacterModelViewport", true, false) as SubViewport
	if viewport == null or not viewport.own_world_3d or not viewport.transparent_bg:
		fail("Preview does not have a separate transparent world")
		return false
	await frames(20)
	await RenderingServer.frame_post_draw
	var image: Image = viewport.get_texture().get_image()
	var opaque: int = 0
	var readable: int = 0
	for y in range(image.get_height()):
		for x in range(image.get_width()):
			var color: Color = image.get_pixel(x, y)
			if color.a <= 0.1: continue
			opaque += 1
			if maxf(color.r, maxf(color.g, color.b)) > 0.02: readable += 1
	if opaque < 10:
		fail("Preview actor has no rendered opaque model pixels")
		return false
	preview_receipts.append({"phase": phase, "decor_id": decor_id, "model_fdid": preview_actor().get_meta("model_file_data_id", 0), "meshes": preview_actor().find_children("*", "MeshInstance3D", true, false).size(), "rendered_model_pixels": opaque, "readable_model_pixels": readable})
	return await capture_housing(phase)

func rotate_preview() -> bool:
	var host: Control = housing_control("HousingCatalogModelScene")
	var point: Vector2 = host.get_global_rect().get_center()
	var camera: Camera3D = root.get_camera_3d()
	var camera_before: Transform3D = camera.global_transform
	var pivot: Node3D = preview_actor().get_parent() as Node3D
	var before: float = pivot.rotation.y
	await hover_point(point)
	var press: InputEventMouseButton = InputEventMouseButton.new()
	press.button_index = MOUSE_BUTTON_LEFT
	press.position = point
	press.global_position = point
	press.pressed = true
	Input.parse_input_event(press)
	await process_frame
	var motion: InputEventMouseMotion = InputEventMouseMotion.new()
	motion.position = point + Vector2(70, 0)
	motion.global_position = motion.position
	motion.relative = Vector2(70, 0)
	motion.button_mask = MOUSE_BUTTON_MASK_LEFT
	Input.parse_input_event(motion)
	await process_frame
	press.pressed = false
	press.position = point + Vector2(70, 0)
	press.global_position = press.position
	Input.parse_input_event(press)
	await frames(10)
	if is_equal_approx(before, pivot.rotation.y) or not camera_before.is_equal_approx(camera.global_transform):
		fail("Physical preview drag failed or moved gameplay camera")
		return false
	preview_receipts.append({"phase": "rotated", "before_yaw": before, "after_yaw": pivot.rotation.y, "gameplay_camera_unchanged": true})
	return await capture_housing("preview-rotated")
