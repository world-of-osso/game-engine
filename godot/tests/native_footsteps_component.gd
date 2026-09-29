extends SceneTree

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	var sound: Node = ClassDB.instantiate("NativeSound")
	root.add_child(sound)
	var empty_root := "user://native-footsteps-empty"
	DirAccess.make_dir_absolute(ProjectSettings.globalize_path(empty_root))
	var listfile := FileAccess.open(empty_root.path_join("community-listfile.csv"), FileAccess.WRITE)
	if listfile == null:
		push_error("Cannot write isolated empty-catalog fixture")
		quit(1)
		return
	listfile.store_string("1;world/maps/unrelated.adt\n")
	listfile.close()
	var empty_loaded: bool = sound.configure_footsteps(ProjectSettings.globalize_path(empty_root))
	DirAccess.remove_absolute(ProjectSettings.globalize_path(empty_root.path_join("community-listfile.csv")))
	DirAccess.remove_absolute(ProjectSettings.globalize_path(empty_root))
	if empty_loaded:
		push_error("A listfile with no local Oggs reported a loaded catalog")
		quit(1)
		return
	if not sound.configure(ProjectSettings.globalize_path("res://../data")):
		push_error("Real local sound catalogs failed to load")
		quit(1)
		return
	if not sound.configure_footsteps(ProjectSettings.globalize_path("res://../data")):
		push_error("Real local footstep listfile or Ogg catalog failed to load")
		quit(1)
		return
	var footsteps := sound.get_node_or_null("Footsteps")
	if footsteps == null:
		push_error("NativeSound has no separate owned Footsteps node")
		quit(1)
		return
	sound.queue_free()
	await process_frame
	if is_instance_valid(sound):
		push_error("Footstep node survived sound teardown")
		quit(1)
		return
	print("PASS: real local native footstep catalog and owned spatial channel lifecycle")
	quit(0)
