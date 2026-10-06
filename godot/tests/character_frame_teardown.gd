extends SceneTree

# Actual Rust CharacterFrame/ModelPreview ownership, without assets or a server.
func _initialize() -> void:
	check.call_deferred()

func check() -> void:
	# A Rust panic aborts this GDScript call instead of returning false.
	create_timer(10.0).timeout.connect(func() -> void:
		push_error("Paperdoll teardown fixture timed out")
		quit(1)
	)
	var parent := Node3D.new()
	root.add_child(parent)
	var probe = ClassDB.instantiate("WowCharacterFrameLifecycleProbe")
	for cycle in range(3):
		var view: TextureRect = probe.attach_preview(parent)
		var host := view.get_parent()
		var ui := host.get_parent()
		var viewport := view.get_child(0)
		var scene := viewport.get_child(0)
		var camera := scene.get_child(0)
		if probe.reset_frame() != true:
			push_error("Cycle %d: paperdoll teardown panicked" % cycle)
			quit(1)
			return
		for node in [ui, host, view, viewport, scene, camera]:
			if is_instance_valid(node):
				push_error("Cycle %d: paperdoll descendant survived teardown" % cycle)
				quit(1)
				return
		if parent.get_child_count() != 0 or probe.reset_frame() != true:
			push_error("Cycle %d: repeated teardown failed or leaked children" % cycle)
			quit(1)
			return
		await process_frame
	parent.free()
	print("PASS: paperdoll UI and preview teardown, repeated reset, and reattachment across frames")
	quit(0)
