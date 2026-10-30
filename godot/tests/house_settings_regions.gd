extends SceneTree
# Source decorative regions only; supplied offline draft, no fresh gameplay/input claim.
func _initialize() -> void:
	call_deferred("run_regions")

func run_regions() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	var output := OS.get_environment("HOUSE_REGIONS_CAPTURE_PATH")
	var verify := OS.get_environment("HOUSE_REGIONS_VERIFY") == "1"
	assert(not output.is_empty())
	for forever in [true]:
		var skin := "forever" if forever else "modern"
		var ui = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		assert(ui.show_house_access_preview(forever, false).is_empty())
		for frame in range(90):
			await process_frame
			await RenderingServer.frame_post_draw
		var panel := ui.find_child("HousingHouseSettingsFrame", true, false) as Control
		assert(panel != null and panel.size == Vector2(600, 550))
		if verify:
			for pair in [
				["HousingSettingsHeaderGradient", Rect2(100, 0, 400, 80)],
				["HousingSettingsPlantDecoLeft", Rect2(6, 7, 0, 0)],
				["HousingSettingsSpacer", Rect2(25, 174, 550, 1)]]:
				var region := ui.find_child(pair[0], true, false) as Control
				assert(region != null)
				assert(region.global_position.is_equal_approx(panel.global_position + pair[1].position))
				if pair[0] != "HousingSettingsPlantDecoLeft":
					assert(region.size == pair[1].size)
				var texture := region.get_node("Parts/Part0") as TextureRect
				assert(texture != null and texture.texture != null)
				print("HOUSE_REGION ", skin, " ", pair[0], " rect=", region.get_global_rect())
		var image := root.get_texture().get_image()
		assert(image.save_png(output.path_join(skin + "-context.png")) == OK)
		assert(image.get_region(Rect2i(panel.get_global_rect())).save_png(output.path_join(skin + "-panel.png")) == OK)
		ui.queue_free()
		await process_frame
	print("PASS house-settings source regions both-skin; verified=", verify)
	quit(0)
