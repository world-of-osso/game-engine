extends SceneTree

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	var probe = ClassDB.instantiate("WowHousingPreviewProbe")
	var host := Control.new()
	host.size = Vector2(301, 334)
	root.add_child(host)
	# Authenticated HouseDecor525 -> ModelType2 / WMO6429362, not an M2 alias.
	var error: String = probe.select_decor(host, 525)
	if not error.is_empty():
		printerr("FAIL: housing WMO preview: ", error)
		quit(1)
		return
	await process_frame
	var actor: Node3D = host.find_child("HousingCatalogActor", true, false)
	if actor == null or int(actor.get_meta("model_file_data_id", 0)) != 6429362:
		printerr("FAIL: wrong WMO catalog identity")
		quit(1)
		return
	if actor.find_children("*", "MeshInstance3D", true, false).is_empty():
		printerr("FAIL: WMO preview lacks actual group geometry")
		quit(1)
		return
	probe.clear_preview()
	if host.get_child_count() != 0:
		printerr("FAIL: WMO survives preview close")
		quit(1)
		return
	host.free()
	print("PASS: authenticated housing WMO group preview and teardown")
	quit(0)
