extends SceneTree

# All three offline authored previews in one rendered process; no server.
var output := OS.get_environment("GODOT_CAPTURE_PATH").get_base_dir()

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	for screen in ["flight_map_preview", "registration_pending_preview", "aura_tooltip_preview"]:
		var ui := ClassDB.instantiate("RegistryUi") as Node
		root.add_child(ui)
		var error: String = ui.call("show_" + screen)
		if not error.is_empty():
			fail(ui, error)
			return
		for frame in range(6):
			await process_frame
			await RenderingServer.frame_post_draw
		var image := root.get_texture().get_image()
		if image == null or image.save_png(output.path_join(screen + ".png")) != OK:
			fail(ui, "Offline preview capture failed")
			return
		if not assert_screen(ui, image, screen):
			fail(ui, "Offline preview behavior failed: " + screen)
			return
		print("PASS capturepolish_rendered_", screen)
		ui.free()
		await process_frame
	quit(0)

func assert_screen(ui: Node, image: Image, screen: String) -> bool:
	if screen == "flight_map_preview":
		var portrait := ui.find_child("FlightMapPortrait", true, false) as Control
		var amount := ui.find_child("FlightMapTooltipMoneyAmount0", true, false) as Label
		var coin := ui.find_child("FlightMapTooltipMoneyCoin0", true, false) as Control
		var title := ui.find_child("FlightMapTooltipText", true, false) as Label
		if portrait == null or amount == null or coin == null or title == null:
			return false
		if title.text != "Sentinel Hill, Westfall" or amount.text != "5" or not coin.is_visible_in_tree():
			return false
		print("FLIGHT_NATIVE portrait=", portrait.get_global_rect(), " coin=", coin.get_global_rect(), " amount=", amount.text)
		return colored_pixels(image, portrait.get_global_rect()) > 100 and colored_pixels(image, coin.get_global_rect()) > 5
	if screen == "registration_pending_preview":
		var status := ui.find_child("LoginStatus", true, false) as Label
		if status == null or not status.text.begins_with("Registration submitted."):
			return false
		var color := status.get_theme_color("font_color")
		print("REGISTRATION_NATIVE status=", status.text, " color=", color)
		return color.is_equal_approx(Color(1.0, 0.82, 0.0, 1.0))
	var labels := ui.find_children("*", "Label", true, false)
	var text := ""
	for label in labels:
		text += label.text + "\n"
	print("AURA_NATIVE ", text)
	return text.contains("Arcane Intellect") and text.contains("Intellect increased by 3%.") and text.contains("60 minutes remaining") and not text.contains("Spell ID:")

func colored_pixels(image: Image, rect: Rect2) -> int:
	var count := 0
	for y in range(int(rect.position.y), int(rect.end.y)):
		for x in range(int(rect.position.x), int(rect.end.x)):
			var pixel := image.get_pixel(x, y)
			if maxf(pixel.r, maxf(pixel.g, pixel.b)) - minf(pixel.r, minf(pixel.g, pixel.b)) > 0.1:
				count += 1
	return count

func fail(ui: Node, message: String) -> void:
	push_error(message)
	ui.free()
	quit(1)
