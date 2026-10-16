extends SceneTree
# Supplied queue proposal through production screen/input. No live matchmaking claim.
var output: String
var verify: bool
var actions: Array[String] = []

func _initialize() -> void:
	call_deferred("run_buttons")

func settle() -> void:
	for frame in range(6):
		await process_frame
		await RenderingServer.frame_post_draw

func drain(ui: Node) -> void:
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

func capture(owner: Control, name: String) -> PackedByteArray:
	var image := root.get_texture().get_image()
	var crop := image.get_region(Rect2i(owner.get_global_rect().grow(3)))
	assert(crop.save_png(output.path_join(name + ".png")) == OK)
	return crop.get_data()

func run_buttons() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	output = OS.get_environment("PET_READY_CAPTURE_PATH")
	verify = OS.get_environment("PET_READY_VERIFY") == "1"
	assert(not output.is_empty())
	for forever in [false, true]:
		for pending in [false, true]:
			var skin := "forever" if forever else "modern"
			var probe = ClassDB.instantiate("UiAuditProbe")
			root.add_child(probe)
			assert(probe.mount_collections(forever, "pet-queue-pending" if pending else "pet-queue").is_empty())
			await settle()
			var ui := probe.find_child("CollectionsAuditUI", true, false)
			assert(ui != null)
			for kind in ["Accept", "Decline"]:
				var owner := ui.find_child("PetBattleQueue" + kind + "Button", true, false) as Control
				assert(owner != null and owner.size == Vector2(120, 21))
				var label := "%s-%s-%s" % [skin, "pending" if pending else "ready", kind]
				var point := owner.get_global_rect().get_center()
				await move(ui, Vector2(200, 500))
				var idle := capture(owner, label + "-idle")
				await move(ui, point)
				var hover := capture(owner, label + "-hover")
				await press(ui, point, true)
				var held := capture(owner, label + "-pressed")
				await move(ui, Vector2(200, 500))
				await press(ui, Vector2(200, 500), false)
				assert(actions.is_empty(), "Outside release cancels")
				await move(ui, point)
				await press(ui, point, true)
				await press(ui, point, false)
				var expected := "pet:queue-accept" if kind == "Accept" else "pet:queue-decline"
				assert(actions == ([] if pending else [expected]), "Real ready action: " + str(actions))
				actions.clear()
				if verify:
					assert((idle == hover) == pending and (idle == held) == pending, "Enabled source state changes only")
					var parts := owner.get_node("Parts")
					assert(parts.get_child_count() == (4 if not pending else 3), "Three source sheet strips plus enabled hover")
					var left := parts.get_node("Part0") as TextureRect
					var middle := parts.get_node("Part1") as TextureRect
					var right := parts.get_node("Part2") as TextureRect
					assert(left.size.x == 12 and middle.size.x == 96 and right.size.x == 12, "Fixed12px source caps")
				print("PET_READY ", label, " input=", expected if not pending else "refused", " verified=", verify)
			await move(ui, Vector2(200, 500))
			assert(root.get_texture().get_image().save_png(output.path_join("%s-%s-context.png" % [skin, "pending" if pending else "ready"])) == OK)
			probe.queue_free()
			await process_frame
	print("PASS pet-ready both-skin physical states/inside-outside release/pending")
	quit(0)
