extends SceneTree

# Receipt-backed visuals, own-world rendering and teardown. No server mutations.
func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	if not ClassDB.class_exists("WowHousingPreviewProbe"):
		printerr("FAIL: housing has no catalog model preview lifecycle")
		quit(1)
		return
	var probe = ClassDB.instantiate("WowHousingPreviewProbe")
	var host := Control.new()
	host.size = Vector2(301, 334)
	root.add_child(host)
	for decor_id in [1693, 2330, 1693]:
		var error: String = probe.select_decor(host, decor_id)
		if not error.is_empty():
			printerr("FAIL: authentic decor preview: ", error)
			quit(1)
			return
		await process_frame
		var view: TextureRect = host.find_child("CharacterModelView", true, false)
		if view == null:
			printerr("FAIL: no rendered catalog model")
			quit(1)
			return
		var viewport: SubViewport = view.find_child("CharacterModelViewport", true, false)
		if viewport == null or not viewport.use_own_world_3d or not viewport.transparent_bg:
			printerr("FAIL: catalog camera shares gameplay world")
			quit(1)
			return
		var models := viewport.find_children("*", "MeshInstance3D", true, false)
		if models.is_empty():
			printerr("FAIL: preview is not authentic 3D geometry")
			quit(1)
			return
		var visual: Node3D = viewport.find_child("HousingCatalogActor", true, false)
		if visual == null or int(visual.get_meta("housing_decor_id", 0)) != decor_id:
			printerr("FAIL: preview retained another catalog entry")
			quit(1)
			return
		probe.rotate_preview(0.5)
		if not is_equal_approx(visual.get_parent().rotation.y, 0.5):
			printerr("FAIL: preview rotation did not move independent actor")
			quit(1)
			return
	# Unknown entries must remove old geometry instead of retaining/falling back.
	var error: String = probe.select_decor(host, 4294967295)
	if error.is_empty() or host.find_child("CharacterModelView", true, false) != null:
		printerr("FAIL: unknown decor silently retained the previous model")
		quit(1)
		return
	probe.clear_preview()
	if host.get_child_count() != 0:
		printerr("FAIL: preview nodes survive close")
		quit(1)
		return
	host.free()
	print("PASS: housing authentic model replacement, own world, rotation, failure and teardown")
	quit(0)
