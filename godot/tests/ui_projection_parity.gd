extends SceneTree
# Incremental UI projection: after each change (move, recolour, hide, text, add, remove,
# hover, patch, a dynamic texture's relayout and new pixels), every frame's node equals
# the node a fresh projection draws, and a dynamic texture redraws only for new pixels.
const STEPS := 12

func _initialize() -> void:
	call_deferred("watchdog")
	call_deferred("run_contract")

func watchdog() -> void:
	await create_timer(60.0).timeout
	fail("projection parity exceeded 60s")

func fail(reason: String) -> void:
	push_error("UI_PROJECTION_PARITY_FAIL " + reason)
	quit(1)

func run_contract() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1280, 720)
	var owner: Node = ClassDB.instantiate("RegistryUi")
	root.add_child(owner)
	var error: String = owner.show_projection_parity_fixture()
	if not error.is_empty():
		fail("fixture initialization: " + error)
		return
	for step in STEPS:
		await process_frame
		error = owner.projection_parity_step(step)
		if not error.is_empty():
			fail(error)
			return
	print("UI_PROJECTION_PARITY_PASS steps=%d" % STEPS)
	quit(0)
