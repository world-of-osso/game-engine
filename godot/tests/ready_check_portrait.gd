extends SceneTree

var completion = preload("res://tests/completion.gd").new(self, 2)
# Post-gate real roster -> production selector/Portrait -> native pixels and cleanup.
var fixture: Node
# Production HUD UIParent: 768 UI units high (ui/ui_parent.rs), not preview scale1.
const HUD_SCALE := 1080.0 / 768.0

func _initialize() -> void:
	call_deferred("run_completion")

func settle() -> bool:
	for frame in range(8):
		await process_frame
		await RenderingServer.frame_post_draw

	return true

func rect(control: Control) -> Dictionary:
	return {"position": [control.global_position.x, control.global_position.y], "size": [control.size.x, control.size.y]}

func wait_for_head() -> Dictionary:
	for frame in range(1200):
		if not completion.require(fixture.tick() == ""):
			return {}
		await process_frame
		var state: Dictionary = fixture.ready_check_portrait_state()
		if state.get("model_shown", false) and state.get("mask_loaded", false) and not state.get("pending", true):
			if not completion.step(await settle(), "settle"):
				return {}
			return fixture.ready_check_portrait_state()
	if not completion.require(false, "Actual received initiator portrait did not load"):
		return {}
	return {}

func run_portrait() -> bool:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	var output := OS.get_environment("READY_PORTRAIT_CAPTURE_PATH")
	if not completion.require(not output.is_empty()):
		return false
	var observations: Array[Dictionary] = []
	for forever in [false, true]:
		var skin := "forever" if forever else "modern"
		fixture = ClassDB.instantiate("PartyPortraitFixture")
		root.add_child(fixture)
		if not completion.require(fixture.initialize(forever) == ""):
			return false
		var names: Array[String] = ["Ann", "Bob"]
		if not completion.require(fixture.set_members(names, true, true) == ""):
			return false
		if not completion.require(fixture.apply_ready_check(false) == ""):
			return false
		var state := await wait_for_head()
		if not completion.require(not state.is_empty(), "wait_for_head incomplete"):
			return false
		if not completion.require(state.appearance == "player Ann race 1 items 0", "Actual initiator, not the viewer Bob"):
			return false
		var panel := fixture.find_child("ReadyCheckFrame", true, false) as Control
		var host := fixture.find_child("ReadyCheckPortrait", true, false) as Control
		var title := fixture.find_child("ReadyCheckFrameTitle", true, false) as Label
		var message := fixture.find_child("ReadyCheckFrameText", true, false) as Label
		var ring := fixture.find_child("ReadyCheckFramePortraitRing", true, false) as Control
		if not completion.require(panel != null and host != null and title != null and message != null and ring != null):
			return false
		if not completion.require(panel.size == Vector2(323, 100) and host.size == Vector2(54, 54)):
			return false
		print("READY_PORTRAIT_METRICS ", skin, " scale=", HUD_SCALE, " viewport=", state.viewport_size, " host=", host.get_global_rect(), " panel=", panel.get_global_rect())
		var physical_size := Vector2(54, 54) * HUD_SCALE
		var physical_viewport := Vector2i(roundi(physical_size.x), roundi(physical_size.y))
		if not completion.require(state.viewport_size == physical_viewport, "Viewport follows 54 UI units at the production HUD pixel scale"):
			return false
		if not completion.require(host.get_global_rect().size.is_equal_approx(physical_size)):
			return false
		if not completion.require(((host.global_position - panel.global_position) / HUD_SCALE).is_equal_approx(Vector2(0, -3))):
			return false
		if not completion.require((state.rect as Rect2).is_equal_approx(host.get_global_rect())):
			return false
		if not completion.require(title.text == "Ready Check" and title.size == Vector2(241, 20)):
			return false
		if not completion.require(((title.global_position - panel.global_position) / HUD_SCALE).is_equal_approx(Vector2(58, 6))):
			return false
		if not completion.require(message.text == "Ann has initiated a ready check." and message.size.x == 240 and message.size.y > 0):
			return false
		var anchor := (message.global_position - panel.global_position) / HUD_SCALE
		if not completion.require(absf(anchor.x - 61.5) <= 0.5 and anchor.y == 37):
			return false
		for label in [title, message]:
			if not completion.require(label.get_theme_font_size("font_size") == 12):
				return false
			if not completion.require(label.get_theme_color("font_color").is_equal_approx(Color(1, 210.0 / 255.0, 0))):
				return false
			if not completion.require(label.get_theme_color("font_shadow_color") == Color.BLACK):
				return false
			if not completion.require(Vector2i(label.get_theme_constant("shadow_offset_x"), label.get_theme_constant("shadow_offset_y")) == Vector2i(1, 1)):
				return false
		var buttons: Array[Dictionary] = []
		for suffix in ["Yes", "No"]:
			var button := fixture.find_child("ReadyCheckFrame" + suffix + "Button", true, false) as Button
			if not completion.require(button != null and not button.disabled and button.size == Vector2(119, 24)):
				return false
			var caption := button.find_child("Text", true, false) as Label
			if not completion.require(caption != null and caption.text == ("Ready" if suffix == "Yes" else "Not Ready")):
				return false
			buttons.append({"name": suffix, "rect": rect(button), "caption": caption.text})
		var image := root.get_texture().get_image()
		if not completion.require(image.get_size() == Vector2i(1920, 1080)):
			return false
		if not completion.require(image.save_png(output.path_join(skin + "-context.png")) == OK):
			return false
		var crop_origin := panel.global_position - Vector2(16, 20) * HUD_SCALE
		var crop_size := Vector2(360, 140) * HUD_SCALE
		var crop := Rect2i(floori(crop_origin.x), floori(crop_origin.y), ceili(crop_size.x), ceili(crop_size.y))
		if not completion.require(image.get_region(crop).save_png(output.path_join(skin + "-panel-crop.png")) == OK):
			return false
		if not completion.require((state.image as Image).save_png(output.path_join(skin + "-head-render.png")) == OK):
			return false
		host.visible = false
		if not completion.step(await settle(), "settle"):
			return false
		if not completion.require(root.get_texture().get_image().save_png(output.path_join(skin + "-head-off-control.png")) == OK):
			return false
		host.visible = true
		if not completion.step(await settle(), "settle"):
			return false
		if not completion.require(root.get_texture().get_image().get_data() == image.get_data(), "Restore actual head RGBA exactly"):
			return false
		observations.append({"case": skin, "hud_scale": HUD_SCALE, "logical_portrait_size": [54, 54], "logical_portrait_origin": [0, -3], "panel": rect(panel), "portrait": rect(host), "ring": rect(ring), "title": rect(title), "message": rect(message), "buttons": buttons, "appearance": state.appearance, "viewport_size": [state.viewport_size.x, state.viewport_size.y], "mask_fdid": state.mask_fdid, "mask_loaded": state.mask_loaded, "pending": state.pending, "model_shown": state.model_shown})
		if not completion.require(fixture.apply_ready_check(true) == ""):
			return false
		if not completion.step(await settle(), "settle"):
			return false
		if not completion.require(fixture.find_child("ReadyCheckFrame", true, false) == null):
			return false
		if not completion.require(not fixture.ready_check_portrait_state().get("exists", true), "Completion releases actual portrait"):
			return false
		if not completion.require(root.find_children("*", "SubViewport", true, false).is_empty(), "No portrait viewport remains after completion"):
			return false
		fixture.free()
		if not completion.step(await settle(), "settle"):
			return false
		if not completion.require(root.find_children("*", "SubViewport", true, false).is_empty()):
			return false
		if not completion.case_done():
			return false
	var receipt := FileAccess.open(OS.get_environment("READY_PORTRAIT_RECEIPT"), FileAccess.WRITE)
	if not completion.require(receipt != null):
		return false
	receipt.store_string(JSON.stringify(observations, "\t"))
	receipt.close()
	print("PASS ready-portrait both-skin actual roster head and completion cleanup")
	return true

func run_completion() -> void:
	completion.finish(await run_portrait())
