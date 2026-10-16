extends SceneTree
# Offline production screen/state projection, physical input; no server/terrain claim.
var output: String
var verify: bool
var actions: Array[String] = []

func _initialize() -> void:
	call_deferred("run_zoom")

func settle() -> void:
	for frame in range(6):
		await process_frame
		await RenderingServer.frame_post_draw

func sync_actions(ui: Node) -> void:
	var action: String = ui.pop_action()
	while not action.is_empty():
		actions.append(action)
		action = ui.pop_action()

func move(ui: Node, point: Vector2) -> void:
	var event := InputEventMouseMotion.new()
	event.position = point
	event.global_position = point
	Input.parse_input_event(event)
	await process_frame
	sync_actions(ui)
	await settle()

func button_event(ui: Node, point: Vector2, pressed: bool) -> void:
	var event := InputEventMouseButton.new()
	event.position = point
	event.global_position = point
	event.button_index = MOUSE_BUTTON_LEFT
	event.pressed = pressed
	Input.parse_input_event(event)
	await process_frame
	sync_actions(ui)
	await settle()

func save_crop(owner: Control, label: String) -> void:
	var image := root.get_texture().get_image()
	var rect := Rect2i(owner.get_global_rect().grow(8))
	assert(image.get_region(rect).save_png(output.path_join(label + ".png")) == OK)

func run_zoom() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	output = OS.get_environment("MINIMAP_ZOOM_CAPTURE_PATH")
	verify = OS.get_environment("MINIMAP_ZOOM_VERIFY") == "1"
	assert(not output.is_empty())
	for forever in [false, true]:
		for zoom in [0, 2, 5]:
			var ui = ClassDB.instantiate("RegistryUi")
			root.add_child(ui)
			assert(ui.show_minimap_zoom_preview(forever, zoom).is_empty())
			await settle()
			var skin := "forever" if forever else "modern"
			for name in ["MinimapZoomIn", "MinimapZoomOut"]:
				var owner := ui.find_child(name, true, false) as Control
				assert(owner != null)
				var disabled: bool = (zoom == 0 and name == "MinimapZoomOut") or (zoom == 5 and name == "MinimapZoomIn")
				await inspect_states(ui, owner, "%s-z%d-%s" % [skin, zoom, name], disabled)
			await move(ui, Vector2(200, 500))
			assert(root.get_texture().get_image().save_png(output.path_join("%s-z%d-context.png" % [skin, zoom])) == OK)
			ui.queue_free()
			await process_frame
	print("PASS minimap zoom both-skin supplied zoom0/2/5 idle/hover/press/outside-release/disabled input")
	quit(0)

func inspect_states(ui: Node, owner: Control, prefix: String, disabled: bool) -> void:
	var point := owner.get_global_rect().get_center()
	await move(ui, Vector2(200, 500))
	save_crop(owner, prefix + "-idle")
	var idle := root.get_texture().get_image().get_region(Rect2i(owner.get_global_rect())).get_data()
	await move(ui, point)
	save_crop(owner, prefix + "-hover")
	var hover := root.get_texture().get_image().get_region(Rect2i(owner.get_global_rect())).get_data()
	await button_event(ui, point, true)
	save_crop(owner, prefix + "-pressed")
	var pressed := root.get_texture().get_image().get_region(Rect2i(owner.get_global_rect())).get_data()
	# Release outside must cancel, even after a real held press over the button.
	await move(ui, Vector2(200, 500))
	await button_event(ui, Vector2(200, 500), false)
	if verify:
		assert(actions.is_empty(), "Outside release never activates")
	actions.clear()
	await move(ui, point)
	await button_event(ui, point, true)
	await button_event(ui, point, false)
	var action: String = "" if actions.is_empty() else actions[0]
	assert(actions.size() <= 1, "At most one click action")
	actions.clear()
	if verify:
		assert(owner.modulate.a == 1.0, "Disabled alpha remains source full opacity")
		var base := owner.get_node("Parts/Part0") as TextureRect
		assert(base != null and base.texture != null, "Source base art loaded")
		assert((base.material is ShaderMaterial) == disabled, "Disabled sampled art is desaturated")
		assert((idle == hover) == disabled, "Only enabled controls gain source hover art")
		assert((idle == pressed) == disabled, "Only enabled controls gain source pushed art")
		var expected := "minimap:zoom_in" if owner.name == "MinimapZoomIn" else "minimap:zoom_out"
		assert(action == ("" if disabled else expected), "Actual input must respect limit enablement")
	print("ZOOM_STATE ", prefix, " disabled=", disabled, " idle_hover_equal=", idle == hover, " idle_pressed_equal=", idle == pressed, " action=", action, " rect=", owner.get_global_rect())
