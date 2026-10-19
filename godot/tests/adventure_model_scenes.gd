extends "res://tests/adventure_model_selector.gd"
# Real Journal selections, source rows and loaded M2 geometry; no scene/pose injection.
# Default Retail ModelScene FOV is unknown: retain existing projection, do not certify it.
var completion: RefCounted

func run() -> void:
	completion = load("res://tests/completion.gd").new(self, 2)
	completion.finish(await run_scenes())

func source_row(table: String, id: int) -> Dictionary:
	var path := ProjectSettings.globalize_path("res://../data/db2/active/" + table + ".csv")
	var file := FileAccess.open(path, FileAccess.READ)
	if file == null:
		completion.fail("Missing source table " + path)
		return {}
	var headers := file.get_csv_line()
	var id_column := headers.find("ID")
	if id_column < 0:
		completion.fail("Missing source ID column " + path)
		return {}
	while not file.eof_reached():
		var fields := file.get_csv_line()
		if fields.size() != headers.size() or int(fields[id_column]) != id:
			continue
		var row := {}
		for index in headers.size():
			row[headers[index]] = fields[index]
		return row
	completion.fail("Missing source %s row%s" % [table, id])
	return {}

func untransformed_bounds(model: Node3D) -> AABB:
	var inverse := model.global_transform.affine_inverse()
	var result := AABB()
	var first := true
	for mesh in model.find_children("*", "MeshInstance3D", true, false):
		if not mesh.is_visible_in_tree():
			continue
		var next: AABB = (inverse * mesh.global_transform) * mesh.get_aabb()
		result = next if first else result.merge(next)
		first = false
	completion.require(not first, "Loaded Journal creature has no visible source geometry")
	return result

func verify_rendered_model(host: Control, model: Node3D) -> bool:
	var viewport := host.find_child("CharacterModelViewport", true, false) as SubViewport
	var view := host.find_child("CharacterModelView", true, false) as TextureRect
	if not completion.require(viewport != null and view != null, "Journal render host missing"):
		return false
	await RenderingServer.frame_post_draw
	var image := viewport.get_texture().get_image()
	var painted := 0
	for y in image.get_height():
		for x in image.get_width():
			var pixel := image.get_pixel(x, y)
			if pixel.a > 0.5 and maxf(pixel.r, maxf(pixel.g, pixel.b)) > 0.08:
				painted += 1
	var mesh_info := []
	for mesh in model.find_children("*", "MeshInstance3D", true, false):
		mesh_info.append({"name":str(mesh.get_path()),"visible":mesh.is_visible_in_tree(),"layers":mesh.layers,"position":str(mesh.global_position)})
	print("JOURNAL_RENDER_OBSERVED ", JSON.stringify({"viewport_size":str(viewport.size),"camera":str(viewport.get_camera_3d()),"view_rect":str(view.get_global_rect()),"view_visible":view.is_visible_in_tree(),"z_index":view.z_index,"painted":painted,"meshes":mesh_info}))
	if not completion.require(painted > 20, "Journal loaded creature does not paint its viewport"):
		return false
	var shown := root.get_texture().get_image()
	var display := int(model.get_meta("creature_display_id"))
	var prefix := shots.path_join("%s-render-display%s" % [str(client.account_state().ui_skin).to_lower(), display])
	if not completion.require(image.save_png(prefix + "-viewport.png") == OK and shown.save_png(prefix + "-shown.png") == OK, "Diagnostic rendered images save failed"):
		return false
	view.hide()
	await settle()
	await RenderingServer.frame_post_draw
	var hidden := root.get_texture().get_image()
	if not completion.require(hidden.save_png(prefix + "-hidden.png") == OK, "Diagnostic hidden image save failed"):
		return false
	view.show()
	await settle()
	var rect := view.get_global_rect()
	var changed := 0
	var changed_center := 0
	var center_region := Rect2(rect.position + rect.size * 0.375, rect.size * 0.25)
	for y in range(maxi(0, int(rect.position.y)), mini(shown.get_height(), int(rect.end.y))):
		for x in range(maxi(0, int(rect.position.x)), mini(shown.get_width(), int(rect.end.x))):
			var delta := shown.get_pixel(x, y) - hidden.get_pixel(x, y)
			if Vector3(delta.r, delta.g, delta.b).length() > 0.04:
				changed += 1
				if center_region.has_point(Vector2(x, y)):
					changed_center += 1
	print("JOURNAL_RENDER_COMPOSITE changed=%s center=%s" % [changed, changed_center])
	return completion.require(changed_center > 20, "Journal model centre is covered in the final framebuffer")

func verify_scene(creature_id: int, scene_id: int, actor_id: int, camera_id: int) -> bool:
	var creature := source_row("JournalEncounterCreature", creature_id)
	var actor := source_row("UiModelSceneActor", actor_id)
	var display := source_row("UiModelSceneActorDisplay", int(actor.get("UiModelSceneActorDisplayID", "0")))
	var camera_info := source_row("UiModelSceneCamera", camera_id)
	if completion.failed:
		return false
	if not completion.require(int(creature.UiModelSceneID) == scene_id and int(actor.UiModelSceneID) == scene_id and int(camera_info.UiModelSceneID) == scene_id, "Wrong authored scene joins"):
		return false
	var host := control("EncounterJournalModelScene")
	if not completion.require(host != null, "Journal model viewport missing"):
		return false
	var cameras := host.find_children("*", "Camera3D", true, false)
	var model := host.find_child("NpcVisualRoot", true, false) as Node3D
	if not completion.require(cameras.size() == 1 and model != null, "Loaded Journal camera/model missing or ambiguous"):
		return false
	var camera: Camera3D = cameras[0]
	# These two fixed source cameras have zero pitch/roll and no zoom spline.
	if not completion.require(float(camera_info.Pitch) == 0.0 and float(camera_info.Roll) == 0.0 and float(actor.OrientationPitch) == 0.0 and float(actor.OrientationRoll) == 0.0, "Source proof case orientation changed"):
		return false
	var yaw := float(camera_info.Yaw)
	var forward := Vector3(cos(yaw), 0, -sin(yaw))
	var target := Vector3(float(camera_info.Target_0), float(camera_info.Target_2), -float(camera_info.Target_1))
	var eye := target - forward * float(camera_info.ZoomDistance)
	var creature_display := source_row("CreatureDisplayInfo", int(creature.CreatureDisplayInfoID))
	var model_data := source_row("CreatureModelData", int(creature_display.get("ModelID", "0")))
	if completion.failed:
		return false
	var intrinsic := float(creature_display.CreatureModelScale) * float(model_data.ModelScale)
	var bounds: AABB = Transform3D(Basis.IDENTITY.scaled(Vector3.ONE * intrinsic), Vector3.ZERO) * untransformed_bounds(model)
	var size := bounds.size
	var maximum := maxf(size.x / 0.9984, maxf(size.z / 1.121, size.y / 2.118))
	var normalized := float(display.Scale) / lerpf(1.0, maximum, float(actor.NormalizedScale))
	var rotation := Basis(Vector3.UP, float(actor.OrientationYaw))
	var center := bounds.get_center()
	var flags := int(actor.Flags)
	var selected_center := Vector3(center.x if flags & 2 else 0.0, center.y if flags & 8 else 0.0, center.z if flags & 4 else 0.0)
	var position := Vector3(float(actor.Position_0), float(actor.Position_2), -float(actor.Position_1)) - rotation * selected_center * normalized
	var scale := Vector3.ONE * intrinsic * normalized
	print("JOURNAL_AUTHORED_SCENE_OBSERVED ", JSON.stringify({"creature":creature_id,"scene":scene_id,"camera":camera_id,"expected_eye":str(eye),"actual_eye":str(camera.global_position),"expected_origin":str(position),"actual_origin":str(model.global_position),"expected_scale":str(scale),"actual_scale":str(model.global_basis.get_scale()),"raw_bounds":str(bounds),"fov_retained_not_retail_certified":camera.fov}))
	if not completion.step(await move_to("EncounterJournalModelTitle"), "move away from model controls"):
		return false
	if not completion.step(await shot("model-scene%s-observed" % scene_id), "authored model scene capture"):
		return false
	if not completion.require(camera.global_position.distance_to(eye) < 0.002, "Journal wrong framing: native eye differs from raw authored source"):
		return false
	if not completion.require((-camera.global_basis.z).distance_to(forward) < 0.002, "Journal camera orientation differs from authored source"):
		return false
	if not completion.require(model.global_position.distance_to(position) < 0.003 and model.global_basis.get_scale().distance_to(scale) < 0.003, "Journal actor normalization/origin differs from authored source"):
		return false
	if not completion.require(model.global_basis.orthonormalized().is_equal_approx(rotation), "Journal actor orientation differs from authored source"):
		return false
	if not await verify_rendered_model(host, model):
		return false
	return completion.case_done()

func select_source_model(instance: int, encounter: int, creature: int, display: int, title: String) -> bool:
	if control("EncounterJournalBack") != null and client.adventure_guide_state().instance > 0:
		if not completion.step(await click("EncounterJournalBack"), "physical Journal Home"):
			return false
	if not completion.step(await open_instance("EncounterJournalDungeonTab", instance), "physical source dungeon selection"):
		return false
	if not completion.step(await click("JournalBoss%s" % encounter), "physical source encounter selection") or not completion.step(await click("EncounterJournalPage3"), "physical Model tab"):
		return false
	return completion.step(await expect_model(creature, display, title), "genuine loaded source creature")

func run_scenes() -> bool:
	var server := OS.get_environment("ADVENTURE_GUIDE_SERVER")
	var account := OS.get_environment("ADVENTURE_GUIDE_ACCOUNT")
	shots = OS.get_environment("ADVENTURE_GUIDE_SHOTS")
	if not completion.require(server == "127.0.0.1:5771" and account.begins_with("fb_") and not shots.is_empty(), "Explicit private5771/fb_* account and shots required"):
		return false
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var deadline := Time.get_ticks_msec() + 180000
	while client.account_state().assets_starting and Time.get_ticks_msec() < deadline:
		await process_frame
	if not completion.require(not client.account_state().assets_starting, "Asset startup timeout"):
		return false
	var error: String = client.connect_account(server, account, OS.get_environment("ADVENTURE_GUIDE_PASSWORD"), false)
	if not completion.require(error.is_empty(), error) or not completion.step(await enter_world_when_ready(), "genuine world readiness"):
		return false
	await journal_key()
	deadline = Time.get_ticks_msec() + 90000
	while not client.adventure_guide_state().loaded and Time.get_ticks_msec() < deadline:
		await process_frame
	if not completion.require(client.adventure_guide_state().loaded, "Journal source catalog not ready"):
		return false
	if not completion.require(client.account_state().ui_skin == OS.get_environment("ADVENTURE_GUIDE_EXPECTED_SKIN"), "Expected skin mismatch"):
		return false
	if not await select_source_model(67, 112, 382, 36475, "Ozruk") or not await verify_scene(382, 9, 10, 6):
		return false
	if not await select_source_model(70, 126, 426, 34580, "Anraphet") or not await verify_scene(426, 65, 88, 69):
		return false
	print("JOURNAL_AUTHORED_MODEL_SCENES_PASS skin=%s source9/source65 existingFOV_not_retail_certified" % client.account_state().ui_skin)
	return true
