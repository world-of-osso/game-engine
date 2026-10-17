extends SceneTree
# Post-gate real roster -> production selector/Portrait -> native pixels and cleanup.
var fixture: Node

func _initialize() -> void:
	call_deferred("run_portrait")

func settle() -> void:
	for frame in range(8):
		await process_frame
		await RenderingServer.frame_post_draw

func rect(control: Control) -> Dictionary:
	return {"position": [control.global_position.x, control.global_position.y], "size": [control.size.x, control.size.y]}

func wait_for_head() -> Dictionary:
	for frame in range(1200):
		assert(fixture.tick() == "")
		await process_frame
		var state: Dictionary = fixture.ready_check_portrait_state()
		if state.get("model_shown", false) and state.get("mask_loaded", false) and not state.get("pending", true):
			await settle()
			return fixture.ready_check_portrait_state()
	assert(false, "Actual received initiator portrait did not load")
	return {}

func run_portrait() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	var output := OS.get_environment("READY_PORTRAIT_CAPTURE_PATH")
	assert(not output.is_empty())
	var observations: Array[Dictionary] = []
	for forever in [false, true]:
		var skin := "forever" if forever else "modern"
		fixture = ClassDB.instantiate("PartyPortraitFixture")
		root.add_child(fixture)
		assert(fixture.initialize(forever) == "")
		var names: Array[String] = ["Ann", "Bob"]
		assert(fixture.set_members(names, true, true) == "")
		assert(fixture.apply_ready_check(false) == "")
		var state := await wait_for_head()
		assert(state.appearance == "player Ann race 1 items 0", "Actual initiator, not the viewer Bob")
		assert(state.viewport_size == Vector2i(54, 54))
		var panel := fixture.find_child("ReadyCheckFrame", true, false) as Control
		var host := fixture.find_child("ReadyCheckPortrait", true, false) as Control
		var title := fixture.find_child("ReadyCheckFrameTitle", true, false) as Label
		var message := fixture.find_child("ReadyCheckFrameText", true, false) as Label
		var ring := fixture.find_child("ReadyCheckFramePortraitRing", true, false) as Control
		assert(panel != null and host != null and title != null and message != null and ring != null)
		assert(panel.size == Vector2(323, 100) and host.size == Vector2(54, 54))
		assert(host.global_position - panel.global_position == Vector2(0, -3))
		assert((state.rect as Rect2).is_equal_approx(host.get_global_rect()))
		assert(title.text == "Ready Check" and title.size == Vector2(323, 20))
		assert(message.text == "Ann has initiated a ready check." and message.size == Vector2(240, 28))
		var anchor := message.global_position - panel.global_position
		assert(absf(anchor.x - 61.5) <= 0.5 and anchor.y == 37)
		for label in [title, message]:
			assert(label.get_theme_font_size("font_size") == 12)
			assert(label.get_theme_color("font_color").is_equal_approx(Color(1, 210.0 / 255.0, 0)))
			assert(label.get_theme_color("font_shadow_color") == Color.BLACK)
			assert(Vector2i(label.get_theme_constant("shadow_offset_x"), label.get_theme_constant("shadow_offset_y")) == Vector2i(1, 1))
		var buttons: Array[Dictionary] = []
		for suffix in ["Yes", "No"]:
			var button := fixture.find_child("ReadyCheckFrame" + suffix + "Button", true, false) as Button
			assert(button != null and not button.disabled and button.size == Vector2(119, 24))
			var caption := button.find_child("Text", true, false) as Label
			assert(caption != null and caption.text == ("Ready" if suffix == "Yes" else "Not Ready"))
			buttons.append({"name": suffix, "rect": rect(button), "caption": caption.text})
		var image := root.get_texture().get_image()
		assert(image.get_size() == Vector2i(1920, 1080))
		assert(image.save_png(output.path_join(skin + "-context.png")) == OK)
		var crop := Rect2i(int(panel.global_position.x) - 16, int(panel.global_position.y) - 20, 360, 140)
		assert(image.get_region(crop).save_png(output.path_join(skin + "-panel-crop.png")) == OK)
		assert((state.image as Image).save_png(output.path_join(skin + "-head-render.png")) == OK)
		host.visible = false
		await settle()
		assert(root.get_texture().get_image().save_png(output.path_join(skin + "-head-off-control.png")) == OK)
		host.visible = true
		await settle()
		assert(root.get_texture().get_image().get_data() == image.get_data(), "Restore actual head RGBA exactly")
		observations.append({"case": skin, "panel": rect(panel), "portrait": rect(host), "ring": rect(ring), "title": rect(title), "message": rect(message), "buttons": buttons, "appearance": state.appearance, "viewport_size": [state.viewport_size.x, state.viewport_size.y], "mask_fdid": state.mask_fdid, "mask_loaded": state.mask_loaded, "pending": state.pending, "model_shown": state.model_shown})
		assert(fixture.apply_ready_check(true) == "")
		await settle()
		assert(fixture.find_child("ReadyCheckFrame", true, false) == null)
		assert(not fixture.ready_check_portrait_state().get("exists", true), "Completion releases actual portrait")
		assert(root.find_children("*", "SubViewport", true, false).is_empty(), "No portrait viewport remains after completion")
		fixture.free()
		await settle()
		assert(root.find_children("*", "SubViewport", true, false).is_empty())
	var receipt := FileAccess.open(OS.get_environment("READY_PORTRAIT_RECEIPT"), FileAccess.WRITE)
	assert(receipt != null)
	receipt.store_string(JSON.stringify(observations, "\t"))
	receipt.close()
	print("PASS ready-portrait both-skin actual roster head and completion cleanup")
	quit(0)
