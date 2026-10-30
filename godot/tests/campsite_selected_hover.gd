extends SceneTree

var completion = preload("res://tests/completion.gd").new(self, 2)
# Supplied selection snapshots, real source card projection/input; no account persistence claim.
var output: String
var verify: bool

func _initialize() -> void:
	call_deferred("run_completion")

func settle() -> bool:
	for frame in range(8):
		await process_frame
		await RenderingServer.frame_post_draw

	return true

func move(ui: Node, point: Vector2) -> bool:
	var event := InputEventMouseMotion.new()
	event.position = point
	event.global_position = point
	Input.parse_input_event(event)
	await process_frame
	if not completion.require(ui.pop_action().is_empty()):
		return false
	if not completion.step(await settle(), "settle"):
		return false

	return true

func click(ui: Node, owner: Control, id: int) -> bool:
	if not completion.step(await move(ui, owner.get_global_rect().get_center()), "move"):
		return false
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = owner.get_global_rect().get_center()
		event.global_position = event.position
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = pressed
		Input.parse_input_event(event)
		await process_frame
		if pressed:
			if not completion.require(ui.pop_action().is_empty(), "Selection on release only"):
				return false
	if not completion.require(ui.pop_action() == "select_campsite:%d" % id, "Existing Campsite action preserved"):
		return false
	# This offline screen has no account: apply the next supplied state after dispatch.
	if not completion.require(ui.select_campsite_preview(id).is_empty()):
		return false
	if not completion.step(await settle(), "settle"):
		return false

	return true

func capture_card(ui: Node, skin: String, step: int, selected: int, id: int) -> bool:
	var owner := ui.find_child("CampsiteScene_%d" % id, true, false) as Control
	if not completion.require(owner != null and owner.size == Vector2(215, 173)):
		return false
	if not completion.step(await move(ui, Vector2(200, 900)), "move"):
		return false
	var name := "%s-step%d-selected%d-card%d" % [skin, step, selected, id]
	if not completion.step(await save(owner, name + "-idle"), "save"):
		return false
	if not completion.step(await move(ui, owner.get_global_rect().get_center()), "move"):
		return false
	if not completion.step(await save(owner, name + "-hover"), "save"):
		return false
	var hover := ui.find_child("CampsiteHover_%d" % id, true, false) as Control
	var corner := ui.find_child("CampsiteHoverArt_%d00" % id, true, false) as Control
	if not completion.require(hover != null and hover.is_visible_in_tree()):
		return false
	if not completion.require(hover.size == Vector2(224, 183)):
		return false
	if not completion.require(hover.global_position == owner.global_position + Vector2(-4, -5)):
		return false
	var texture := corner.get_node("Parts/Part0") as TextureRect
	if not completion.require(texture != null and texture.texture != null):
		return false
	print("CAMP_HOVER ", name, " corner_size=", corner.size, " source=", texture.texture.region if texture.texture is AtlasTexture else texture.texture)
	if verify:
		var margin := 18 if selected == id else 17
		var source_y := 176 if selected == id else 65
		if not completion.require(corner.size == Vector2(margin, margin), "Source selected-hover18 vs ordinary-hover17 margins"):
			return false
		if not completion.require(texture.texture is AtlasTexture and texture.texture.region == Rect2(865, source_y, margin, margin), "Actual native source atlas member, not only changed geometry"):
			return false
	if not completion.step(await move(ui, Vector2(200, 900)), "move"):
		return false
	if not completion.require(not hover.is_visible_in_tree(), "Pointer leave hides hover art"):
		return false

	return true

func save(owner: Control, label: String) -> bool:
	var image := root.get_texture().get_image()
	if not completion.require(image.get_region(Rect2i(owner.get_global_rect().grow(10))).save_png(output.path_join(label + ".png")) == OK):
		return false

	return true

func run_cards() -> bool:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	output = OS.get_environment("CAMP_HOVER_CAPTURE_PATH")
	verify = OS.get_environment("CAMP_HOVER_VERIFY") == "1"
	if not completion.require(not output.is_empty()):
		return false
	for forever in [true]:
		var ui = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		if not completion.require(ui.show_glue_menus_preview("campsite", forever).is_empty()):
			return false
		if not completion.step(await settle(), "settle"):
			return false
		var skin := "forever" if forever else "modern"
		for step in range(3):
			var selected: int = [1, 2, 1][step]
			if step > 0:
				if not completion.step(await click(ui, ui.find_child("CampsiteScene_%d" % selected, true, false), selected), "click"):
					return false
			if not completion.step(await capture_card(ui, skin, step, selected, 1), "capture_card"):
				return false
			if not completion.step(await capture_card(ui, skin, step, selected, 2), "capture_card"):
				return false
			if not completion.require(root.get_texture().get_image().save_png(output.path_join("%s-step%d-selected%d-context.png" % [skin, step, selected])) == OK):
				return false
		if not completion.case_done():
			return false
		ui.queue_free()
		await process_frame
	print("PASS Campsite both-skin pointer/selection source-state fixture")
	return true

func run_completion() -> void:
	completion.finish(await run_cards())
