extends SceneTree

# The client added the way Godot adds the main scene: before the root viewport enters
# the tree, so the client becomes ready while the root is still setting up its children.
# Startup display options still attach the root viewport's anti-aliasing controller.
# Run: godot --path godot -s res://tests/startup_main_scene.gd -- --screen particledebug
const FRAMES := 5

func _initialize() -> void:
	root.add_child(load("res://scenes/client.tscn").instantiate())
	call_deferred("run_test")

func run_test() -> void:
	for frame in FRAMES:
		await process_frame
	if root.get_node_or_null("NativeTaa") == null:
		fail("Startup display options did not attach NativeTaa to the root viewport")
		return
	print("PASS: startup as main scene attaches the root viewport controllers")
	quit(0)

func fail(message: String) -> void:
	push_error(message)
	quit(1)
