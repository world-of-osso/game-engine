extends SceneTree

var completion = preload("res://tests/completion.gd").new(self, 6)
# Offline production screen/state projection, physical input; no server/terrain claim.
var output: String
var verify: bool
var actions: Array[String] = []

func _initialize() -> void:
	call_deferred("run_completion")

func settle() -> bool:
	for frame in range(6):
		await process_frame
		await RenderingServer.frame_post_draw

	return true

func sync_actions(ui: Node) -> bool:
	var action: String = ui.pop_action()
	while not action.is_empty():
		actions.append(action)
		action = ui.pop_action()

	return true

func move(ui: Node, point: Vector2) -> bool:
	var event := InputEventMouseMotion.new()
	event.position = point
	event.global_position = point
	Input.parse_input_event(event)
	await process_frame
	if not completion.step(sync_actions(ui), "sync_actions"):
		return false
	if not completion.step(await settle(), "settle"):
		return false

	return true

func button_event(ui: Node, point: Vector2, pressed: bool) -> bool:
	var event := InputEventMouseButton.new()
	event.position = point
	event.global_position = point
	event.button_index = MOUSE_BUTTON_LEFT
	event.pressed = pressed
	Input.parse_input_event(event)
	await process_frame
	if not completion.step(sync_actions(ui), "sync_actions"):
		return false
	if not completion.step(await settle(), "settle"):
		return false

	return true

func save_crop(owner: Control, label: String) -> bool:
	var image := root.get_texture().get_image()
	var rect := Rect2i(owner.get_global_rect().grow(8))
	if not completion.require(image.get_region(rect).save_png(output.path_join(label + ".png")) == OK):
		return false

	return true

func run_zoom() -> bool:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	output = OS.get_environment("MINIMAP_ZOOM_CAPTURE_PATH")
	verify = OS.get_environment("MINIMAP_ZOOM_VERIFY") == "1"
	if not completion.require(not output.is_empty()):
		return false
	for forever in [true]:
		for zoom in [0, 2, 5]:
			var ui = ClassDB.instantiate("RegistryUi")
			root.add_child(ui)
			if not completion.require(ui.show_minimap_zoom_preview(forever, zoom).is_empty()):
				return false
			if not completion.step(await settle(), "settle"):
				return false
			var skin := "forever" if forever else "modern"
			for name in ["MinimapZoomIn", "MinimapZoomOut"]:
				var owner := ui.find_child(name, true, false) as Control
				if not completion.require(owner != null):
					return false
				var disabled: bool = (zoom == 0 and name == "MinimapZoomOut") or (zoom == 5 and name == "MinimapZoomIn")
				if not completion.step(await inspect_states(ui, owner, "%s-z%d-%s" % [skin, zoom, name], disabled), "inspect_states"):
					return false
			if not completion.step(await move(ui, Vector2(200, 500)), "move"):
				return false
			if not completion.require(root.get_texture().get_image().save_png(output.path_join("%s-z%d-context.png" % [skin, zoom])) == OK):
				return false
			if not completion.case_done():
				return false
			ui.queue_free()
			await process_frame
	print("PASS minimap zoom both-skin supplied zoom0/2/5 idle/hover/press/outside-release/disabled input")
	return true

func inspect_states(ui: Node, owner: Control, prefix: String, disabled: bool) -> bool:
	var point := owner.get_global_rect().get_center()
	if not completion.step(await move(ui, Vector2(200, 500)), "move"):
		return false
	if not completion.step(save_crop(owner, prefix + "-idle"), "save_crop"):
		return false
	var idle := root.get_texture().get_image().get_region(Rect2i(owner.get_global_rect())).get_data()
	if not completion.step(await move(ui, point), "move"):
		return false
	if not completion.step(save_crop(owner, prefix + "-hover"), "save_crop"):
		return false
	var hover := root.get_texture().get_image().get_region(Rect2i(owner.get_global_rect())).get_data()
	if not completion.step(await button_event(ui, point, true), "button_event"):
		return false
	if not completion.step(save_crop(owner, prefix + "-pressed"), "save_crop"):
		return false
	var pressed := root.get_texture().get_image().get_region(Rect2i(owner.get_global_rect())).get_data()
	# Release outside must cancel, even after a real held press over the button.
	if not completion.step(await move(ui, Vector2(200, 500)), "move"):
		return false
	if not completion.step(await button_event(ui, Vector2(200, 500), false), "button_event"):
		return false
	if verify:
		if not completion.require(actions.is_empty(), "Outside release never activates"):
			return false
	actions.clear()
	if not completion.step(await move(ui, point), "move"):
		return false
	if not completion.step(await button_event(ui, point, true), "button_event"):
		return false
	if not completion.step(await button_event(ui, point, false), "button_event"):
		return false
	var action: String = "" if actions.is_empty() else actions[0]
	if not completion.require(actions.size() <= 1, "At most one click action"):
		return false
	actions.clear()
	if verify:
		if not completion.require(owner.modulate.a == 1.0, "Disabled alpha remains source full opacity"):
			return false
		var base := owner.get_node("Parts/Part0") as TextureRect
		if not completion.require(base != null and base.texture != null, "Source base art loaded"):
			return false
		if not completion.require((base.material is ShaderMaterial) == disabled, "Disabled sampled art is desaturated"):
			return false
		if not completion.require((idle == hover) == disabled, "Only enabled controls gain source hover art"):
			return false
		if not completion.require((idle == pressed) == disabled, "Only enabled controls gain source pushed art"):
			return false
		var expected := "minimap:zoom_in" if owner.name == "MinimapZoomIn" else "minimap:zoom_out"
		if not completion.require(action == ("" if disabled else expected), "Actual input must respect limit enablement"):
			return false
	print("ZOOM_STATE ", prefix, " disabled=", disabled, " idle_hover_equal=", idle == hover, " idle_pressed_equal=", idle == pressed, " action=", action, " rect=", owner.get_global_rect())
	return true

func run_completion() -> void:
	completion.finish(await run_zoom())
