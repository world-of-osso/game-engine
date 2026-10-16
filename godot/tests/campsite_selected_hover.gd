extends SceneTree
# Supplied selection snapshots, real source card projection/input; no account persistence claim.
var output: String
var verify: bool

func _initialize() -> void:
	call_deferred("run_cards")

func settle() -> void:
	for frame in range(8):
		await process_frame
		await RenderingServer.frame_post_draw

func move(ui: Node, point: Vector2) -> void:
	var event := InputEventMouseMotion.new()
	event.position = point
	event.global_position = point
	Input.parse_input_event(event)
	await process_frame
	assert(ui.pop_action().is_empty())
	await settle()

func click(ui: Node, owner: Control, id: int) -> void:
	await move(ui, owner.get_global_rect().get_center())
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = owner.get_global_rect().get_center()
		event.global_position = event.position
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = pressed
		Input.parse_input_event(event)
		await process_frame
		if pressed:
			assert(ui.pop_action().is_empty(), "Selection on release only")
	assert(ui.pop_action() == "select_campsite:%d" % id, "Existing Campsite action preserved")
	# This offline screen has no account: apply the next supplied state after dispatch.
	assert(ui.select_campsite_preview(id).is_empty())
	await settle()

func capture_card(ui: Node, skin: String, step: int, selected: int, id: int) -> void:
	var owner := ui.find_child("CampsiteScene_%d" % id, true, false) as Control
	assert(owner != null and owner.size == Vector2(215, 173))
	await move(ui, Vector2(200, 900))
	var name := "%s-step%d-selected%d-card%d" % [skin, step, selected, id]
	await save(owner, name + "-idle")
	await move(ui, owner.get_global_rect().get_center())
	await save(owner, name + "-hover")
	var hover := ui.find_child("CampsiteHover_%d" % id, true, false) as Control
	var corner := ui.find_child("CampsiteHoverArt_%d00" % id, true, false) as Control
	assert(hover != null and hover.is_visible_in_tree())
	assert(hover.size == Vector2(224, 183))
	assert(hover.global_position == owner.global_position + Vector2(-4, -5))
	var texture := corner.get_node("Parts/Part0") as TextureRect
	assert(texture != null and texture.texture != null)
	print("CAMP_HOVER ", name, " corner_size=", corner.size, " source=", texture.texture.region if texture.texture is AtlasTexture else texture.texture)
	if verify:
		assert(corner.size == Vector2(18, 18) if selected == id else corner.size == Vector2(17, 17), "Source selected-hover18 vs ordinary-hover17 margins")
	await move(ui, Vector2(200, 900))
	assert(not hover.is_visible_in_tree(), "Pointer leave hides hover art")

func save(owner: Control, label: String) -> void:
	var image := root.get_texture().get_image()
	assert(image.get_region(Rect2i(owner.get_global_rect().grow(10))).save_png(output.path_join(label + ".png")) == OK)

func run_cards() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	output = OS.get_environment("CAMP_HOVER_CAPTURE_PATH")
	verify = OS.get_environment("CAMP_HOVER_VERIFY") == "1"
	assert(not output.is_empty())
	for forever in [false, true]:
		var ui = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		assert(ui.show_glue_menus_preview("campsite", forever).is_empty())
		await settle()
		var skin := "forever" if forever else "modern"
		for step in range(3):
			var selected: int = [1, 2, 1][step]
			if step > 0:
				await click(ui, ui.find_child("CampsiteScene_%d" % selected, true, false), selected)
			await capture_card(ui, skin, step, selected, 1)
			await capture_card(ui, skin, step, selected, 2)
			assert(root.get_texture().get_image().save_png(output.path_join("%s-step%d-selected%d-context.png" % [skin, step, selected])) == OK)
		ui.queue_free()
		await process_frame
	print("PASS Campsite both-skin pointer/selection source-state fixture")
	quit(0)
