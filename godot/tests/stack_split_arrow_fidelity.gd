extends SceneTree
# Supplied state / production projection and pointer queue, not a quantity or server reducer.
var output: String
var verify: bool
var actions: Array[String] = []

func _initialize() -> void:
	call_deferred("run_arrows")

func settle() -> void:
	for frame in range(8):
		await process_frame
		await RenderingServer.frame_post_draw

func drain(ui: Node) -> void:
	for action in ui.stack_split_preview_actions():
		actions.append(action)

func move(ui: Node, point: Vector2) -> void:
	var event := InputEventMouseMotion.new()
	event.position = point
	event.global_position = point
	Input.parse_input_event(event)
	await process_frame
	drain(ui)
	await settle()

func press(ui: Node, point: Vector2, down: bool) -> void:
	var event := InputEventMouseButton.new()
	event.position = point
	event.global_position = point
	event.button_index = MOUSE_BUTTON_LEFT
	event.pressed = down
	Input.parse_input_event(event)
	await process_frame
	drain(ui)
	await settle()

func capture(owner: Control, label: String) -> Image:
	var image := root.get_texture().get_image()
	assert(image.save_png(output.path_join(label + "-context.png")) == OK)
	var crop := image.get_region(Rect2i(owner.get_global_rect()))
	assert(crop.save_png(output.path_join(label + "-crop.png")) == OK)
	return crop

func run_arrows() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	output = OS.get_environment("STACK_ARROW_CAPTURE_PATH")
	verify = OS.get_environment("STACK_ARROW_VERIFY") == "1"
	assert(not output.is_empty())
	for forever in [false, true]:
		var skin := "forever" if forever else "modern"
		for bundles in [false, true]:
			for enabled in [false, true]:
				for key in ["Left", "Right"]:
					var label: String = skin + ("-bundle-" if bundles else "-single-") + key + ("-enabled" if enabled else "-disabled")
					var ui = ClassDB.instantiate("RegistryUi")
					root.add_child(ui)
					assert(ui.show_stack_split_preview(forever, bundles, enabled).is_empty())
					await settle()
					var owner := ui.find_child("StackSplitFrame" + key + "Button", true, false) as Control
					assert(owner != null and owner.size == Vector2(16, 16))
					var point := owner.get_global_rect().get_center()
					var outside := Vector2(100, 100)
					await move(ui, outside)
					var idle := capture(owner, label + "-idle")
					await move(ui, point)
					var hover := capture(owner, label + "-hover")
					assert(idle.get_data() == hover.get_data(), "Source arrows have no hover overlay")
					await press(ui, point, true)
					var pushed := capture(owner, label + "-pressed")
					if verify and enabled:
						assert(idle.get_data() != pushed.get_data(), "Source down art must paint during physical press")
					elif not enabled:
						assert(idle.get_data() == pushed.get_data(), "Disabled art remains static")
					await move(ui, outside)
					await press(ui, outside, false)
					assert(actions.is_empty(), "Outside release cancels")
					await move(ui, point)
					await press(ui, point, true)
					await press(ui, point, false)
					var expected: Array[String] = ["stack_split:" + key.to_lower()] if enabled else []
					assert(actions == expected, "Exact arrow action or disabled refusal; no reducer call")
					actions.clear()
					print("STACK_ARROW ", label, " physical art/cancel/action PASS")
					ui.queue_free()
					await process_frame
	print("PASS stack-arrow both-skin source states; verified=", verify)
	quit(0)
