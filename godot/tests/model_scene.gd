extends SceneTree

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var client = ClassDB.instantiate("GameClient")
	root.add_child(client)
	if not client.has_method("load_model_scene"):
		push_error("Native client cannot load a model scene")
		quit(1)
		return
	var loaded: Dictionary = client.call("load_model_scene", "res://../data/models/club_1h_torch_a_01.m2")
	if loaded.has("error"):
		push_error(str(loaded.error))
		quit(1)
		return
	var bounds: AABB = loaded.bounds
	var camera: Camera3D = root.get_camera_3d()
	if bounds.size.length() <= 0.0 or camera == null:
		push_error("Loaded geometry needs nonempty bounds and an active camera")
		quit(1)
		return
	if camera.is_position_behind(bounds.get_center()):
		push_error("Camera points away from the imported model")
		quit(1)
		return
	var before: int = client.get_child_count()
	var rejected: Dictionary = client.call("load_model_scene", "res://../data/models/does-not-exist.m2")
	if not rejected.has("error") or client.get_child_count() != before:
		push_error("Failed import must report its error without replacing the scene")
		quit(1)
		return
	client.queue_free()
	print("PASS: real M2 scene loads, frames geometry and preserves scene on import failure")
	quit(0)
