extends SceneTree

func _initialize() -> void:
	if not ClassDB.class_exists("GameClient"):
		push_error("GameClient extension class was not registered")
		quit(1)
		return
	var client = ClassDB.instantiate("GameClient")
	if not client is Node3D:
		push_error("GameClient must be a native 3D scene root")
		quit(1)
		return
	root.add_child(client)
	client.queue_free()
	print("PASS: Rust client instantiated and attached to Godot scene tree")
	quit(0)
